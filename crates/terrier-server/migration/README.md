# Database migrations

Run commands from the repository root inside `collar dev shell`, or through `collar dev run -- <command>`. Start PostgreSQL with `collar dev up -d` first.

```sh
cargo run -p migration -- up
cargo run -p migration -- status
sea-orm-cli migrate generate migration_name -d crates/terrier-server/migration
generate-entities
```

`generate-entities` uses the pinned SeaORM 1.1 CLI and writes to `crates/terrier-server/src/entities`. Regenerate after applying migrations. The pinned generator combines enum labels across schemas; use a clean database for generation if failed-test schemas contain enums with the same names.

The server requires `DATABASE_URL` and applies pending migrations before opening its HTTP listener. A PostgreSQL transaction lock serializes migrations across instances. Migration failure stops startup; the transaction rolls back.

The standalone migrator remains available for manual administration while the server is stopped:

```sh
nix build .#migrator
./result/bin/migration up
```

## Runtime and migration logins

`MIGRATION_DATABASE_URL` connects as the schema owner. `DATABASE_URL` connects as a separate runtime login without superuser, CREATEROLE, schema ownership or membership in the migration role. Both URLs must target the same database and schema. Startup runs migrations, grants the runtime role domain read/insert/update and audit read access, then checks its privileges. Requests never use the migration pool.

For local PostgreSQL with trusted socket authentication, create a runtime login:

```sql
CREATE ROLE terrier_runtime LOGIN;
```

Set `DATABASE_URL` to use that login and `MIGRATION_DATABASE_URL` to use the owner. For password authentication, provision credentials through the secret provider. Do not grant audit insert/update/delete/truncate, schema CREATE, or migration-role membership to runtime.

## RFC 0008 reconciliation

The forward migrations add the reconciled workflow fields, constraints, enums, versions and audit table. Existing OIDC users are retained. Populated legacy domain tables stop migration with the table name and required mapping details; resolve missing deadlines, actors, membership times and submission evidence explicitly. The migration never guesses history or deletes conflicting rows. Rollback would lose retained history, so these migrations are forward-only.

Audit triggers require transaction context. Application handlers use `audited!`; automatic changes use `audited_automatic!` with a reason. Domain deletion and truncation are rejected. Migration tests need a login that can create test schemas and runtime roles; each successful test removes its fixtures.

## Identity migration

The OIDC migration requires an empty user table. If users already exist, it stops without modifying them. Map those users explicitly to their Keycloak issuer and subject before applying an adapted migration. Do not infer identity from email or delete existing users to make the migration pass.

This migration is forward-only: rollback would discard identity mappings.

## Integration tests

Tests create isolated PostgreSQL schemas and remove them after successful runs. Set `TEST_DATABASE_URL` to a development database where the role can create schemas. These tests are ignored in ordinary runs and must be run explicitly:

```sh
TEST_DATABASE_URL="$DATABASE_URL" cargo test -p terrier-server --test database -- --ignored
```
