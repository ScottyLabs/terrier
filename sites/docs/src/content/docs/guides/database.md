---
title: Database
description: Find tables, query them with SeaORM, and change the schema.
---

Terrier uses PostgreSQL with SeaORM. Routes and policies access the connection
through `AppState.db`; they do not open a connection for each request.

## Where to look

| File | Purpose |
| --- | --- |
| `rfcs/0008-database-model.md` | Schema design, relationships, and enum overview |
| `crates/terrier-server/migration/src/` | Migrations that create and change the schema |
| `crates/terrier-server/migration/src/lib.rs` | Migration execution order |
| `crates/terrier-server/src/entities/` | Generated Rust types for tables and relations |
| `crates/terrier-server/src/entities/sea_orm_active_enums.rs` | Generated enum types and database values |
| `crates/terrier-server/src/state.rs` | Connection and startup migrations |
| `crates/terrier-server/tests/database.rs` | Database setup and integration tests |

The RFC explains the design; migrations establish the actual schema. For example,
the identity migration adds `oidc_issuer` and `oidc_subject` to `user` and removes
the old `is_global_admin` column. Use the current entities when writing queries.

## Tables and relationships

Most records have UUID primary keys. Link tables often use a pair of IDs as their
primary key, such as `(user_id, hackathon_id)` in `user_hackathon_role`.

| Area | Tables and relationships |
| --- | --- |
| Users and access | `user` stores local profiles and OIDC identities. `user_hackathon_role` assigns a role within a hackathon; `invitation` records invitations to those roles. |
| Hackathons | `hackathon` stores event settings and application configuration. Roles include hacker, organizer, judge, and sponsor. |
| Applications | `application` links a user to a hackathon. `reimbursement_application` belongs to an application. |
| Teams and projects | `team` belongs to a hackathon; `team_member` links users to teams. `project` references a team, hackathon, and event. |
| Judging | `prize_track` belongs to a hackathon and sponsor organization. `project_track_sub` links projects to tracks; `judge_track_assign` links judges to tracks. `evaluation` records a judge's evaluation of a project for a track. |
| Sponsors and organizers | `sponsor_org` stores organizations, `sponsor` links users to them, and `organizer` links users to hackathons. |
| Schedule | `event` belongs to a hackathon. `event_check_in` records a user's attendance and who checked them in. |

A user's Terrier UUID is independent of their provider identity. The unique
`(oidc_issuer, oidc_subject)` pair identifies a login; email is not an identity key.
Login synchronizes email while preserving the UUID and locally edited display
name. Instance admin identities are configured outside the database, as described
in [Authentication](/guides/authentication/).

Some tables have JSONB fields for form answers or configurable data. Those fields
still need validation against the relevant form or API schema; JSON storage does
not validate their contents for you.

## Reading data

Each generated entity module has an `Entity` for queries, a `Model` for a returned
row, and `Column` and `Relation` enums for filters and joins. A query that may find
no row returns an `Option<Model>` inside its `Result`.

For example, inside a team policy with `team_id`, `hackathon_id`, and `state`:

```rust
use sea_orm::{ColumnTrait, EntityTrait, QueryFilter};
use crate::{entities::team, error::ApiError};

let team = team::Entity::find_by_id(team_id)
    .filter(team::Column::HackathonId.eq(hackathon_id))
    .one(&state.db)
    .await
    .map_err(|error| ApiError::Internal(error.into()))?
    .ok_or(ApiError::NotFound)?;
```

The filter keeps the lookup within the requested hackathon. `.one(...)` returns
at most one row, `.ok_or(...)` handles absence, and `.map_err(...)` handles a
failed query. Return the checked model from the policy for the handler to use.
See [API errors](/reference/api-errors/) for error choices.

A foreign key confirms that a referenced row exists; it does not grant access or
guarantee that separately referenced rows belong to the same hackathon. Policies
must check the caller's role and the resource's scope. For tables without their
own `hackathon_id`, follow the parent relation, such as check-in to event.

## Writing data

SeaORM uses `ActiveModel` for inserts and updates. `Set(value)` marks a field to
be written, while an unset field is omitted. For example, after the policy has
authorized team creation, a handler can insert a team with its assigned hackathon:

```rust
use sea_orm::{ActiveModelTrait, Set};
use crate::{entities::team, error::ApiError};

let created = team::ActiveModel {
    id: Set(uuid::Uuid::new_v4()),
    hackathon_id: Set(hackathon_id),
    name: Set(name),
    table_number: Set(None),
}
.insert(&state.db)
.await
.map_err(|error| ApiError::Internal(error.into()))?;
```

Here, `name` is the validated request value and `hackathon_id` comes from the
policy's checked scope. Updates and deletes must retain that scope in their query.
When several writes must succeed together, use a transaction and commit only
after all succeed. Checks involving capacity or concurrent changes must be enforced
within that transaction using appropriate locking or database constraints.

Use the generated enums for status and role values rather than strings. Database
models are not API response types: return a separate response struct so internal
fields, including OIDC identity details, stay private.

## Changing the schema

From the repository root in the dev shell, with PostgreSQL running:

1. Generate a migration:

   ```sh
   sea-orm-cli migrate generate add_team_description -d crates/terrier-server/migration
   ```

2. Implement the schema change in its `up` method and check that it is listed in
   `migration/src/lib.rs`. Add a new migration rather than editing one already
   applied, and keep the database diagram in RFC 0008 current.
3. Apply it and regenerate entities:

   ```sh
   cargo run -p migration -- up
   generate-entities
   ```

4. Review the migration and generated diff together, then update affected routes,
   tests, and API schemas. Entity files are generated; do not edit them by hand.

The server also applies pending migrations before opening its listener. A
PostgreSQL transaction lock prevents concurrent instances from migrating at the
same time, and migration failure stops startup. The initial OIDC identity
migration requires an empty user table and cannot be rolled back; this restriction
belongs to that migration, not to future migrations. See
`crates/terrier-server/migration/README.md` for details and manual commands.

## Tests

Database tests use a development database where the role can create schemas. Each
test creates an isolated schema and removes it after a successful run; they do
not reset the whole database. The existing setup in `tests/database.rs` is private
to that test file, so extract shared setup when another suite needs it.

Run the existing migration and identity tests from the repository root:

```sh
TEST_DATABASE_URL="$DATABASE_URL" cargo test -p terrier-server --test database -- --ignored
```

Database-dependent tests are ignored in ordinary runs and must be included
explicitly. Route tests belong in `crates/terrier-server/tests/<domain>.rs`, as
shown in the [route workflow](/guides/api-routes/).
