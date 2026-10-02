# Tillbahrain Foundation Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Establish the trusted offline-first Tillbahrain application foundation on which all retail workflows depend.

**Architecture:** Tauri v2 hosts a Rust domain/backend with typed IPC over a React 19/TypeScript/Vite frontend. SQLite/sqlx 0.8 is the operational local source of truth. Foundation modules own money/quantity arithmetic, database initialization/migrations, identity/onboarding, authentication/RBAC, audit-chain verification and startup health.

**Tech Stack:** Rust 2021, Tauri v2, sqlx 0.8 + SQLite, Tokio, Serde, rust_decimal, Argon2id, SHA-256, ULID; React 19, TypeScript, Vite.

**Spec:** `docs/superpowers/specs/2026-10-03-tillbahrain-architecture-design.md`

## Global Constraints

- Product name `Tillbahrain`; package `tillbahrain`; desktop identifier `com.tillbahrain.pos`; target version `1.0.0`.
- Windows 10/11 x64 primary target; cashier UI must remain usable at 1024×768.
- SQLite database name `tillbahrain.db`; WAL, foreign keys ON, synchronous NORMAL, busy timeout approximately 15 seconds, bounded pool.
- Money uses integer minor units; BHD exponent 3; no floating-point monetary source of truth.
- Quantities use exact decimal semantics.
- Authorization fails closed; actor/branch come from authenticated state, not frontend claims.
- English fallback with Arabic/RTL parity.
- Optional services may degrade but cannot block local POS foundation startup.

## Review Focus

1. Fresh database creation and migration failure must be explicit and recoverable.
2. BHD overflow/sign handling must never silently wrap.
3. Wrong/locked/inactive PIN paths must not create a session.
4. Session privilege checks must re-read current user role/active state.
5. Audit-chain verification must detect mutation/reordering.

---

### Task 1: Project shell and CI

**Files:**
- Create: `package.json`, `tsconfig.json`, `vite.config.ts`, `index.html`, `src/*`
- Create: `src-tauri/Cargo.toml`, `src-tauri/tauri.conf.json`, `src-tauri/src/main.rs`, `src-tauri/src/lib.rs`
- Create: `.github/workflows/ci.yml`
- Create: `scripts/check_invoke_contract.py`

**Interfaces:**
- Produces Tauri command registration and frontend typed invoke wrapper boundary.

- [ ] Add minimal frontend test/build scripts and Tauri shell metadata.
- [ ] Configure official Tauri single-instance plugin before app run; second launch focuses the existing main window.
- [ ] Add CI jobs for frontend and Rust checks/tests on Linux plus Windows Tauri build placeholder gate once the shell compiles.
- [ ] Add invoke-contract checker that fails when frontend literal invoke names are not registered in Rust.
- [ ] Verify frontend tests/build and Rust CI.

### Task 2: Exact money and quantity

**Files:**
- Create: `src-tauri/src/domain/mod.rs`
- Create: `src-tauri/src/domain/money.rs`
- Create: `src-tauri/src/domain/quantity.rs`

**Interfaces:**
- Produces `Money`, `Currency`, `Quantity` and checked arithmetic consumed by all financial/inventory services.

- [ ] Write failing money tests for BHD exponent, checked addition/subtraction, sign handling and formatting.
- [ ] Write failing quantity tests for exact decimal parsing/add/subtract and invalid input.
- [ ] Implement minimal primitives without `f32`/`f64` source-of-truth paths.
- [ ] Run focused and full Rust tests.

### Task 3: SQLite initialization and migrations

**Files:**
- Create: `src-tauri/src/db.rs`
- Create: `src-tauri/migrations/0001_foundation.sql`
- Test: `src-tauri/tests/migrations.rs`

**Interfaces:**
- Produces `Database::open(path)` / `SqlitePool` with migrations complete before use.

- [ ] Write fresh-install migration test first.
- [ ] Implement `SqliteConnectOptions` with create-if-missing, foreign keys ON, WAL, synchronous NORMAL and 15-second busy timeout.
- [ ] Create foundation schema for branches, roles, users, devices, app_config, onboarding_state and audit_logs with constraints/indexes.
- [ ] Seed role identifiers only; never seed credentials.
- [ ] Re-run migrator in test and prove idempotent schema state.

### Task 4: PIN authentication, sessions and RBAC

**Files:**
- Create: `src-tauri/src/auth.rs`
- Create: `src-tauri/src/session.rs`
- Test: `src-tauri/tests/auth.rs`

**Interfaces:**
- Produces Argon2id PIN hash/verify, lockout state updates, opaque 32-byte sessions, role enforcement helpers.

- [ ] Write tests for valid PIN, wrong PIN, inactive user, lockout and malformed PIN.
- [ ] Implement 4–6 digit validation and Argon2id hashing.
- [ ] Implement failed-attempt tracking and lockout transition in a DB transaction.
- [ ] Issue random opaque session tokens; keep non-reversible lookup digest only in memory.
- [ ] Re-read user active/role/branch on privileged resolution and fail closed.

### Task 5: Audit chain

**Files:**
- Create: `src-tauri/src/audit.rs`
- Test: `src-tauri/tests/audit.rs`

**Interfaces:**
- Produces append-only audit writer and SHA-256 chain verifier.

- [ ] Write tamper/reorder detection tests first.
- [ ] Define canonical hash material from previous hash + immutable event fields.
- [ ] Append audit row inside caller transaction when required.
- [ ] Verify full chain deterministically.

### Task 6: Onboarding and startup health IPC

**Files:**
- Create: `src-tauri/src/onboarding.rs`
- Create: `src-tauri/src/startup.rs`
- Create/modify: `src-tauri/src/lib.rs`
- Create: `src/api.ts`, `src/App.tsx`, `src/i18n.ts`, `src/styles.css`

**Interfaces:**
- Produces `onboarding_get_state`, `onboarding_mark_step`, `startup_health_check` typed IPC commands and bilingual frontend startup/setup states.

- [ ] Write Rust command/service tests for resumable ordered setup state and hard DB-error vs degraded optional-component health.
- [ ] Implement backend commands with DB-backed state.
- [ ] Add typed frontend invoke wrappers.
- [ ] Add English/Arabic dictionaries with key parity and `dir=rtl` switching.
- [ ] Render checking/ready/degraded/error startup states and six-step wizard shell.
- [ ] Add frontend tests for RTL switch and startup-state rendering.

### Task 7: Foundation verification

**Files:**
- Modify as required by failures only.

**Interfaces:**
- Consumes all foundation modules; produces CI evidence for the exact commit.

- [ ] Run `npm ci`, TypeScript, invoke-contract check, frontend tests and build.
- [ ] Run `cargo check`, fmt check, clippy `-D warnings`, Rust tests and doc tests in CI.
- [ ] Run Windows `npm run tauri build` in CI.
- [ ] Inspect failures to root cause; do not weaken tests.
- [ ] Record exact green commit before starting retail-core slice.
