use std::path::Path;

use aws_sdk_s3::Client;

use backon::{ExponentialBuilder, Retryable};
use tokio::{fs, io::AsyncWriteExt};

pub use error::{Error, Result};

async fn _download(key: &str, bucket: &str, path: &Path, client: &Client) -> Result<()> {
    let response = client
        .get_object()
        .bucket(bucket)
        .key(key)
        .send()
        .await
        .map_err(aws_sdk_s3::Error::from)?;

    let file = response
        .body
        .collect()
        .await
        .map_err(std::io::Error::from)?
        .into_bytes();

    fs::OpenOptions::new()
        .create_new(true)
        .write(true)
        .open(path)
        .await?
        .write_all(&file)
        .await?;

    Ok(())
}

pub async fn download(
    key: &str,
    bucket: &str,
    path: &Path,
    max_retry_count: usize,
    client: &Client,
) -> Result<Vec<u8>> {
    if !path.exists() {
        let download_fn = || async move { _download(key, bucket, path, client).await };
        download_fn
            .retry(ExponentialBuilder::default().with_max_times(max_retry_count))
            .await?;
    }

    let data = fs::read(path).await?;

    Ok(data)
}
