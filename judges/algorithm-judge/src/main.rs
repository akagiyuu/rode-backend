mod config;

use std::{
    path::{Path, PathBuf},
    sync::Arc,
    time::Duration,
};

use anyhow::{Context, Result, bail};
use bstr::ByteSlice;
use code_executor::{CPP, ExitStatus, JAVA, Language, Metrics, PYTHON, Runner};
use config::CONFIG;
use database::{client::Params, deadpool_postgres, queries, tokio_postgres::NoTls};
use futures::StreamExt;
use lapin::{
    ConnectionProperties,
    message::Delivery,
    options::{BasicAckOptions, BasicConsumeOptions},
    types::FieldTable,
};
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
pub async fn connect_s3() -> Result<Arc<s3_wrapper::Client>> {
    let config = aws_config::load_from_env().await;
    let client = aws_sdk_s3::Client::new(&config);
    let client = s3_wrapper::Client::new(client, CONFIG.s3_bucket.clone()).await?;

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
async fn compile(
    submission_id: Uuid,
    language: Language<'_>,
    code: &[u8],
    database_client: &deadpool_postgres::Client,
) -> Result<PathBuf> {
    match language.compiler.compile(code).await {
        Ok(project_path) => Ok(project_path),
        Err(error) => {
            queries::submission::update_status()
                .params(
                    database_client,
                    &queries::submission::UpdateStatusParams {
                        score: 0.,
                        error: Some("Compilation error"),
                        failed_test_case: None,
                        id: submission_id,
                    },
                )
                .await?;

            bail!(error)
        }
    }
}

async fn init_runner<'a>(
    question: &queries::question::Get,
    language: Language<'a>,
    project_path: &'a Path,
) -> Result<Runner<'a>> {
    const PROCESS_COUNT_LIMIT: usize = 512;

    let time_limit = question
        .time_limit
        .context("Time limit must be specified")?;
    let memory_limit = question
        .memory_limit
        .context("Memory limit must be specified")? as i64;

    let runner = Runner::new(
        language.runner_args,
        project_path,
        Duration::from_millis(time_limit as u64),
        memory_limit,
        PROCESS_COUNT_LIMIT,
    )?;

    Ok(runner)
}

#[repr(i32)]
enum Verdict {
    Timeout = 0,
    RuntimeError = 1,
    WrongAnswer = 2,
    Accepted = 3,
}

impl Verdict {
    fn as_error(&self, test_case_index: i32) -> String {
        match self {
            Self::Timeout => format!("Time limit exceeded on test {}", test_case_index),
            Self::RuntimeError => format!("Runtime error on test {}", test_case_index),
            Self::WrongAnswer => format!("Wrong answer on test {}", test_case_index),
            _ => unreachable!(),
        }
    }
}

#[tracing::instrument(err)]
async fn run(
    submission_id: Uuid,
    test_case: &queries::test_case::GetByQuestionId,
    project_path: &Path,
    runner: &Runner<'_>,
    database_client: &deadpool_postgres::Client,
    s3_client: &s3_wrapper::Client,
) -> Result<(Verdict, Metrics)> {
    let input_path = test_case
        .input_path
        .clone()
        .context("Input must be specified")?;

    let input = s3_client.get(input_path).await?;

    let metrics = runner.run(&input).await?;

    match metrics.exit_status {
        ExitStatus::Success => {
            let expected_output = s3_client.get(test_case.output_path.clone()).await?;
        }
        ExitStatus::RuntimeError => todo!(),
        ExitStatus::Timeout => todo!(),
    };
}

#[tracing::instrument(err)]
async fn process(
    delivery: Delivery,
    database_client: &deadpool_postgres::Client,
    s3_client: &s3_wrapper::Client,
) -> Result<()> {
    let id = Uuid::from_slice(&delivery.data)?;
    let submission = queries::submission::get()
        .bind(database_client, &id)
        .one()
        .await?;
    let language = get_language(submission.language)?;

    let (project_path, question) = tokio::try_join!(
        compile(id, language, submission.code.as_bytes(), database_client),
        async {
            queries::question::get()
                .bind(database_client, &submission.question_id)
                .one()
                .await
                .map_err(anyhow::Error::from)
        }
    )?;

    let (runner, test_cases) =
        tokio::try_join!(init_runner(&question, language, &project_path), async {
            queries::test_case::get_by_question_id()
                .bind(database_client, &submission.question_id)
                .iter()
                .await
                .map_err(anyhow::Error::from)
        })?;
    tokio::pin!(test_cases);

    while let Some(test_case) = test_cases.next().await {
        let test_case = test_case?;

        let test_case_result = run(
            id,
            &test_case,
            &project_path,
            &runner,
            database_client,
            s3_client,
        )
        .await?;
        if test_case.is_hidden {
            return Ok(());
        }

        let params = match self {
            TestCaseResult::Timeout => queries::submission_detail::InsertParams {
                submission_id,
                index: test_case.index,
                status: self.as_database_enum(),
                run_time: time_limit.as_millis() as i32,
                stdout: "",
                stderr: "",
            },
            TestCaseResult::RuntimeError => queries::submission_detail::InsertParams {
                submission_id,
                index: test_case.index,
                status: self.as_database_enum(),
                run_time: 0,
                stdout: "",
                stderr: "",
            },
            TestCaseResult::WrongAnswer(metrics) | TestCaseResult::Accepted(metrics) => {
                queries::submission_detail::InsertParams {
                    submission_id,
                    index: test_case.index,
                    status: self.as_database_enum(),
                    run_time: metrics.run_time.as_millis() as i32,
                    stdout: metrics.stdout.to_str()?,
                    stderr: metrics.stderr.to_str()?,
                }
            }
        };

        queries::submission_detail::insert()
            .params(database_client, &params)
            .await?;
        if matches!(test_case_result, TestCaseResult::Accepted(_)) {
            continue;
        }

        queries::submission::update_status()
            .params(
                database_client,
                &queries::submission::UpdateStatusParams {
                    id,
                    score: 0.,
                    error: Some(test_case_result.as_error(test_case.index)),
                    failed_test_case: Some(test_case.index),
                },
            )
            .await?;

        return Ok(());
    }

    queries::submission::update_status()
        .params(
            database_client,
            &queries::submission::UpdateStatusParams::<&str> {
                id,
                score: question.score,
                error: None,
                failed_test_case: None,
            },
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
