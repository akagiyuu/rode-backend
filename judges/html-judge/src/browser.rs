use anyhow::{Result, anyhow};
use chromiumoxide::{Browser, BrowserConfig};
use futures::StreamExt;

pub async fn spawn() -> Result<Browser> {
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
