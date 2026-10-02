PRAGMA foreign_keys = ON;

CREATE TABLE branches (
    branch_id TEXT PRIMARY KEY NOT NULL,
    branch_code TEXT NOT NULL UNIQUE,
    name TEXT NOT NULL,
    currency TEXT NOT NULL DEFAULT 'BHD' CHECK(length(currency) = 3),
    timezone TEXT NOT NULL DEFAULT 'Asia/Bahrain',
    address TEXT,
    phone TEXT,
    receipt_header TEXT,
    receipt_footer TEXT,
    tax_number TEXT,
    cr_number TEXT,
    active INTEGER NOT NULL DEFAULT 1 CHECK(active IN (0,1)),
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    deleted_at TEXT,
    version INTEGER NOT NULL DEFAULT 1 CHECK(version > 0),
    sync_state TEXT NOT NULL DEFAULT 'local'
);

CREATE TABLE roles (
    role_id TEXT PRIMARY KEY NOT NULL,
    role_code TEXT NOT NULL UNIQUE CHECK(role_code IN ('owner','manager','cashier')),
    display_name TEXT NOT NULL,
    created_at TEXT NOT NULL
);

INSERT INTO roles(role_id, role_code, display_name, created_at) VALUES
('role_owner', 'owner', 'Owner', CURRENT_TIMESTAMP),
('role_manager', 'manager', 'Manager', CURRENT_TIMESTAMP),
('role_cashier', 'cashier', 'Cashier', CURRENT_TIMESTAMP);

CREATE TABLE users (
    user_id TEXT PRIMARY KEY NOT NULL,
    branch_id TEXT NOT NULL REFERENCES branches(branch_id),
    display_name TEXT NOT NULL,
    username TEXT NOT NULL COLLATE NOCASE UNIQUE,
    pin_hash TEXT NOT NULL,
    role_code TEXT NOT NULL REFERENCES roles(role_code),
    branch_scope TEXT,
    active INTEGER NOT NULL DEFAULT 1 CHECK(active IN (0,1)),
    failed_attempts INTEGER NOT NULL DEFAULT 0 CHECK(failed_attempts >= 0),
    locked_until TEXT,
    last_login_at TEXT,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    deleted_at TEXT,
    version INTEGER NOT NULL DEFAULT 1 CHECK(version > 0),
    sync_state TEXT NOT NULL DEFAULT 'local'
);
CREATE INDEX idx_users_branch_active ON users(branch_id, active);

CREATE TABLE devices (
    device_id TEXT PRIMARY KEY NOT NULL,
    branch_id TEXT NOT NULL REFERENCES branches(branch_id),
    device_code TEXT NOT NULL,
    name TEXT NOT NULL,
    status TEXT NOT NULL DEFAULT 'active',
    active INTEGER NOT NULL DEFAULT 1 CHECK(active IN (0,1)),
    next_receipt_sequence INTEGER NOT NULL DEFAULT 1 CHECK(next_receipt_sequence > 0),
    last_seen_at TEXT,
    last_heartbeat_at TEXT,
    heartbeat_sent_at TEXT,
    observed_ip TEXT,
    app_version TEXT,
    heartbeat_sequence INTEGER NOT NULL DEFAULT 0 CHECK(heartbeat_sequence >= 0),
    hub_identity TEXT,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    deleted_at TEXT,
    version INTEGER NOT NULL DEFAULT 1 CHECK(version > 0),
    sync_state TEXT NOT NULL DEFAULT 'local',
    UNIQUE(branch_id, device_code)
);

CREATE TABLE app_config (
    config_key TEXT PRIMARY KEY NOT NULL,
    value_json TEXT NOT NULL CHECK(json_valid(value_json)),
    updated_at TEXT NOT NULL,
    updated_by TEXT
);

CREATE TABLE onboarding_state (
    step_key TEXT PRIMARY KEY NOT NULL,
    completed INTEGER NOT NULL DEFAULT 0 CHECK(completed IN (0,1)),
    completed_at TEXT,
    payload_json TEXT CHECK(payload_json IS NULL OR json_valid(payload_json)),
    updated_at TEXT NOT NULL
);

CREATE TABLE licenses (
    license_id TEXT PRIMARY KEY NOT NULL,
    license_key TEXT NOT NULL UNIQUE,
    tier TEXT NOT NULL CHECK(tier IN ('core','plus')),
    store_name TEXT NOT NULL,
    issued_at TEXT NOT NULL,
    expires_at TEXT NOT NULL,
    signature_b64 TEXT NOT NULL,
    imported_at TEXT NOT NULL,
    state TEXT NOT NULL CHECK(state IN ('active','grace','lapsed','invalid'))
);

CREATE TABLE audit_logs (
    audit_id TEXT PRIMARY KEY NOT NULL,
    event TEXT NOT NULL,
    entity_type TEXT NOT NULL,
    entity_id TEXT,
    actor_id TEXT,
    actor_type TEXT NOT NULL,
    ai_action_id TEXT,
    device_id TEXT,
    origin_device_id TEXT,
    branch_id TEXT,
    before_json TEXT CHECK(before_json IS NULL OR json_valid(before_json)),
    after_json TEXT CHECK(after_json IS NULL OR json_valid(after_json)),
    reason TEXT,
    occurred_at TEXT NOT NULL,
    hash TEXT NOT NULL CHECK(length(hash) = 64),
    previous_hash TEXT CHECK(previous_hash IS NULL OR length(previous_hash) = 64),
    override_flag INTEGER NOT NULL DEFAULT 0 CHECK(override_flag IN (0,1))
);
CREATE INDEX idx_audit_chain ON audit_logs(device_id, occurred_at, audit_id);
CREATE INDEX idx_audit_entity ON audit_logs(entity_type, entity_id, occurred_at);

CREATE TABLE diagnostics (
    diagnostic_id TEXT PRIMARY KEY NOT NULL,
    occurred_at TEXT NOT NULL,
    device_id TEXT,
    severity TEXT NOT NULL CHECK(severity IN ('info','warning','error','critical')),
    kind TEXT NOT NULL,
    message TEXT NOT NULL,
    stack TEXT,
    app_version TEXT,
    extra_json TEXT CHECK(extra_json IS NULL OR json_valid(extra_json)),
    uploaded_at TEXT
);
CREATE INDEX idx_diagnostics_kind_time ON diagnostics(kind, occurred_at DESC);

CREATE TABLE analytics_events (
    event_id TEXT PRIMARY KEY NOT NULL,
    occurred_at TEXT NOT NULL,
    device_id TEXT,
    branch_id TEXT,
    event_name TEXT NOT NULL,
    payload_json TEXT CHECK(payload_json IS NULL OR json_valid(payload_json)),
    uploaded_at TEXT
);
