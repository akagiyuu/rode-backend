use foyer::{Engine, HybridCache, HybridCacheBuilder};
use anyhow::Result;

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

    async fn _get(&self, key: &str) -> Result<Vec<u8>> {
        let response = self
            .client
            .get_object()
            .bucket(&self.bucket)
            .key(key)
            .send()
            .await
            .map_err(aws_sdk_s3::Error::from)?;

        let data = response
            .body
            .collect()
            .await
            .map_err(anyhow::Error::from)?
            .to_vec();

        Ok(data)
    }

    pub async fn get(&self, key: String) -> Result<Vec<u8>> {
        self.cache.fetch(key, || async move { self._get(&key).await });

        todo!()
    }
}
