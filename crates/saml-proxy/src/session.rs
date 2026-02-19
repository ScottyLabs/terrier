use chrono::{DateTime, Utc};
use dashmap::DashMap;
use std::sync::Arc;
use std::time::Duration;

const SESSION_TTL: Duration = Duration::from_secs(15 * 60);
const CLEANUP_INTERVAL: Duration = Duration::from_secs(5 * 60);

pub struct AuthSession {
    pub relay_state: Option<String>,
    pub original_request_id: String,
    pub sp_acs_url: String,
    pub sp_entity_id: String,
    pub selected_university: Option<String>,
    pub proxy_request_id: Option<String>,
    pub created_at: DateTime<Utc>,
}

#[derive(Clone)]
pub struct SessionStore {
    inner: Arc<DashMap<String, AuthSession>>,
}

impl Default for SessionStore {
    fn default() -> Self {
        Self::new()
    }
}

impl SessionStore {
    pub fn new() -> Self {
        Self {
            inner: Arc::new(DashMap::new()),
        }
    }

    pub fn create(
        &self,
        original_request_id: String,
        sp_acs_url: String,
        sp_entity_id: String,
        relay_state: Option<String>,
    ) -> String {
        let id = uuid::Uuid::new_v4().to_string();
        let session = AuthSession {
            relay_state,
            original_request_id,
            sp_acs_url,
            sp_entity_id,
            selected_university: None,
            proxy_request_id: None,
            created_at: Utc::now(),
        };
        self.inner.insert(id.clone(), session);
        id
    }

    pub fn get(&self, id: &str) -> Option<dashmap::mapref::one::Ref<'_, String, AuthSession>> {
        let entry = self.inner.get(id)?;
        let elapsed = Utc::now()
            .signed_duration_since(entry.created_at)
            .to_std()
            .unwrap_or(Duration::ZERO);

        if elapsed > SESSION_TTL {
            drop(entry);
            self.inner.remove(id);
            return None;
        }

        Some(entry)
    }

    pub fn update_proxy_request_id(&self, id: &str, request_id: String) -> bool {
        if let Some(mut entry) = self.inner.get_mut(id) {
            entry.proxy_request_id = Some(request_id);
            true
        } else {
            false
        }
    }

    pub fn update_university(&self, id: &str, entity_id: String) -> bool {
        if let Some(mut entry) = self.inner.get_mut(id) {
            entry.selected_university = Some(entity_id);
            true
        } else {
            false
        }
    }

    pub fn remove(&self, id: &str) -> Option<AuthSession> {
        self.inner.remove(id).map(|(_, session)| session)
    }

    fn cleanup_expired(&self) {
        let now = Utc::now();
        self.inner.retain(|_, session| {
            let elapsed = now
                .signed_duration_since(session.created_at)
                .to_std()
                .unwrap_or(Duration::ZERO);
            elapsed <= SESSION_TTL
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn create_and_get() {
        let store = SessionStore::new();
        let id = store.create(
            "req_123".into(),
            "https://sp.example.com/acs".into(),
            "https://sp.example.com".into(),
            Some("relay".into()),
        );

        let session = store.get(&id).unwrap();
        assert_eq!(session.original_request_id, "req_123");
        assert_eq!(session.sp_acs_url, "https://sp.example.com/acs");
        assert_eq!(session.sp_entity_id, "https://sp.example.com");
        assert_eq!(session.relay_state.as_deref(), Some("relay"));
        assert!(session.selected_university.is_none());
        assert!(session.proxy_request_id.is_none());
    }

    #[test]
    fn get_nonexistent_returns_none() {
        let store = SessionStore::new();
        assert!(store.get("nonexistent").is_none());
    }

    #[test]
    fn update_university() {
        let store = SessionStore::new();
        let id = store.create("req".into(), "acs".into(), "sp".into(), None);

        assert!(store.update_university(&id, "https://idp.cmu.edu".into()));

        let session = store.get(&id).unwrap();
        assert_eq!(
            session.selected_university.as_deref(),
            Some("https://idp.cmu.edu")
        );
    }

    #[test]
    fn update_proxy_request_id() {
        let store = SessionStore::new();
        let id = store.create("req".into(), "acs".into(), "sp".into(), None);

        assert!(store.update_proxy_request_id(&id, "proxy_req_456".into()));

        let session = store.get(&id).unwrap();
        assert_eq!(session.proxy_request_id.as_deref(), Some("proxy_req_456"));
    }

    #[test]
    fn update_nonexistent_returns_false() {
        let store = SessionStore::new();
        assert!(!store.update_university("missing", "val".into()));
        assert!(!store.update_proxy_request_id("missing", "val".into()));
    }

    #[test]
    fn remove_returns_session() {
        let store = SessionStore::new();
        let id = store.create("req".into(), "acs".into(), "sp".into(), None);

        let session = store.remove(&id).unwrap();
        assert_eq!(session.original_request_id, "req");

        // Should be gone now
        assert!(store.get(&id).is_none());
        assert!(store.remove(&id).is_none());
    }

    #[test]
    fn expired_session_returns_none_on_get() {
        let store = SessionStore::new();
        let id = store.create("req".into(), "acs".into(), "sp".into(), None);

        // Backdate the session to make it expired
        if let Some(mut entry) = store.inner.get_mut(&id) {
            entry.created_at = Utc::now() - chrono::Duration::minutes(20);
        }

        assert!(store.get(&id).is_none());
        // The expired entry should have been removed
        assert!(store.inner.get(&id).is_none());
    }

    #[test]
    fn cleanup_removes_expired_sessions() {
        let store = SessionStore::new();
        let fresh_id = store.create("fresh".into(), "acs".into(), "sp".into(), None);
        let old_id = store.create("old".into(), "acs".into(), "sp".into(), None);

        if let Some(mut entry) = store.inner.get_mut(&old_id) {
            entry.created_at = Utc::now() - chrono::Duration::minutes(20);
        }

        store.cleanup_expired();

        assert!(store.get(&fresh_id).is_some());
        assert!(store.get(&old_id).is_none());
    }
}

pub async fn session_cleanup_task(store: SessionStore) {
    let mut interval = tokio::time::interval(CLEANUP_INTERVAL);
    loop {
        interval.tick().await;
        let before = store.inner.len();
        store.cleanup_expired();
        let removed = before - store.inner.len();
        if removed > 0 {
            tracing::info!(removed, "cleaned up expired sessions");
        }
    }
}
