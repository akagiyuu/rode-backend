mod config;

use std::{sync::Arc, time::Duration};

use anyhow::{Context, Result, bail};
use bstr::ByteSlice;
use code_executor::{CPP, JAVA, Language, PYTHON, Runner};
use config::CONFIG;
use database::{deadpool_postgres, queries, tokio_postgres::NoTls};
use futures::StreamExt;
use lapin::{
    ConnectionProperties,
    message::Delivery,
    options::{BasicAckOptions, BasicConsumeOptions},
    types::FieldTable,
};
use s3_wrapper::download;
use tokio::task::JoinSet;
use tracing::level_filters::LevelFilter;
use tracing_subscriber::{
    EnvFilter, Layer, filter, fmt, layer::SubscriberExt, util::SubscriberInitExt,
};
use uuid::Uuid;

pub fn init_tracing() {
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::fmt::layer()
                .pretty()
                .with_timer(fmt::time::ChronoLocal::rfc_3339())
                .with_filter(filter::filter_fn(|metadata| {
                    !metadata.target().contains("chromiumoxide")
                })),
        )
        .with(
            EnvFilter::builder()
                .with_default_directive(LevelFilter::INFO.into())
                .from_env_lossy(),
        )
        .init();
}

#[tracing::instrument(err)]
pub async fn create_amqp_channel() -> Result<lapin::Channel> {
    let amqp_connection =
        lapin::Connection::connect(&CONFIG.amqp_url, ConnectionProperties::default()).await?;
    let channel = amqp_connection.create_channel().await?;

    Ok(channel)
}

#[tracing::instrument(err)]
pub fn connect_database() -> Result<deadpool_postgres::Pool> {
    let mut database_config = deadpool_postgres::Config::new();
    database_config.url = Some(CONFIG.database_url.clone());
    let database = database_config.create_pool(Some(deadpool_postgres::Runtime::Tokio1), NoTls)?;

    return Ok(database);
}

#[tracing::instrument(err)]
pub async fn connect_s3() -> Result<Arc<aws_sdk_s3::Client>> {
    let config = aws_config::load_from_env().await;
    let client = aws_sdk_s3::Client::new(&config);

    Ok(Arc::new(client))
}

fn get_language(language_raw: i16) -> Result<Language<'static>> {
    match language_raw {
        0 => Ok(CPP),
        1 => Ok(JAVA),
        2 => Ok(PYTHON),
        _ => bail!("Invalid language"),
    }
}

