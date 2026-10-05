use sqlx::{Sqlite, Transaction};
use thiserror::Error;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NewAuditEvent {
    pub event: String,
    pub entity_type: Option<String>,
    pub entity_id: Option<String>,
    pub actor_id: Option<String>,
    pub actor_type: String,
    pub ai_action_id: Option<String>,
    pub device_id: Option<String>,
    pub origin_device_id: Option<String>,
    pub branch_id: Option<String>,
    pub before_json: Option<String>,
    pub after_json: Option<String>,
    pub reason: Option<String>,
    pub override_flag: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AuditRecord {
    pub audit_id: String,
    pub event: String,
    pub entity_type: Option<String>,
    pub entity_id: Option<String>,
    pub actor_id: Option<String>,
    pub actor_type: String,
    pub ai_action_id: Option<String>,
    pub device_id: Option<String>,
    pub origin_device_id: Option<String>,
    pub branch_id: Option<String>,
    pub before_json: Option<String>,
    pub after_json: Option<String>,
    pub reason: Option<String>,
    pub created_at: String,
    pub hash: String,
    pub previous_hash: Option<String>,
    pub override_flag: bool,
}

#[derive(Debug, Error)]
pub enum AuditError {
    #[error("audit chain mismatch at index {index}")]
    ChainMismatch { index: usize },
    #[error("database error: {0}")]
    Database(#[from] sqlx::Error),
}

pub async fn append_audit(
    _tx: &mut Transaction<'_, Sqlite>,
    _event: NewAuditEvent,
) -> Result<AuditRecord, AuditError> {
    todo!("RED: append audit record")
}

pub async fn verify_audit_chain(_pool: &sqlx::SqlitePool) -> Result<(), AuditError> {
    todo!("RED: verify stored audit chain")
}

pub fn verify_records(_records: &[AuditRecord]) -> Result<(), AuditError> {
    todo!("RED: deterministic chain verification")
}

#[cfg(test)]
mod tests {
    use super::*;
    use sqlx::sqlite::SqlitePoolOptions;

    async fn test_pool() -> sqlx::SqlitePool {
        let pool = SqlitePoolOptions::new()
            .max_connections(1)
            .connect("sqlite::memory:")
            .await
            .unwrap();
        sqlx::migrate!("./migrations").run(&pool).await.unwrap();
        pool
    }

    fn event(name: &str) -> NewAuditEvent {
        NewAuditEvent {
            event: name.to_owned(),
            entity_type: Some("sale".to_owned()),
            entity_id: Some("sale-1".to_owned()),
            actor_id: Some("user-1".to_owned()),
            actor_type: "user".to_owned(),
            ai_action_id: None,
            device_id: Some("device-1".to_owned()),
            origin_device_id: Some("device-1".to_owned()),
            branch_id: Some("branch-1".to_owned()),
            before_json: None,
            after_json: Some("{\"status\":\"paid\"}".to_owned()),
            reason: None,
            override_flag: false,
        }
    }

    #[tokio::test]
    async fn appended_records_form_a_valid_chain() {
        let pool = test_pool().await;
        let mut tx = pool.begin().await.unwrap();
        let first = append_audit(&mut tx, event("sale.created")).await.unwrap();
        let second = append_audit(&mut tx, event("sale.paid")).await.unwrap();
        tx.commit().await.unwrap();

        assert_eq!(first.previous_hash, None);
        assert_eq!(second.previous_hash.as_deref(), Some(first.hash.as_str()));
        verify_audit_chain(&pool).await.unwrap();
    }

    #[tokio::test]
    async fn mutation_is_detected() {
        let pool = test_pool().await;
        let mut tx = pool.begin().await.unwrap();
        let record = append_audit(&mut tx, event("sale.created")).await.unwrap();
        tx.commit().await.unwrap();
        sqlx::query("UPDATE audit_logs SET event='sale.changed' WHERE audit_id=?")
            .bind(record.audit_id)
            .execute(&pool)
            .await
            .unwrap();

        assert!(matches!(
            verify_audit_chain(&pool).await,
            Err(AuditError::ChainMismatch { index: 0 })
        ));
    }

    #[tokio::test]
    async fn record_reordering_is_detected() {
        let pool = test_pool().await;
        let mut tx = pool.begin().await.unwrap();
        append_audit(&mut tx, event("first")).await.unwrap();
        append_audit(&mut tx, event("second")).await.unwrap();
        tx.commit().await.unwrap();

        let rows = sqlx::query_as::<_, DbAuditRecord>(
            "SELECT audit_id,event,entity_type,entity_id,actor_id,actor_type,ai_action_id,device_id,origin_device_id,branch_id,before_json,after_json,reason,created_at,hash,previous_hash,override_flag FROM audit_logs ORDER BY rowid",
        )
        .fetch_all(&pool)
        .await
        .unwrap();
        let mut records: Vec<AuditRecord> = rows.into_iter().map(Into::into).collect();
        records.swap(0, 1);
        assert!(matches!(
            verify_records(&records),
            Err(AuditError::ChainMismatch { index: 0 })
        ));
    }
}

#[cfg(test)]
#[derive(Debug, sqlx::FromRow)]
struct DbAuditRecord {
    audit_id: String,
    event: String,
    entity_type: Option<String>,
    entity_id: Option<String>,
    actor_id: Option<String>,
    actor_type: String,
    ai_action_id: Option<String>,
    device_id: Option<String>,
    origin_device_id: Option<String>,
    branch_id: Option<String>,
    before_json: Option<String>,
    after_json: Option<String>,
    reason: Option<String>,
    created_at: String,
    hash: String,
    previous_hash: Option<String>,
    override_flag: i64,
}

#[cfg(test)]
impl From<DbAuditRecord> for AuditRecord {
    fn from(row: DbAuditRecord) -> Self {
        Self {
            audit_id: row.audit_id,
            event: row.event,
            entity_type: row.entity_type,
            entity_id: row.entity_id,
            actor_id: row.actor_id,
            actor_type: row.actor_type,
            ai_action_id: row.ai_action_id,
            device_id: row.device_id,
            origin_device_id: row.origin_device_id,
            branch_id: row.branch_id,
            before_json: row.before_json,
            after_json: row.after_json,
            reason: row.reason,
            created_at: row.created_at,
            hash: row.hash,
            previous_hash: row.previous_hash,
            override_flag: row.override_flag == 1,
        }
    }
}
