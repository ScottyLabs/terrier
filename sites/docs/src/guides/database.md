# Database

Terrier uses PostgreSQL with SeaORM. Routes and policies access the connection through `AppState.db`; they do not open a connection for each request.

## Where to look

| File | Purpose |
| -- | -- |
| `rfcs/0008-database-model.md` | Schema design, relationships, and enum overview |
| `crates/terrier-server/migration/src/` | Migrations that create and change the schema |
| `crates/terrier-server/migration/src/lib.rs` | Migration execution order |
| `crates/terrier-server/src/entities/` | Generated Rust types for tables and relations |
| `crates/terrier-server/src/entities/sea_orm_active_enums.rs` | Generated enum types and database values |
| `crates/terrier-server/src/state.rs` | Connection and startup migrations |
| `crates/terrier-server/tests/database.rs` | Database setup and integration tests |

The RFC explains the design; migrations establish the actual schema. For example, the identity migration adds `oidc_issuer` and `oidc_subject` to `user` and removes the old `is_global_admin` column. Use the current entities when writing queries. RFC 0008 fields, lifecycle states, versions and audit storage are migrated. Domain routes and their access policies are implemented separately.

## Tables and relationships

Most records have UUID primary keys. Link tables often use a pair of IDs as their primary key, such as `(user_id, hackathon_id)` in `user_hackathon_role`.

| Area | Tables and relationships |
| -- | -- |
| Users and access | `user` stores OIDC identity and profile defaults. `user_hackathon_role` stores one primary role, scoped profile data and directory consent. Invitations target an existing user and carry scoped grants, hashed tokens and expiry/revocation state. |
| Hackathons | `hackathon` stores settings, deadlines, form schemas, directory fields, reimbursement categories and scoped sponsor associations. |
| Applications | `application` separates draft/submitted answers, admission decisions and attendance. Submitted/reviewed snapshots reference `audit_event`. Reimbursements are unique per application/category and store private evidence and decimal amounts. |
| Teams and projects | `team_member` stores requests, membership and leadership. One active membership or pending request is allowed per user/hackathon. Each team retains one project, with draft/submitted state and an event reference. |
| Judging | `project_track_sub` links projects and tracks; `judge_track_assign` stores independent manual/sponsor grants. `evaluation` stores assignments and results and references the evaluated project snapshot. Tracks retain separate current and published winners. |
| Sponsors and organizers | `sponsor` links a user, hackathon and organization. `organizer` stores hackathon-scoped capabilities. Tracks may have no sponsor. |
| Schedule | `event` belongs to a hackathon. `event_check_in` is unique per event/user and retains its validity and checking actor. |
| Audit history | `audit_event` stores the actor, scope, action, resource key, version and before/after values. Database triggers append entries alongside domain changes. |

Composite foreign keys enforce matching hackathon scope. Retained records keep their uniqueness keys; deactivation does not free them for duplicate resources.

A user's Terrier UUID is independent of their provider identity. The unique `(oidc_issuer, oidc_subject)` pair identifies a login; email is not an identity key. Login synchronizes email while preserving the UUID and locally edited display name. Instance admin identities are configured outside the database, as described in [Authentication](authentication.md).

Some tables have JSONB fields for form answers or configurable data. Those fields still need validation against the relevant form or API schema; JSON storage does not validate their contents for you.

## Reading data

Each generated entity module has an `Entity` for queries, a `Model` for a returned row, and `Column` and `Relation` enums for filters and joins. A query that may find no row returns an `Option<Model>` inside its `Result`.

For example, inside a team policy with `team_id`, `hackathon_id`, and `state`:

```rust
use sea_orm::{ColumnTrait, EntityTrait, QueryFilter};
use crate::{entities::team, error::ApiError};

let team = team::Entity::find_by_id(team_id)
    .filter(team::Column::HackathonId.eq(hackathon_id))
    .filter(team::Column::IsActive.eq(true))
    .one(&state.db)
    .await
    .map_err(|error| ApiError::Internal(error.into()))?
    .ok_or(ApiError::NotFound)?;
```

The filter keeps the lookup within the requested hackathon. `.one(...)` returns at most one row, `.ok_or(...)` handles absence, and `.map_err(...)` handles a failed query. Return the checked model from the policy for the handler to use. See [API errors](../reference/api-errors.md) for error choices.

A simple foreign key confirms that a referenced row exists. The composite keys also enforce matching hackathon scope. Neither grants access: policies must check the caller's active role, the resource's scope and the active parent hackathon. For tables without their own `hackathon_id`, follow the parent relation, such as check-in to event.

