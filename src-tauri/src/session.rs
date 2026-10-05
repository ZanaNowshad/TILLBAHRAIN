use rand::{rngs::OsRng, RngCore};
use sha2::{Digest, Sha256};
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

    pub async fn issue(&self, user_id: impl Into<String>) -> String {
        let mut raw = [0_u8; 32];
        let mut rng = OsRng;
        rng.fill_bytes(&mut raw);
        let digest = token_digest(&raw);
        self.sessions.write().await.insert(
            digest,
            SessionRecord {
                user_id: user_id.into(),
                expires_at: Instant::now() + self.ttl,
            },
        );
        hex::encode(raw)
    }

    pub async fn resolve_user_id(&self, token: &str) -> Option<String> {
        let raw = decode_token(token)?;
        let digest = token_digest(&raw);
        let mut sessions = self.sessions.write().await;
        let record = sessions.get(&digest)?.clone();
        if record.expires_at <= Instant::now() {
            sessions.remove(&digest);
            return None;
        }
        Some(record.user_id)
    }

    pub async fn revoke(&self, token: &str) -> bool {
        let Some(raw) = decode_token(token) else {
            return false;
        };
        self.sessions
            .write()
            .await
            .remove(&token_digest(&raw))
            .is_some()
    }

    pub async fn len(&self) -> usize {
        self.sessions.read().await.len()
    }

    pub async fn is_empty(&self) -> bool {
        self.sessions.read().await.is_empty()
    }
}

fn decode_token(token: &str) -> Option<[u8; 32]> {
    let bytes = hex::decode(token).ok()?;
    bytes.try_into().ok()
}

fn token_digest(raw: &[u8; 32]) -> [u8; 32] {
    Sha256::digest(raw).into()
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
