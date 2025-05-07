mod config;

use std::sync::Arc;

use ::tracing::level_filters::LevelFilter;
use anyhow::{Result, anyhow};
use chromiumoxide::{Browser, BrowserConfig, Page};
use config::CONFIG;
use database::{deadpool_postgres, queries, tokio_postgres::NoTls};
use futures::StreamExt;
use lapin::{
    ConnectionProperties,
    message::Delivery,
    options::{BasicAckOptions, BasicConsumeOptions},
    types::FieldTable,
};
use tokio::task::JoinSet;
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

#[tracing::instrument(err)]
pub async fn create_browser() -> Result<Browser> {
    let browser_config = BrowserConfig::builder()
        .with_head()
        .build()
        .map_err(|err| anyhow!(err))?;

    let (browser, mut handler) = Browser::launch(browser_config).await?;

    tokio::spawn(async move {
        loop {
            _ = handler.next().await;
        }
    });

    Ok(browser)
}

#[tracing::instrument(err)]
async fn process(
    delivery: Delivery,
    database_client: &deadpool_postgres::Client,
    s3_client: &aws_sdk_s3::Client,
    page: &Page,
) -> Result<()> {
    let id = Uuid::from_slice(&delivery.data)?;
    let submission = queries::submission::get()
        .bind(database_client, &id)
        .one()
        .await?;
    let question = queries::question::get()
        .bind(database_client, &submission.question_id)
        .one()
        .await?;
    let test_case = queries::test_case::get_by_question_id()
        .bind(database_client, &submission.question_id)
        .one()
        .await?;

    let expected_image = s3_wrapper::download(
        &test_case.output_path,
        &CONFIG.s3_bucket,
        &CONFIG.s3_dir.join(&test_case.output_path),
        CONFIG.s3_max_retry_count,
        s3_client,
    )
    .await?;

    match html_image_comparer::diff(
        &submission.code,
        &expected_image,
        CONFIG.width,
        CONFIG.height,
        page,
    )
    .await
    {
        Ok((match_ratio, _)) => {
            queries::submission::update_status()
                .bind(
                    database_client,
                    &(match_ratio as f32 * question.score),
                    &None::<&str>,
                    &None,
                    &id,
                )
                .await?;
        }
        Err(_) => {
            queries::submission::update_status()
                .bind(
                    database_client,
                    &0.,
                    &Some("Failed to run the html"),
                    &Some(1),
                    &id,
                )
                .await?;
        }
    };

    Ok(())
}

#[tokio::main]
async fn main() -> Result<()> {
    init_tracing();

    let amqp_channel = create_amqp_channel().await?;
    let database = connect_database()?;
    let s3_client = connect_s3().await?;
    let browser = create_browser().await?;

    let mut join_set = JoinSet::new();

    for _ in 0..CONFIG.thread_count {
        let mut consumer = amqp_channel
            .basic_consume(
                &CONFIG.html_queue_name,
                &CONFIG.id,
                BasicConsumeOptions::default(),
                FieldTable::default(),
            )
            .await?;
        let database = database.clone();
        let s3_client = s3_client.clone();
        let browser = browser.new_page("about:blank").await?;

        join_set.spawn(async move {
            while let Some(delivery) = consumer.next().await {
                let delivery = delivery?;
                delivery.ack(BasicAckOptions::default()).await?;

                let _ = process(delivery, &database.get().await?, &s3_client, &browser).await;
            }

            Ok::<_, anyhow::Error>(())
        });
    }

    while let Some(res) = join_set.join_next().await {
        let _ = res?;
    }

    Ok(())
}
