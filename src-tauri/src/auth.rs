use crate::session::SessionStore;
use sqlx::SqlitePool;
use thiserror::Error;

pub const MAX_FAILED_ATTEMPTS: i64 = 5;
pub const LOCKOUT_MINUTES: i64 = 5;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Role {
    Owner,
    Manager,
    Cashier,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Permission {
    PosSell,
    HeldCart,
    CustomerWrite,
    DeliveryOperate,
    ShiftCash,
    RefundApprove,
    DiscountApprove,
    PriceOverrideApprove,
    InventoryWrite,
    Purchasing,
    RiderManage,
    Reports,
    SecuritySettings,
    DeviceTrust,
    ExternalCredentials,
    Backup,
    StorefrontOwnership,
}

impl Role {
    pub fn from_db_name(_name: &str) -> Option<Self> {
        todo!("RED: role mapping")
    }

    pub fn allows(self, _permission: Permission) -> bool {
        todo!("RED: RBAC")
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Principal {
    pub user_id: String,
    pub branch_id: String,
    pub role: Role,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LoginSession {
    pub token: String,
    pub principal: Principal,
}

#[derive(Debug, Error)]
pub enum AuthError {
    #[error("PIN must contain 4 to 6 ASCII digits")]
    MalformedPin,
    #[error("invalid credentials")]
    InvalidCredentials,
    #[error("user is inactive")]
    InactiveUser,
    #[error("user is locked")]
    LockedOut,
    #[error("session is invalid or expired")]
    InvalidSession,
    #[error("permission denied")]
    PermissionDenied,
    #[error("stored role is invalid")]
    InvalidRole,
    #[error("stored PIN hash is invalid")]
    InvalidPinHash,
    #[error("PIN hashing failed")]
    PinHash,
    #[error("database error: {0}")]
    Database(#[from] sqlx::Error),
}

pub fn validate_pin_format(_pin: &str) -> Result<(), AuthError> {
    todo!("RED: PIN validation")
}

pub fn hash_pin(_pin: &str) -> Result<String, AuthError> {
    todo!("RED: Argon2id hash")
}

pub fn verify_pin(_pin: &str, _encoded_hash: &str) -> Result<bool, AuthError> {
    todo!("RED: Argon2id verify")
}

pub async fn authenticate(
    _pool: &SqlitePool,
    _sessions: &SessionStore,
    _user_id: &str,
    _pin: &str,
) -> Result<LoginSession, AuthError> {
    todo!("RED: transactional authentication")
}

pub async fn resolve_principal(
    _pool: &SqlitePool,
    _sessions: &SessionStore,
    _token: &str,
) -> Result<Principal, AuthError> {
    todo!("RED: session principal resolution")
}

pub async fn authorize(
    _pool: &SqlitePool,
    _sessions: &SessionStore,
    _token: &str,
    _permission: Permission,
) -> Result<Principal, AuthError> {
    todo!("RED: fail-closed authorization")
}

#[cfg(test)]
mod tests {
    use super::*;
    use sqlx::sqlite::SqlitePoolOptions;

    async fn test_pool() -> SqlitePool {
        let pool = SqlitePoolOptions::new()
            .max_connections(1)
            .connect("sqlite::memory:")
            .await
            .unwrap();
        sqlx::migrate!("./migrations").run(&pool).await.unwrap();
        pool
    }

    async fn seed_user(pool: &SqlitePool, role: &str, active: bool) -> String {
        let pin_hash = hash_pin("4321").unwrap();
        sqlx::query(
            "INSERT INTO branches (branch_id, branch_code, name, created_at, updated_at) VALUES ('branch-1','MAIN','Main',CURRENT_TIMESTAMP,CURRENT_TIMESTAMP)",
        )
        .execute(pool)
        .await
        .unwrap();
        sqlx::query(
            "INSERT INTO users (user_id, branch_id, display_name, username, pin_hash, role_id, active, created_at, updated_at) VALUES ('user-1','branch-1','Test User','test',?,(SELECT role_id FROM roles WHERE name = ?),?,CURRENT_TIMESTAMP,CURRENT_TIMESTAMP)",
        )
        .bind(&pin_hash)
        .bind(role)
        .bind(i64::from(active))
        .execute(pool)
        .await
        .unwrap();
        pin_hash
    }

    #[test]
    fn pin_format_is_strict() {
        assert!(validate_pin_format("1234").is_ok());
        assert!(validate_pin_format("123456").is_ok());
        assert!(matches!(validate_pin_format("123"), Err(AuthError::MalformedPin)));
        assert!(matches!(validate_pin_format("1234567"), Err(AuthError::MalformedPin)));
        assert!(matches!(validate_pin_format("12a4"), Err(AuthError::MalformedPin)));
    }

    #[test]
    fn argon2_pin_hash_is_salted_and_verifiable() {
        let first = hash_pin("4321").unwrap();
        let second = hash_pin("4321").unwrap();
        assert_ne!(first, second);
        assert!(first.starts_with("$argon2id$"));
        assert!(verify_pin("4321", &first).unwrap());
        assert!(!verify_pin("1111", &first).unwrap());
    }

    #[test]
    fn rbac_fails_closed_for_sensitive_permissions() {
        assert!(Role::Owner.allows(Permission::DeviceTrust));
        assert!(!Role::Manager.allows(Permission::DeviceTrust));
        assert!(Role::Manager.allows(Permission::RefundApprove));
        assert!(!Role::Cashier.allows(Permission::RefundApprove));
        assert!(Role::Cashier.allows(Permission::PosSell));
    }

    #[tokio::test]
    async fn valid_pin_issues_session() {
        let pool = test_pool().await;
        seed_user(&pool, "manager", true).await;
        let sessions = SessionStore::default();

        let login = authenticate(&pool, &sessions, "user-1", "4321").await.unwrap();
        assert_eq!(login.token.len(), 64);
        assert_eq!(login.principal.role, Role::Manager);
        assert_eq!(resolve_principal(&pool, &sessions, &login.token).await.unwrap(), login.principal);
    }

    #[tokio::test]
    async fn wrong_pin_increments_attempts_without_session() {
        let pool = test_pool().await;
        seed_user(&pool, "cashier", true).await;
        let sessions = SessionStore::default();

        assert!(matches!(
            authenticate(&pool, &sessions, "user-1", "1111").await,
            Err(AuthError::InvalidCredentials)
        ));
        let failed: i64 = sqlx::query_scalar("SELECT failed_attempts FROM users WHERE user_id='user-1'")
            .fetch_one(&pool)
            .await
            .unwrap();
        assert_eq!(failed, 1);
        assert_eq!(sessions.len().await, 0);
    }

    #[tokio::test]
    async fn inactive_user_never_gets_session() {
        let pool = test_pool().await;
        seed_user(&pool, "cashier", false).await;
        let sessions = SessionStore::default();

        assert!(matches!(
            authenticate(&pool, &sessions, "user-1", "4321").await,
            Err(AuthError::InactiveUser)
        ));
        assert_eq!(sessions.len().await, 0);
    }

    #[tokio::test]
    async fn fifth_failure_transitions_to_lockout() {
        let pool = test_pool().await;
        seed_user(&pool, "cashier", true).await;
        sqlx::query("UPDATE users SET failed_attempts = 4 WHERE user_id='user-1'")
            .execute(&pool)
            .await
            .unwrap();
        let sessions = SessionStore::default();

        assert!(matches!(
            authenticate(&pool, &sessions, "user-1", "1111").await,
            Err(AuthError::LockedOut)
        ));
        let locked: i64 = sqlx::query_scalar(
            "SELECT CASE WHEN locked_until IS NOT NULL AND datetime(locked_until) > CURRENT_TIMESTAMP THEN 1 ELSE 0 END FROM users WHERE user_id='user-1'",
        )
        .fetch_one(&pool)
        .await
        .unwrap();
        assert_eq!(locked, 1);
    }

    #[tokio::test]
    async fn malformed_pin_does_not_touch_failure_counter() {
        let pool = test_pool().await;
        seed_user(&pool, "cashier", true).await;
        let sessions = SessionStore::default();

        assert!(matches!(
            authenticate(&pool, &sessions, "user-1", "12x4").await,
            Err(AuthError::MalformedPin)
        ));
        let failed: i64 = sqlx::query_scalar("SELECT failed_attempts FROM users WHERE user_id='user-1'")
            .fetch_one(&pool)
            .await
            .unwrap();
        assert_eq!(failed, 0);
    }

    #[tokio::test]
    async fn authorization_re_reads_current_role_and_active_state() {
        let pool = test_pool().await;
        seed_user(&pool, "manager", true).await;
        let sessions = SessionStore::default();
        let login = authenticate(&pool, &sessions, "user-1", "4321").await.unwrap();
        assert!(authorize(&pool, &sessions, &login.token, Permission::RefundApprove)
            .await
            .is_ok());

        sqlx::query("UPDATE users SET role_id='role_cashier' WHERE user_id='user-1'")
            .execute(&pool)
            .await
            .unwrap();
        assert!(matches!(
            authorize(&pool, &sessions, &login.token, Permission::RefundApprove).await,
            Err(AuthError::PermissionDenied)
        ));

        sqlx::query("UPDATE users SET active=0 WHERE user_id='user-1'")
            .execute(&pool)
            .await
            .unwrap();
        assert!(matches!(
            resolve_principal(&pool, &sessions, &login.token).await,
            Err(AuthError::InactiveUser)
        ));
    }
}
