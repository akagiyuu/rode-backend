mod browser;
mod config;

use std::sync::Arc;

use anyhow::Result;
use chromiumoxide::{Browser, Page};
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
use uuid::Uuid;

async fn process(
    delivery: Delivery,
    database_client: &deadpool_postgres::Client,
    s3_client: &aws_sdk_s3::Client,
    tab: &Page,
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

    let page = browser.new_page("about:blank").await?;
    match html_image_comparer::diff(
        &submission.code,
        &expected_image,
        CONFIG.width,
        CONFIG.height,
        &page,
    )
    .await
    {
        Ok((match_ratio, _)) => {
            queries::submission::update_status()
                .bind(
                    database_client,
                    &(match_ratio as f32 * question.score),
                    &None::<String>,
                    &-1,
                    &id,
                )
                .await?;
        }
        Err(error) => {
            eprintln!("{:?}", error);

            queries::submission::update_status()
                .bind(
                    database_client,
                    &0.,
                    &Some("Failed to run the html".to_string()),
                    &1,
                    &id,
                )
                .await?;
        }
    };

    Ok(())
}

#[tokio::main]
async fn main() -> Result<()> {
    let mut database_config = deadpool_postgres::Config::new();
    database_config.url = Some(CONFIG.database_url.clone());
    let database = database_config.create_pool(Some(deadpool_postgres::Runtime::Tokio1), NoTls)?;
    let s3_client = Arc::new(aws_sdk_s3::Client::new(&aws_config::load_from_env().await));
    let browser = browser::spawn().await?;

    let amqp_connection =
        lapin::Connection::connect(&CONFIG.amqp_url, ConnectionProperties::default()).await?;
    let channel = amqp_connection.create_channel().await?;

    let mut join_set = JoinSet::new();

    for _ in 0..CONFIG.thread_count {
        let mut consumer = channel
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

                if let Err(error) =
                    process(delivery, &database.get().await?, &s3_client, &browser).await
                {
                    eprintln!("{:?}", error);
                }
            }

            Ok::<_, anyhow::Error>(())
        });
    }

    while let Some(res) = join_set.join_next().await {
        let res = res?;
        if let Err(error) = res {
            eprintln!("{:?}", error);
        }
    }

    Ok(())
}
