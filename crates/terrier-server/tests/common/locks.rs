use super::{ApiError, SessionLockStore};

#[derive(Clone, Default)]
pub struct MemoryLocks(
    std::sync::Arc<tokio::sync::Mutex<std::collections::HashMap<String, String>>>,
);

impl SessionLockStore for MemoryLocks {
    async fn acquire(&self, key: &str, owner: &str) -> Result<bool, ApiError> {
        let mut locks = self.0.lock().await;
        if locks.contains_key(key) {
            return Ok(false);
        }
        locks.insert(key.into(), owner.into());
        Ok(true)
    }

    async fn release(&self, key: &str, owner: &str) -> Result<(), ApiError> {
        let mut locks = self.0.lock().await;
        if locks.get(key).is_none_or(|value| value != owner) {
            return Err(ApiError::Internal(anyhow::anyhow!(
                "session lock owner changed"
            )));
        }
        locks.remove(key);
        Ok(())
    }
}
