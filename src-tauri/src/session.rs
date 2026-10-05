use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;

pub const DEFAULT_SESSION_TTL: Duration = Duration::from_secs(12 * 60 * 60);

#[derive(Clone)]
pub struct SessionStore {
    sessions: Arc<RwLock<HashMap<[u8; 32], SessionRecord>>>,
    ttl: Duration,
}

#[derive(Debug, Clone)]
#[allow(dead_code)]
struct SessionRecord {
    user_id: String,
    expires_at: Instant,
}

impl Default for SessionStore {
    fn default() -> Self {
        Self::new(DEFAULT_SESSION_TTL)
    }
}

impl SessionStore {
    pub fn new(ttl: Duration) -> Self {
        Self {
            sessions: Arc::new(RwLock::new(HashMap::new())),
            ttl,
        }
    }

    pub async fn issue(&self, _user_id: impl Into<String>) -> String {
        let _ = self.ttl;
        todo!("RED: opaque random session issuance")
    }

    pub async fn resolve_user_id(&self, _token: &str) -> Option<String> {
        todo!("RED: session resolution")
    }

    pub async fn revoke(&self, _token: &str) -> bool {
        todo!("RED: session revocation")
    }

    pub async fn len(&self) -> usize {
        self.sessions.read().await.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn session_token_is_opaque_resolvable_and_revocable() {
        let sessions = SessionStore::default();
        let token = sessions.issue("user-1").await;
        assert_eq!(token.len(), 64);
        assert!(!token.contains("user-1"));
        assert_eq!(
            sessions.resolve_user_id(&token).await.as_deref(),
            Some("user-1")
        );
        assert!(sessions.revoke(&token).await);
        assert_eq!(sessions.resolve_user_id(&token).await, None);
    }

    #[tokio::test]
    async fn malformed_session_token_is_rejected() {
        let sessions = SessionStore::default();
        assert_eq!(sessions.resolve_user_id("not-a-token").await, None);
    }

    #[tokio::test]
    async fn expired_session_is_rejected() {
        let sessions = SessionStore::new(Duration::ZERO);
        let token = sessions.issue("user-1").await;
        assert_eq!(sessions.resolve_user_id(&token).await, None);
    }
}
