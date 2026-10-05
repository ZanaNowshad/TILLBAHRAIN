use sqlx::sqlite::{SqliteConnectOptions, SqliteJournalMode, SqlitePoolOptions, SqliteSynchronous};
use sqlx::SqlitePool;
use std::path::Path;
use std::time::Duration;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum DatabaseError {
    #[error("database connection failed: {0}")]
    Sqlx(#[from] sqlx::Error),
    #[error("database migration failed: {0}")]
    Migration(#[from] sqlx::migrate::MigrateError),
}

#[derive(Clone)]
pub struct Database {
    pool: SqlitePool,
}

impl Database {
    pub async fn open(path: impl AsRef<Path>) -> Result<Self, DatabaseError> {
        let options = base_options()
            .filename(path.as_ref())
            .create_if_missing(true)
            .journal_mode(SqliteJournalMode::Wal);
        Self::connect(options, 5).await
    }

    #[cfg(test)]
    async fn open_memory() -> Result<Self, DatabaseError> {
        let options = base_options().filename(":memory:").create_if_missing(true);
        Self::connect(options, 1).await
    }

    async fn connect(
        options: SqliteConnectOptions,
        max_connections: u32,
    ) -> Result<Self, DatabaseError> {
        let pool = SqlitePoolOptions::new()
            .max_connections(max_connections)
            .acquire_timeout(Duration::from_secs(15))
            .connect_with(options)
            .await?;

        sqlx::migrate!("./migrations").run(&pool).await?;
        Ok(Self { pool })
    }

    pub fn pool(&self) -> &SqlitePool {
        &self.pool
    }

    pub async fn is_healthy(&self) -> bool {
        sqlx::query_scalar::<_, i64>("SELECT 1")
            .fetch_one(&self.pool)
            .await
            .is_ok()
    }
}

fn base_options() -> SqliteConnectOptions {
    SqliteConnectOptions::new()
        .foreign_keys(true)
        .synchronous(SqliteSynchronous::Normal)
        .busy_timeout(Duration::from_secs(15))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn fresh_database_runs_foundation_migration() {
        let db = Database::open_memory().await.unwrap();
        let table_count: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM sqlite_master WHERE type = 'table' AND name IN ('branches','roles','users','devices','app_config','onboarding_state','audit_logs')",
        )
        .fetch_one(db.pool())
        .await
        .unwrap();
        assert_eq!(table_count, 7);

        let role_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM roles")
            .fetch_one(db.pool())
            .await
            .unwrap();
        assert_eq!(role_count, 3);
    }

    #[tokio::test]
    async fn foreign_keys_are_enforced() {
        let db = Database::open_memory().await.unwrap();
        let enabled: i64 = sqlx::query_scalar("PRAGMA foreign_keys")
            .fetch_one(db.pool())
            .await
            .unwrap();
        assert_eq!(enabled, 1);
    }

    #[tokio::test]
    async fn migrator_is_idempotent() {
        let db = Database::open_memory().await.unwrap();
        sqlx::migrate!("./migrations").run(db.pool()).await.unwrap();
        assert!(db.is_healthy().await);
    }
}
