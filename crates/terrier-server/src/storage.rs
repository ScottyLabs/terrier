use anyhow::{Context, anyhow};
use aws_sdk_s3::{
    Client,
    config::{Credentials, Region, RequestChecksumCalculation, ResponseChecksumValidation},
    error::DisplayErrorContext,
    primitives::ByteStream,
};
use axum::body::Bytes;

/// S3-compatible object storage for uploaded files.
#[derive(Clone)]
pub struct Storage {
    client: Client,
    bucket: String,
}

impl Storage {
    pub fn from_env() -> anyhow::Result<Self> {
        let var = |name: &str| std::env::var(name).with_context(|| format!("{name} must be set"));
        Ok(Self::new(
            &var("S3_ENDPOINT")?,
            var("S3_REGION")?,
            var("S3_ACCESS_KEY")?,
            var("S3_SECRET_KEY")?,
            var("S3_BUCKET")?,
        ))
    }

    pub fn new(
        endpoint: &str,
        region: String,
        access_key: String,
        secret_key: String,
        bucket: String,
    ) -> Self {
        let config = aws_sdk_s3::Config::builder()
            .endpoint_url(endpoint)
            .region(Region::new(region))
            .credentials_provider(Credentials::new(
                access_key, secret_key, None, None, "terrier",
            ))
            // Garage addresses buckets by path, not by subdomain.
            .force_path_style(true)
            // Optional checksums differ across S3-compatible servers.
            .request_checksum_calculation(RequestChecksumCalculation::WhenRequired)
            .response_checksum_validation(ResponseChecksumValidation::WhenRequired)
            .build();
        Self {
            client: Client::from_conf(config),
            bucket,
        }
    }

    /// Fails unless the configured bucket is reachable with the configured key.
    pub async fn check(&self) -> anyhow::Result<()> {
        self.client
            .head_bucket()
            .bucket(&self.bucket)
            .send()
            .await
            .map_err(|error| anyhow!("{}", DisplayErrorContext(error)))
            .with_context(|| format!("reach S3 bucket {}", self.bucket))?;
        Ok(())
    }

    pub async fn put(&self, key: &str, body: Bytes, content_type: &str) -> anyhow::Result<()> {
        self.client
            .put_object()
            .bucket(&self.bucket)
            .key(key)
            .content_type(content_type)
            .body(ByteStream::from(body))
            .send()
            .await
            .map_err(|error| anyhow!("{}", DisplayErrorContext(error)))
            .with_context(|| format!("store object {key}"))?;
        Ok(())
    }

    pub async fn delete(&self, key: &str) -> anyhow::Result<()> {
        self.client
            .delete_object()
            .bucket(&self.bucket)
            .key(key)
            .send()
            .await
            .map_err(|error| anyhow!("{}", DisplayErrorContext(error)))
            .with_context(|| format!("delete object {key}"))?;
        Ok(())
    }

    pub async fn get(&self, key: &str) -> anyhow::Result<Bytes> {
        let object = self
            .client
            .get_object()
            .bucket(&self.bucket)
            .key(key)
            .send()
            .await
            .map_err(|error| anyhow!("{}", DisplayErrorContext(error)))
            .with_context(|| format!("read object {key}"))?;
        Ok(object
            .body
            .collect()
            .await
            .with_context(|| format!("read object {key}"))?
            .into_bytes())
    }
}

#[cfg(test)]
mod tests {
    use super::Storage;

    #[tokio::test]
    #[ignore = "requires Garage and the S3_* variables"]
    async fn garage_stores_and_returns_objects() -> anyhow::Result<()> {
        let storage = Storage::from_env()?;
        storage.check().await?;
        let key = format!("test/{}", uuid::Uuid::new_v4());
        storage
            .put(&key, "%PDF-1.7 test".into(), "application/pdf")
            .await?;
        assert_eq!(storage.get(&key).await?, "%PDF-1.7 test");
        storage
            .get("test/missing")
            .await
            .expect_err("missing object");
        storage.delete(&key).await?;
        storage.get(&key).await.expect_err("deleted object");
        storage.delete(&key).await?;

        let largest = axum::body::Bytes::from(vec![
            b'a';
            usize::try_from(crate::files::MAX_FILE_BYTES)
                .expect("file cap fits usize")
        ]);
        storage
            .put(&key, largest.clone(), "application/octet-stream")
            .await?;
        assert_eq!(storage.get(&key).await?, largest);
        Ok(())
    }

    #[tokio::test]
    #[ignore = "requires Garage and the S3_* variables"]
    async fn garage_rejects_the_wrong_secret() -> anyhow::Result<()> {
        let storage = Storage::new(
            &std::env::var("S3_ENDPOINT")?,
            std::env::var("S3_REGION")?,
            std::env::var("S3_ACCESS_KEY")?,
            "wrong-secret".into(),
            std::env::var("S3_BUCKET")?,
        );
        storage
            .check()
            .await
            .expect_err("storage health check should fail");
        Ok(())
    }
}
