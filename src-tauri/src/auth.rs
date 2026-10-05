use crate::session::SessionStore;
use argon2::{
    password_hash::{
        rand_core::OsRng as PasswordOsRng, PasswordHash, PasswordHasher, PasswordVerifier,
        SaltString,
    },
    Argon2,
};
use sqlx::{FromRow, SqlitePool};
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
    pub fn from_db_name(name: &str) -> Option<Self> {
        match name {
            "owner" => Some(Self::Owner),
            "manager" => Some(Self::Manager),
            "cashier" => Some(Self::Cashier),
            _ => None,
        }
    }

    pub fn allows(self, permission: Permission) -> bool {
        match self {
            Role::Owner => true,
            Role::Manager => !matches!(
                permission,
                Permission::SecuritySettings
                    | Permission::DeviceTrust
                    | Permission::ExternalCredentials
                    | Permission::Backup
                    | Permission::StorefrontOwnership
            ),
            Role::Cashier => matches!(
                permission,
                Permission::PosSell
                    | Permission::HeldCart
                    | Permission::CustomerWrite
                    | Permission::DeliveryOperate
                    | Permission::ShiftCash
            ),
        }
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

#[derive(Debug, FromRow)]
struct AuthRow {
    user_id: String,
    branch_id: String,
    pin_hash: String,
    role_name: String,
    active: i64,
    failed_attempts: i64,
    is_locked: i64,
}

#[derive(Debug, FromRow)]
struct PrincipalRow {
    user_id: String,
    branch_id: String,
    role_name: String,
    active: i64,
    is_locked: i64,
}

pub fn validate_pin_format(pin: &str) -> Result<(), AuthError> {
    if (4..=6).contains(&pin.len()) && pin.bytes().all(|byte| byte.is_ascii_digit()) {
        Ok(())
    } else {
        Err(AuthError::MalformedPin)
    }
}

pub fn hash_pin(pin: &str) -> Result<String, AuthError> {
    validate_pin_format(pin)?;
    let salt = SaltString::generate(&mut PasswordOsRng);
    Argon2::default()
        .hash_password(pin.as_bytes(), &salt)
        .map(|hash| hash.to_string())
        .map_err(|_| AuthError::PinHash)
}

pub fn verify_pin(pin: &str, encoded_hash: &str) -> Result<bool, AuthError> {
    validate_pin_format(pin)?;
    let parsed = PasswordHash::new(encoded_hash).map_err(|_| AuthError::InvalidPinHash)?;
    Ok(Argon2::default()
        .verify_password(pin.as_bytes(), &parsed)
        .is_ok())
}

pub async fn authenticate(
    pool: &SqlitePool,
    sessions: &SessionStore,
    user_id: &str,
    pin: &str,
) -> Result<LoginSession, AuthError> {
    validate_pin_format(pin)?;
    let mut tx = pool.begin().await?;
    let row = sqlx::query_as::<_, AuthRow>(
        "SELECT u.user_id, u.branch_id, u.pin_hash, r.name AS role_name, u.active, u.failed_attempts, CASE WHEN u.locked_until IS NOT NULL AND datetime(u.locked_until) > CURRENT_TIMESTAMP THEN 1 ELSE 0 END AS is_locked FROM users u JOIN roles r ON r.role_id = u.role_id WHERE u.user_id = ? AND u.deleted_at IS NULL",
    )
    .bind(user_id)
    .fetch_optional(&mut *tx)
    .await?
    .ok_or(AuthError::InvalidCredentials)?;

    if row.active != 1 {
        return Err(AuthError::InactiveUser);
    }
    if row.is_locked == 1 {
        return Err(AuthError::LockedOut);
    }

    let verified = verify_pin(pin, &row.pin_hash)?;
    if !verified {
        let next_attempts = row.failed_attempts.saturating_add(1);
        if next_attempts >= MAX_FAILED_ATTEMPTS {
            let modifier = format!("+{LOCKOUT_MINUTES} minutes");
            sqlx::query(
                "UPDATE users SET failed_attempts = ?, locked_until = datetime('now', ?), updated_at = CURRENT_TIMESTAMP, version = version + 1 WHERE user_id = ?",
            )
            .bind(next_attempts)
            .bind(modifier)
            .bind(&row.user_id)
            .execute(&mut *tx)
            .await?;
            tx.commit().await?;
            return Err(AuthError::LockedOut);
        }

        sqlx::query(
            "UPDATE users SET failed_attempts = ?, updated_at = CURRENT_TIMESTAMP, version = version + 1 WHERE user_id = ?",
        )
        .bind(next_attempts)
        .bind(&row.user_id)
        .execute(&mut *tx)
        .await?;
        tx.commit().await?;
        return Err(AuthError::InvalidCredentials);
    }

    let role = Role::from_db_name(&row.role_name).ok_or(AuthError::InvalidRole)?;
    sqlx::query(
        "UPDATE users SET failed_attempts = 0, locked_until = NULL, last_login_at = CURRENT_TIMESTAMP, updated_at = CURRENT_TIMESTAMP, version = version + 1 WHERE user_id = ?",
    )
    .bind(&row.user_id)
    .execute(&mut *tx)
    .await?;
    tx.commit().await?;

    let principal = Principal {
        user_id: row.user_id,
        branch_id: row.branch_id,
        role,
    };
    let token = sessions.issue(principal.user_id.clone()).await;
    Ok(LoginSession { token, principal })
}

pub async fn resolve_principal(
    pool: &SqlitePool,
    sessions: &SessionStore,
    token: &str,
) -> Result<Principal, AuthError> {
    let user_id = sessions
        .resolve_user_id(token)
        .await
        .ok_or(AuthError::InvalidSession)?;
    let row = sqlx::query_as::<_, PrincipalRow>(
        "SELECT u.user_id, u.branch_id, r.name AS role_name, u.active, CASE WHEN u.locked_until IS NOT NULL AND datetime(u.locked_until) > CURRENT_TIMESTAMP THEN 1 ELSE 0 END AS is_locked FROM users u JOIN roles r ON r.role_id = u.role_id WHERE u.user_id = ? AND u.deleted_at IS NULL",
    )
    .bind(user_id)
    .fetch_optional(pool)
    .await?
    .ok_or(AuthError::InvalidSession)?;

    if row.active != 1 {
        return Err(AuthError::InactiveUser);
    }
    if row.is_locked == 1 {
        return Err(AuthError::LockedOut);
    }
    let role = Role::from_db_name(&row.role_name).ok_or(AuthError::InvalidRole)?;
    Ok(Principal {
        user_id: row.user_id,
        branch_id: row.branch_id,
        role,
    })
}

pub async fn authorize(
    pool: &SqlitePool,
    sessions: &SessionStore,
    token: &str,
    permission: Permission,
) -> Result<Principal, AuthError> {
    let principal = resolve_principal(pool, sessions, token).await?;
    if principal.role.allows(permission) {
        Ok(principal)
    } else {
        Err(AuthError::PermissionDenied)
    }
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
        .bind(if active { 1_i64 } else { 0_i64 })
        .execute(pool)
        .await
        .unwrap();
        pin_hash
    }

    #[test]
    fn pin_format_is_strict() {
        assert!(validate_pin_format("1234").is_ok());
        assert!(validate_pin_format("123456").is_ok());
        assert!(matches!(
            validate_pin_format("123"),
            Err(AuthError::MalformedPin)
        ));
        assert!(matches!(
            validate_pin_format("1234567"),
            Err(AuthError::MalformedPin)
        ));
        assert!(matches!(
            validate_pin_format("12a4"),
            Err(AuthError::MalformedPin)
        ));
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

        let login = authenticate(&pool, &sessions, "user-1", "4321")
            .await
            .unwrap();
        assert_eq!(login.token.len(), 64);
        assert_eq!(login.principal.role, Role::Manager);
        assert_eq!(
            resolve_principal(&pool, &sessions, &login.token)
                .await
                .unwrap(),
            login.principal
        );
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
        let failed: i64 =
            sqlx::query_scalar("SELECT failed_attempts FROM users WHERE user_id='user-1'")
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
        let failed: i64 =
            sqlx::query_scalar("SELECT failed_attempts FROM users WHERE user_id='user-1'")
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
        let login = authenticate(&pool, &sessions, "user-1", "4321")
            .await
            .unwrap();
        assert!(
            authorize(&pool, &sessions, &login.token, Permission::RefundApprove)
                .await
                .is_ok()
        );

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
