use sqlx::Row;
use tempfile::tempdir;
use tillbahrain_lib::db;

#[tokio::test]
async fn fresh_database_applies_foundation_migrations_and_required_pragmas() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("tillbahrain.db");
    let pool = db::open(&path).await.expect("open Tillbahrain DB");

    let foreign_keys: i64 = sqlx::query("PRAGMA foreign_keys")
        .fetch_one(&pool)
        .await
        .unwrap()
        .get(0);
    assert_eq!(foreign_keys, 1);

    let journal_mode: String = sqlx::query("PRAGMA journal_mode")
        .fetch_one(&pool)
        .await
        .unwrap()
        .get(0);
    assert_eq!(journal_mode.to_ascii_lowercase(), "wal");

    for table in [
        "branches",
        "roles",
        "users",
        "devices",
        "app_config",
        "onboarding_state",
        "audit_logs",
        "diagnostics",
    ] {
        let count: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM sqlite_master WHERE type = 'table' AND name = ?",
        )
        .bind(table)
        .fetch_one(&pool)
        .await
        .unwrap();
        assert_eq!(count, 1, "missing required table {table}");
    }
}

#[tokio::test]
async fn migrations_are_idempotent_on_reopen() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("tillbahrain.db");
    let first = db::open(&path).await.unwrap();
    first.close().await;
    let second = db::open(&path).await.unwrap();
    let migration_rows: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM _sqlx_migrations")
        .fetch_one(&second)
        .await
        .unwrap();
    assert!(migration_rows >= 1);
}
