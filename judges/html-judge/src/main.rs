mod browser;
mod config;

use std::ops::DerefMut;

use anyhow::{Context, Result};
use chromiumoxide::Browser;
use config::CONFIG;
use database::{Question, SubmissionHistory, TestCase, UpdateSubmissionHistory};
use diesel_async::{
    AsyncPgConnection,
    pooled_connection::{AsyncDieselConnectionManager, deadpool::Pool},
};
use futures::StreamExt;
use lapin::{
    ConnectionProperties,
    message::Delivery,
    options::{BasicAckOptions, BasicConsumeOptions},
    types::FieldTable,
};
use uuid::Uuid;

async fn process(
    delivery: Delivery,
    s3_client: &aws_sdk_s3::Client,
    database: Pool<AsyncPgConnection>,
    browser: &Browser,
) -> Result<()> {
    let id = Uuid::from_slice(&delivery.data)?;

    let page = browser.new_page("about:blank").await?;
    let submission = SubmissionHistory::get(id, database.get().await?.deref_mut()).await?;

    let (question, test_cases) = tokio::try_join!(
        async {
            let mut connection = database.get().await?;
            let question = Question::get(submission.question_id, &mut connection).await?;

            Ok::<_, anyhow::Error>(question)
        },
        async {
            let mut connection = database.get().await?;
            let test_cases =
                TestCase::get_by_question_id(submission.question_id, &mut connection).await?;

            Ok::<_, anyhow::Error>(test_cases)
        },
    )?;
    let test_case = test_cases.first().context("No test case defined")?;
    let expected_image = s3_wrapper::download(
        &test_case.output_path,
        &CONFIG.s3_bucket,
        &CONFIG.s3_dir.join(&test_case.output_path),
        CONFIG.s3_max_retry_count,
        s3_client,
    )
    .await?;

    let update_data = match html_image_comparer::diff(
        &submission.code,
        &expected_image,
        CONFIG.width,
        CONFIG.height,
        &page,
    )
    .await
    {
        Ok((match_ratio, _)) => UpdateSubmissionHistory {
            id,
            score: match_ratio as f32 * question.score,
            compilation_error: None,
        },
        Err(error) => {
            eprintln!("{:?}", error);

            UpdateSubmissionHistory {
                id,
                score: 0.,
                compilation_error: Some("Failed to run the html"),
            }
        }
    };
    update_data
        .update(database.get().await?.deref_mut())
        .await?;

    Ok(())
}

#[tokio::main]
async fn main() -> Result<()> {
    let s3_client = aws_sdk_s3::Client::new(&aws_config::load_from_env().await);

    let manager = AsyncDieselConnectionManager::new(&CONFIG.database_url);
    let database = Pool::builder(manager).build()?;

    let amqp_connection =
        lapin::Connection::connect(&CONFIG.amqp_url, ConnectionProperties::default()).await?;
    let channel = amqp_connection.create_channel().await?;
    let mut consumer = channel
        .basic_consume(
            &CONFIG.html_queue_name,
            &CONFIG.id,
            BasicConsumeOptions::default(),
            FieldTable::default(),
        )
        .await?;

    let browser = browser::spawn().await?;

    while let Some(delivery) = consumer.next().await {
        let delivery = delivery?;
        delivery.ack(BasicAckOptions::default()).await?;

        if let Err(error) = process(delivery, &s3_client, database.clone(), &browser).await {
            eprintln!("{:?}", error);
        }
    }

    Ok(())
}
