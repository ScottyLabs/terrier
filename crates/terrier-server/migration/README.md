# Database migrations

Run commands from the repository root inside `collar dev shell`, or through
`collar dev run -- <command>`. Start PostgreSQL with `collar dev up -d` first.

```sh
cargo run -p migration -- up
cargo run -p migration -- status
sea-orm-cli migrate generate migration_name -d crates/terrier-server/migration
generate-entities
```

`generate-entities` uses the pinned SeaORM 1.1 CLI and writes to
`crates/terrier-server/src/entities`. Regenerate after applying migrations.

The server requires `DATABASE_URL` and applies pending migrations before opening
its HTTP listener. A PostgreSQL transaction lock serializes migrations across
instances. Migration failure stops startup; the transaction rolls back.

The standalone migrator remains available for manual administration while the
server is stopped:

```sh
nix build .#migrator
./result/bin/migration up
```

## Identity migration

The OIDC migration requires an empty user table. If users already exist, it stops
without modifying them. Map those users explicitly to their Keycloak issuer and
subject before applying an adapted migration. Do not infer identity from email
or delete existing users to make the migration pass.

This migration is forward-only: rollback would discard identity mappings.

## Integration tests

Tests create isolated PostgreSQL schemas and remove them after successful runs.
Set `TEST_DATABASE_URL` to a development database where the role can create
schemas. These tests are ignored in ordinary runs and must be run explicitly:

```sh
TEST_DATABASE_URL="$DATABASE_URL" cargo test -p terrier-server --test database -- --ignored
```
