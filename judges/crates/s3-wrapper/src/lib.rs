use anyhow::{Context, Result};
use foyer::{Engine, HybridCache, HybridCacheBuilder, HybridCacheEntry};

#[derive(Debug)]
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

    pub async fn get(&self, key: String) -> Result<HybridCacheEntry<String, Vec<u8>>> {
        if self.cache.contains(&key) {
            let entry = self.cache.get(&key).await?.context("Cache missed")?;

            return Ok(entry);
        }

        let response = self
            .client
            .get_object()
            .bucket(&self.bucket)
            .key(&key)
            .send()
            .await?;

        let data = response.body.collect().await?.to_vec();

        let entry = self.cache.insert(key, data);

        Ok(entry)
    }
}
