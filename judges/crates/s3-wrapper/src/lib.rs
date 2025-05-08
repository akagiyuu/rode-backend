mod error;

use std::path::Path;

use backon::{ExponentialBuilder, Retryable};
use foyer::{Engine, HybridCache, HybridCacheBuilder};
use tokio::{fs, io::AsyncWriteExt};

pub use error::Error;
pub type Result<T> = std::result::Result<T, Error>;

pub struct Client {
    client: aws_sdk_s3::Client,
    bucket: String,
    cache: HybridCache<String, Vec<u8>>,
}

impl Client {
    pub async fn new(client: aws_sdk_s3::Client, bucket: String) -> Result<Self> {
        // 1GB Cache
        let cache = HybridCacheBuilder::new()
            .with_name("s3")
            .memory(1024)
            .storage(Engine::Large)
            .build()
            .await?;

        Ok(Self {
            client,
            bucket,
            cache,
        })
    }

    pub async fn get(&self, key: &str) -> Result<Vec<u8>, Error> {
        let response = self
            .client
            .get_object()
            .bucket(self.bucket)
            .key(key)
            .send()
            .await?;

        let file = response
            .body
            .collect()
            .await
            .map_err(std::io::Error::from)?
            .into_bytes();

        todo!()
    }
}
