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
