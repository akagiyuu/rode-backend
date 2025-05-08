mod config;

use std::sync::Arc;

use anyhow::{Result, anyhow};
use chromiumoxide::{Browser, BrowserConfig, Page};
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
    s3_client: &s3_wrapper::Client,
    page: &Page,
) -> Result<()> {
    let id = Uuid::from_slice(&delivery.data)?;
    let submission = queries::submission::get()
        .bind(database_client, &id)
        .one()
        .await?;
    let (question, test_case) = tokio::try_join!(
        async {
            queries::question::get()
                .bind(database_client, &submission.question_id)
                .one()
                .await
        },
        async {
            queries::test_case::get_by_question_id()
                .bind(database_client, &submission.question_id)
                .one()
                .await
        }
    )?;

    let expected_image = s3_client.get(test_case.output_path.clone()).await?;

    let update_status_params = match html_image_comparer::diff(
        &submission.code,
        &expected_image,
        CONFIG.width,
        CONFIG.height,
        page,
    )
    .await
    {
        Ok((match_ratio, _)) => queries::submission::UpdateStatusParams {
            id,
            score: match_ratio as f32 * question.score,
            error: None,
            failed_test_case: None,
        },
        Err(_) => queries::submission::UpdateStatusParams {
            id,
            score: 0.,
            error: Some("Failed to run the html"),
            failed_test_case: Some(1),
        },
    };
    queries::submission::update_status()
        .params(database_client, &update_status_params)
        .await?;

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
        let page = browser.new_page("about:blank").await?;

        join_set.spawn(async move {
            while let Some(delivery) = consumer.next().await {
                let delivery = delivery?;
                delivery.ack(BasicAckOptions::default()).await?;

                let _ = process(delivery, &database.get().await?, &s3_client, &page).await;
            }

            Ok::<_, anyhow::Error>(())
        });
    }

    while let Some(res) = join_set.join_next().await {
        let _ = res?;
    }

    Ok(())
}