#[tracing::instrument(err)]
async fn process(
    delivery: Delivery,
    database_client: &deadpool_postgres::Client,
    s3_client: &aws_sdk_s3::Client,
) -> Result<()> {
    let id = Uuid::from_slice(&delivery.data)?;
    let submission = queries::submission::get()
        .bind(database_client, &id)
        .one()
        .await?;
    let language = get_language(submission.language)?;
    let project_path = match language.compiler.compile(submission.code.as_bytes()).await {
        Ok(project_path) => project_path,
        Err(error) => {
            queries::submission::update_status()
                .bind(database_client, &0., &Some("Compilation error"), &None, &id)
                .await?;

            bail!(error)
        }
    };

    let question = queries::question::get()
        .bind(database_client, &submission.question_id)
        .one()
        .await?;
    let time_limit = question
        .time_limit
        .context("Time limit must be specified")?;
    let memory_limit = question
        .memory_limit
        .context("Memory limit must be specified")? as i64;

    let runner = Runner::new(
        language.runner_args,
        &project_path,
        Duration::from_millis(time_limit as u64),
        memory_limit,
    )?;

    let test_cases = queries::test_case::get_by_question_id()
        .bind(database_client, &submission.question_id)
        .iter()
        .await?;
    tokio::pin!(test_cases);

    while let Some(test_case) = test_cases.next().await {
        let test_case = test_case?;
        let input_path = test_case.input_path.context("Input must be specified")?;

        let input = download(
            &CONFIG.s3_bucket,
            &input_path,
            &CONFIG.s3_dir.join(&input_path),
            CONFIG.s3_max_retry_count,
            s3_client,
        )
        .await?;

        let metrics = match runner.run(&input).await {
            Ok(metrics) => metrics,
            Err(code_executor::Error::Timeout) => {
                queries::submission::update_status()
                    .bind(
                        database_client,
                        &0.,
                        &Some(format!("Time limit exceeded on test {}", test_case.index)),
                        &Some(test_case.index),
                        &id,
                    )
                    .await?;
                if !test_case.is_hidden {
                    queries::submission_detail::insert()
                        .bind(
                            database_client,
                            &id,
                            &test_case.index,
                            &0,
                            &time_limit,
                            &"",
                            &"",
                        )
                        .await?;
                }

                return Ok(());
            }
            Err(code_executor::Error::Runtime { message }) => {
                tracing::info!("Runtime error on test {}: {}", test_case.index, message);

                queries::submission::update_status()
                    .bind(
                        database_client,
                        &0.,
                        &Some(format!("Runtime error on test {}", test_case.index)),
                        &Some(test_case.index),
                        &id,
                    )
                    .await?;

                if !test_case.is_hidden {
                    queries::submission_detail::insert()
                        .bind(database_client, &id, &test_case.index, &1, &0, &"", &"")
                        .await?;
                }

                return Ok(());
            }
            Err(error) => bail!(error),
        };

        let expected_output = download(
            &CONFIG.s3_bucket,
            &test_case.output_path,
            &CONFIG.s3_dir.join(&input_path),
            CONFIG.s3_max_retry_count,
            s3_client,
        )
        .await?;

        if metrics.stdout.trim() != expected_output.trim() {
            queries::submission::update_status()
                .bind(
                    database_client,
                    &0.,
                    &Some(format!("Wrong answer on test {}", test_case.index)),
                    &Some(test_case.index),
                    &id,
                )
                .await?;

            if !test_case.is_hidden {
                queries::submission_detail::insert()
                    .bind(
                        database_client,
                        &id,
                        &test_case.index,
                        &2,
                        &(metrics.run_time.as_millis() as i32),
                        &metrics.stdout.to_str()?,
                        &metrics.stderr.to_str()?,
                    )
                    .await?;
            }

            return Ok(());
        }

        if !test_case.is_hidden {
            queries::submission_detail::insert()
                .bind(
                    database_client,
                    &id,
                    &test_case.index,
                    &3,
                    &(metrics.run_time.as_millis() as i32),
                    &metrics.stdout.to_str()?,
                    &metrics.stderr.to_str()?,
                )
                .await?;
        }
    }

    queries::submission::update_status()
        .bind(
            database_client,
            &question.score,
            &Some("Accepted"),
            &None,
            &id,
        )
        .await?;

    Ok(())
}

#[tokio::main]
async fn main() -> Result<()> {
    init_tracing();

    let amqp_channel = create_amqp_channel().await?;
    let database = connect_database()?;
    let s3_client = connect_s3().await?;

    let mut join_set = JoinSet::new();

    for _ in 0..CONFIG.thread_count {
        let mut consumer = amqp_channel
            .basic_consume(
                &CONFIG.algorithm_queue_name,
                &CONFIG.id,
                BasicConsumeOptions::default(),
                FieldTable::default(),
            )
            .await?;
        let database = database.clone();
        let s3_client = s3_client.clone();

        join_set.spawn(async move {
            while let Some(delivery) = consumer.next().await {
                let delivery = delivery?;
                delivery.ack(BasicAckOptions::default()).await?;

                let _ = process(delivery, &database.get().await?, &s3_client).await;
            }

            Ok::<_, anyhow::Error>(())
        });
    }

    while let Some(res) = join_set.join_next().await {
        let _ = res?;
    }

    Ok(())
}
