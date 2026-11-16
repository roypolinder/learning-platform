---
name: write-migration
description: Write a new PostgreSQL schema migration for the learning-platform API (sqlx, apps/api/migrations). Covers file naming, SQL conventions used in this repo, safety rules for existing data, and how to apply and verify the migration locally.
---

# Write a database migration

## When to use

- A feature needs a new table, column, index, constraint or enum-like value in PostgreSQL.
- An existing column must be renamed, retyped, made `NOT NULL`, or dropped.
- Data must be backfilled as part of a schema change.

Do not use this for course content changes; those live in `courses/*.yaml`.

## Context

- Migrations live in `apps/api/migrations/` and are embedded at compile time by
  `sqlx::migrate!("./migrations")` in `apps/api/src/db.rs`.
- They run automatically on API startup (`db::migrate` in `app.rs`) and via the
  `migrate` subcommand of the `learning-platform` binary.
- sqlx records a checksum per applied migration. **Never edit a migration that has
  already been merged or applied anywhere** — always add a new file.
- Queries use runtime `sqlx::query`/`query_as` (no compile-time macros), so schema
  mismatches only show up when tests run against a real database.

## Steps

1. **Name the file** `apps/api/migrations/<YYYYMMDDHHMMSS>_<snake_case_summary>.sql`.
   The timestamp must sort after every existing migration; check the directory first.
   Use the same style as existing files, e.g. `20261114000003_add_step_feedback.sql`.
2. **Write forward-only SQL** (sqlx here has no down migrations). Follow the existing conventions:
   - Primary keys are `id UUID PRIMARY KEY` (ids are generated in Rust, not by the DB).
   - Timestamps are `TIMESTAMPTZ NOT NULL DEFAULT now()`; name them `created_at`, `updated_at`, `<event>_at`.
   - Foreign keys are explicit: `REFERENCES users (id) ON DELETE CASCADE` (or `SET NULL` for optional ownership).
   - Model enums as `TEXT` with a `CHECK (col IN (...))`, not PostgreSQL `ENUM` types.
   - Name indexes `<table>_<purpose>` (e.g. `auth_sessions_user`) and constraints `<table>_<rule>`.
   - Add a one-line `--` comment only for non-obvious intent (why, not what).
3. **Keep existing rows valid.**
   - New `NOT NULL` columns need a `DEFAULT`, or add nullable → backfill with `UPDATE` → `SET NOT NULL`.
   - When widening a `CHECK`, drop and re-add the constraint by name in the same migration.
   - Prefer adding a new column over retyping one that live code reads.
   - Never store secrets in plain text; follow `auth_sessions.token_hash` (store a hash).
4. **Update the Rust code** that reads/writes the affected tables under
   `apps/api/src/domains/`, including any `FromRow` structs and SQL strings.
5. **Start Postgres and apply**:
   ```bash
   scripts/db.sh start
   cargo run -q -p lp-api --bin learning-platform -- migrate
   ```
   Confirm it applies cleanly; if it fails, fix the new file (it is not applied yet, so editing is safe).
6. **Verify** with the full checks:
   ```bash
   cargo fmt --all
   cargo clippy --workspace --all-targets
   cargo test --workspace   # API tests need DATABASE_URL from .env
   ```
   Add or update a test in `apps/api/tests/` that exercises the new schema.
7. **Review before committing**: one logical change per migration, no edits to older
   migration files, and the timestamp is still the latest after rebasing.