## Writing data

SeaORM uses `ActiveModel` for inserts and updates. `Set(value)` marks a field to write. On insert, unset fields use database defaults; on update, they are left unchanged. After an organizer policy checks access and the handler validates the event fields, insert through `audited!`:

```rust
use sea_orm::{ActiveModelTrait, Set};
use crate::{audited, audit::{Audit, Scope}, entities::event};

let new_event = event::ActiveModel {
    id: Set(uuid::Uuid::new_v4()),
    hackathon_id: Set(hackathon_id),
    name: Set(name),
    description: Set(description),
    start_time: Set(start_time),
    end_time: Set(end_time),
    location: Set(location),
    ..Default::default()
};
let created = audited!(audit, Scope::Hackathon(hackathon_id), |db| {
    Ok(new_event.insert(db).await?)
}).await?;
```

`hackathon_id` comes from the checked resource; the other values come from validated input. Updates must retain resource scope and, when required, the last-read version in their query. Return `409 Conflict` if a revision-qualified update changes no row. Keep related writes in one block and check concurrency rules under the required locks. For team creation, insert both the team and its leader membership before commit.

Use the generated enums for status and role values rather than strings. Database models are not API response types: return a separate response struct so internal fields, including OIDC identity details, stay private.

## Deactivation and history

Domain routes retain rows and change lifecycle state through `PATCH`. Do not use SeaORM delete methods or cascading deletes for business removal. See the [handler guidance](api-routes.md#3-write-the-handler) for permissions, revisions and restoration.

The target schema in [RFC 0008](https://git.cmu.dev/ScottyLabs/terrier/src/branch/main/rfcs/0008-database-model.md#deactivation-and-lifecycle-state) uses `is_active` for resources such as hackathons, teams and organizer links. Membership instead uses `TeamMember.status`: the API's `is_active: false` maps to `removed`, clears leadership and records the end time. Join requests use `pending`, `withdrawn` and `rejected`; acceptance sets `active`. Do not add a second activity flag to membership.

Membership states are `pending`, `active`, `withdrawn`, `rejected` and `removed`. The legacy `invited` value is retained for explicit legacy mappings, not new joins.

Commit lifecycle changes, dependent changes and audit records in one transaction. Team departure must transfer leadership or deactivate an empty team. Retain projects and prior decisions. Exclude inactive parents, resources and relationships from ordinary reads and authorization; restrict history access to authorized users. Recheck uniqueness, scope and workflow rules before restoration. Retained keys must not be freed by deleting rows; later membership episodes reuse the retained team/user row and preserve earlier episodes in audit history.

Session/token cleanup and personal-data erasure require separate policies. Submitted projects cannot be deleted or withdrawn.

## Changing the schema

From the repository root in the dev shell, with PostgreSQL running:

1. Generate a migration:

   ```sh
   sea-orm-cli migrate generate add_team_description -d crates/terrier-server/migration
   ```

1. Implement the schema change in its `up` method and check that it is listed in `migration/src/lib.rs`. Add a new migration rather than editing one already applied, and keep the database diagram in RFC 0008 current.

1. Apply it and regenerate entities:

   ```sh
   cargo run -p migration -- up
   generate-entities
   ```

1. Review the migration and generated diff together, then update affected routes, tests, and API schemas. Entity files are generated; do not edit them by hand.

On startup the server applies migrations through `DATABASE_URL` under a transaction lock, then serves requests on the same connection pool. A migration failure stops startup and rolls back.

The RFC migration accepts empty domain tables and retains existing OIDC users. Nonempty legacy domain tables require explicit mappings for missing deadlines, actors, membership times and submission evidence. It stops without changing rows rather than inventing that history. Existing migrations are unchanged and the new migrations are forward-only. See the [migration instructions](https://git.cmu.dev/ScottyLabs/terrier/src/branch/main/crates/terrier-server/migration/README.md).

## Tests

Database tests connect through `DATABASE_URL`, where the login can create schemas. Each test creates an isolated schema and removes it after a successful run; they do not reset the whole database. The existing setup in `tests/database.rs` is private to that test file, so extract shared setup when another suite needs it.

Start PostgreSQL with `pitchfork start --local`, then run the existing migration and identity tests from the repository root:

```sh
cargo test -p terrier-server --test database
```

Database tests run with the rest of the suite, and kennel's `test` and `mutants` checks give them an empty database of their own in CI. Route tests belong in `crates/terrier-server/tests/<domain>.rs`, as shown in the [route workflow](api-routes.md).
