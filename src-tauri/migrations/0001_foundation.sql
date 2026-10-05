PRAGMA foreign_keys = ON;

CREATE TABLE IF NOT EXISTS branches (
    branch_id TEXT PRIMARY KEY NOT NULL,
    branch_code TEXT NOT NULL UNIQUE,
    name TEXT NOT NULL,
    currency TEXT NOT NULL DEFAULT 'BHD',
    timezone TEXT NOT NULL DEFAULT 'Asia/Bahrain',
    address TEXT,
    phone TEXT,
    receipt_header TEXT,
    receipt_footer TEXT,
    tax_number TEXT,
    cr_number TEXT,
    active INTEGER NOT NULL DEFAULT 1 CHECK (active IN (0, 1)),
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    deleted_at TEXT,
    version INTEGER NOT NULL DEFAULT 1,
    sync_state TEXT NOT NULL DEFAULT 'local'
);

CREATE TABLE IF NOT EXISTS roles (
    role_id TEXT PRIMARY KEY NOT NULL,
    name TEXT NOT NULL UNIQUE CHECK (name IN ('owner', 'manager', 'cashier')),
    created_at TEXT NOT NULL
);

INSERT OR IGNORE INTO roles (role_id, name, created_at) VALUES
    ('role_owner', 'owner', CURRENT_TIMESTAMP),
    ('role_manager', 'manager', CURRENT_TIMESTAMP),
    ('role_cashier', 'cashier', CURRENT_TIMESTAMP);

CREATE TABLE IF NOT EXISTS users (
    user_id TEXT PRIMARY KEY NOT NULL,
    branch_id TEXT NOT NULL REFERENCES branches(branch_id),
    display_name TEXT NOT NULL,
    username TEXT NOT NULL COLLATE NOCASE UNIQUE,
    pin_hash TEXT NOT NULL,
    role_id TEXT NOT NULL REFERENCES roles(role_id),
    branch_scope TEXT NOT NULL DEFAULT 'branch',
    active INTEGER NOT NULL DEFAULT 1 CHECK (active IN (0, 1)),
    failed_attempts INTEGER NOT NULL DEFAULT 0 CHECK (failed_attempts >= 0),
    locked_until TEXT,
    last_login_at TEXT,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    deleted_at TEXT,
    version INTEGER NOT NULL DEFAULT 1,
    sync_state TEXT NOT NULL DEFAULT 'local'
);

CREATE TABLE IF NOT EXISTS devices (
    device_id TEXT PRIMARY KEY NOT NULL,
    branch_id TEXT NOT NULL REFERENCES branches(branch_id),
    code TEXT NOT NULL,
    name TEXT NOT NULL,
    status TEXT NOT NULL DEFAULT 'active',
    active INTEGER NOT NULL DEFAULT 1 CHECK (active IN (0, 1)),
    next_receipt_sequence INTEGER NOT NULL DEFAULT 1 CHECK (next_receipt_sequence > 0),
    last_seen_at TEXT,
    last_heartbeat_at TEXT,
    heartbeat_sent_at TEXT,
    observed_ip TEXT,
    app_version TEXT,
    heartbeat_sequence INTEGER NOT NULL DEFAULT 0 CHECK (heartbeat_sequence >= 0),
    hub_identity TEXT,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    deleted_at TEXT,
    version INTEGER NOT NULL DEFAULT 1,
    sync_state TEXT NOT NULL DEFAULT 'local',
    UNIQUE(branch_id, code)
);

CREATE TABLE IF NOT EXISTS app_config (
    config_key TEXT PRIMARY KEY NOT NULL,
    config_value TEXT NOT NULL,
    updated_at TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS onboarding_state (
    step_key TEXT PRIMARY KEY NOT NULL,
    completed INTEGER NOT NULL DEFAULT 0 CHECK (completed IN (0, 1)),
    completed_at TEXT,
    payload_json TEXT
);

CREATE TABLE IF NOT EXISTS audit_logs (
    audit_id TEXT PRIMARY KEY NOT NULL,
    event TEXT NOT NULL,
    entity_type TEXT,
    entity_id TEXT,
    actor_id TEXT,
    actor_type TEXT NOT NULL,
    ai_action_id TEXT,
    device_id TEXT,
    origin_device_id TEXT,
    branch_id TEXT,
    before_json TEXT,
    after_json TEXT,
    reason TEXT,
    created_at TEXT NOT NULL,
    hash TEXT NOT NULL,
    previous_hash TEXT,
    override_flag INTEGER NOT NULL DEFAULT 0 CHECK (override_flag IN (0, 1))
);

CREATE INDEX IF NOT EXISTS idx_users_branch_active ON users(branch_id, active);
CREATE INDEX IF NOT EXISTS idx_devices_branch_active ON devices(branch_id, active);
CREATE INDEX IF NOT EXISTS idx_audit_branch_created ON audit_logs(branch_id, created_at);
