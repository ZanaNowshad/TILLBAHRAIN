# Tillbahrain Architecture Design

## Authority

This design implements the user-approved **TILLBAHRAIN — MASTER DEVELOPMENT DIRECTIVE** dated 2026-10-03. That directive is the product authority. This document decomposes it into implementable system boundaries without weakening any requirement.

## Objective

Build Tillbahrain as a production-grade Bahrain-native, offline-first retail POS and store operating system for Windows 10/11 x64. SQLite on each trusted device is the operational source of truth. Cloud, AI, WhatsApp, storefront, telemetry, updater and licensing are optional integrations and may degrade without blocking normal selling.

## Architecture

Tillbahrain is a Tauri v2 desktop application with a React 19/TypeScript/Vite frontend and a Rust 2021 domain/backend. The frontend calls typed Rust IPC commands. Domain services own validation, authorization, money/quantity arithmetic, transactions and persistence. SQLite/sqlx 0.8 provides local durable state with WAL, foreign keys, synchronous=NORMAL and a bounded connection pool.

The system is split into independently testable bounded modules:

1. **Foundation** — identity, configuration, SQLite, migrations, exact money/quantity, authentication, RBAC, audit, onboarding, startup health, localization shell.
2. **Retail core** — catalogue, barcodes, pricing, tax, shifts, cash, carts, checkout, payments, stock ledger, receipts, reprint, refunds/voids.
3. **Back office** — inventory operations, purchasing, suppliers, customers, loyalty, addresses, reports, users, riders and settings.
4. **Delivery + WhatsApp** — delivery lifecycle, exceptions, rider cash custody, WhatsApp sidecar/inbox/order review/payment review, notifications, timeline and repeat order.
5. **Multi-terminal** — device trust, LAN hub, pairing, mDNS, sync registry, heartbeat, parity, conflicts, DLQ and reconciliation.
6. **Optional systems** — AI, market intelligence, image search, migration agent, storefront, Cloudflare worker, backup/restore, advisory licensing, signed updater and telemetry.
7. **Release hardening** — security, accessibility, RTL, performance, E2E, Windows clean install and artifact evidence.

## Cross-cutting invariants

- Money is integer minor units; never IEEE floating point.
- Inventory/sale quantities use exact decimal semantics.
- Completed financial history is immutable. Corrections use compensating events.
- Checkout, refunds, PO receiving, delivery payment confirmation, loyalty mutations and sync application are transactional where the directive requires atomicity.
- Retried mutations use idempotency keys and must not duplicate durable effects.
- `stock_movements` is inventory authority; `stock_levels` is a derived cache.
- `loyalty_events` is loyalty authority; customer points are a cache.
- Historical reports and receipts use transaction snapshots, not current catalogue rows.
- Actor and branch context are derived from authenticated session/device state, not trusted from the frontend.
- Authorization fails closed.
- Optional integrations cannot invalidate a committed local sale.
- Secrets belong in the OS credential manager, never source, migrations, ordinary DB configuration, diagnostics or audit plaintext.

## Foundation slice

The first implementation slice establishes enough trusted infrastructure for every later subsystem:

- `com.tillbahrain.pos`, Tillbahrain branding, single-instance Tauri shell.
- SQLite initialization at `tillbahrain.db`, automatic embedded migrations and startup health reporting.
- Exact `Money` and `Quantity` primitives.
- Foundation tables: branches, roles, users, devices, app_config, onboarding_state, audit_logs.
- Owner/manager/cashier role model and Argon2id PIN authentication primitives.
- Opaque in-process session tokens with expiry and role re-check.
- SHA-256 linked audit-chain append/verification primitives.
- Resumable onboarding state and a minimal English/Arabic shell with RTL direction switching.
- CI that compiles/tests Rust and frontend and performs invoke-contract checking.

## Error strategy

Domain errors are typed and mapped to stable IPC error payloads. Startup health distinguishes `ready`, `degraded` and `error`. SQLite/migration failures are hard startup errors; optional subsystem failures are degraded states. Frontend local-storage failures are soft failures.

## Testing strategy

Tests are behavior-first and follow red→green cycles. Every slice must provide focused unit/domain tests plus impacted-suite verification. Migrations are tested on a fresh DB and re-run. CI is the authority for Rust verification when the execution environment lacks Rust.

## Release strategy

No release is called complete until Windows clean-install evidence exists, including installer filename, bytes, SHA-256, exact commit and test OS. Until then status is `PARTIALLY COMPLETE`, `FUNCTIONALLY COMPLETE WITH LIMITATIONS`, or `BLOCKED` as required by the directive.
