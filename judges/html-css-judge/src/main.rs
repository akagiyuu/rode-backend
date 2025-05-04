mod browser;
mod config;

use anyhow::Result;
use config::CONFIG;
use database::SubmissionHistory;
use diesel_async::{
    pooled_connection::{deadpool::Pool, AsyncDieselConnectionManager}, AsyncConnection, AsyncPgConnection
};
use futures::StreamExt;
use lapin::{
    ConnectionProperties,
    options::{BasicAckOptions, BasicConsumeOptions},
    types::FieldTable,
};
use uuid::Uuid;

#[tokio::main]
async fn main() -> Result<()> {
    let mut database = AsyncPgConnection::establish(&CONFIG.database_url).await?;

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
        let id = Uuid::from_slice(&delivery.data)?;

        let page = browser.new_page("about:blank").await?;
        let submission = SubmissionHistory::get(, data);
        html_image_comparer::diff();
    }

    Ok(())
}
