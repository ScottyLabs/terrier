# API routes and policies

This guide uses `GET /api/users/{user_id}` to show how to add a route, policy, tests, and documentation. The handler is in `crates/terrier-server/src/profile.rs`; its policy is in `src/auth/policy.rs`. All `src/` paths refer to the server crate. Run commands in the development shell unless they use a `collar` wrapper.

## 1. Choose the route and response

Start with what the route does and who should be able to use it. For the profile route, the caller supplies a Terrier user UUID and may only request their own ID. A successful request returns their ID, email, and display name, represented by this type in `crates/terrier-server/src/profile.rs`:

```rust
use axum::{Json, Router, routing::get};
use serde::Serialize;
use utoipa::{OpenApi, ToSchema};

#[derive(Serialize, ToSchema)]
struct Profile {
    id: String,
    email: String,
    display_name: String,
}
```

`Serialize` enables JSON output; `ToSchema` describes the fields in OpenAPI. Use a separate response type to control which database fields are public. Request types use `Deserialize` to parse JSON.

Check the relevant RFC and models in `src/entities/`. If the route needs a schema change, add and apply a migration before running `generate-entities`. See the [database guide](database.md).

## 2. Choose the policy

Start with the [policy reference](../reference/policies.md). Reuse a policy if it already checks the required permissions, or combine existing policies:

- `leash::any!(A, B, C)` accepts the first policy that passes.
- `leash::all!(A, B, C)` requires every policy to pass.

A, B, and C stand for existing policy types. Give complex or repeated combinations a type alias. When the handler needs to know which alternative passed, `policy!` gives those branches names. For example, this pattern belongs beside the policies in `src/auth/policy.rs`:

```rust
leash::policy! {
    pub enum Access for AppState {
        First = A,
        Second = B,
    }
}
```

Like `any!(A, B)`, it tries A first, then B if A fails. A handler using `Auth<Access>` receives `Access::First(a)` or `Access::Second(b)` in `auth.data`. Each variant carries the successful policy's output.

Only write a new check when existing policies cannot express the rule. `any!` and `policy!` continue after every error, including `Internal`, so they must not be used to hide database failures.

The profile route uses `OwnProfile` in `src/auth/policy.rs` to compare the requested ID with the signed-in user's ID:

```rust
use axum::{extract::{FromRequestParts, Path}, http::request::Parts};
use leash::Policy;
use uuid::Uuid;

use super::current_user::CurrentUser;
use crate::{error::ApiError, state::AppState};

pub struct OwnProfile;

impl Policy<AppState> for OwnProfile {
    type Output = CurrentUser;
    type Error = ApiError;

    async fn check(parts: &mut Parts, state: &AppState) -> Result<Self::Output, Self::Error> {
        let user = CurrentUser::from_request_parts(parts, state).await?;
        let Path(id) = Path::<Uuid>::from_request_parts(parts, state)
            .await
            .map_err(|_| ApiError::BadRequest("Invalid user ID.".into()))?;
        if user.user().id != id {
            return Err(ApiError::Forbidden);
        }
        Ok(user)
    }
}
```

`Output` is what the handler receives after success, and `Error` is the failure type. Here, the check returns the loaded user, 400 for an invalid UUID, or 403 for another user's ID.

`FromRequestParts` reads request metadata without consuming the body. The first call gets the user attached by middleware; the second parses the path UUID. Parsing errors become `BadRequest`.

Policies live under `src/auth/` and use `AppState.db` for queries. Return the checked resource, with query failures reported as `ApiError::Internal`. Keep Terrier types out of the generic `leash` crate.

**For hackathon routes**, check the caller's active role in an active hackathon, then include that hackathon in the resource query. For example, inside a policy in `src/auth/policy.rs`, with IDs parsed from the path:

```rust
use sea_orm::{ColumnTrait, EntityTrait, QueryFilter};
use crate::entities::team;

let team = team::Entity::find_by_id(team_id)
    .filter(team::Column::HackathonId.eq(hackathon_id))
    .filter(team::Column::IsActive.eq(true))
    .one(&state.db)
    .await
    .map_err(|error| ApiError::Internal(error.into()))?
    .ok_or(ApiError::NotFound)?;
```

The filter prevents a team from another hackathon being returned, even if its ID is valid. Use `Path<(Uuid, Uuid)>` for a path with both IDs, keep the hackathon filter in writes, and check roles against that same `hackathon_id`. See the [database guide](database.md) for queries and transactions.

Add policies and named combinations to the [policy reference](../reference/policies.md) with their rule, output, errors, source, and tests. Use `OwnProfile` as the template.

## 3. Write the handler

The alias in `crates/terrier-server/src/auth/policy.rs` supplies Terrier's state type for both individual and composed policies:

```rust
pub type Auth<P> = leash::Authorized<P, AppState>;
```

Middleware has already checked login and CSRF for writes. Axum then runs `P::check`: failure returns an error before the handler runs, while success puts the checked data in `auth.data`. In `crates/terrier-server/src/profile.rs`, import `Auth` and `OwnProfile` from `crate::auth::policy`. The handler uses the returned user directly:

```rust
async fn profile(auth: Auth<OwnProfile>) -> Json<Profile> {
    let user = auth.data.user();
    Json(Profile {
        id: user.id.to_string(),
        email: user.email.clone(),
        display_name: user.display_name.clone(),
    })
}
```

`Json<Profile>` serializes the response. For a handler that can fail, return `Result<Json<Profile>, ApiError>` and wrap success in `Ok(Json(...))`.

For a request body, put `Json<Input>` last in the handler arguments because it consumes the body. Map parsing failures to `ApiError::BadRequest`; use `ApiError::Validation` for parsed input that fails a field rule. Axum's default extractor errors do not automatically use Terrier's JSON error format.

The profile handler only reads, so it needs no audit block. For writes, add `audit: Audit` and use `audited!`. Use the block's `db` for every query. Triggers record the actor and old/new values in the same transaction. A failed write or audit insert rolls back both. The block returns its `Ok(value)` after commit.

To add a display-name update route, validate the input and update the user inside an audited transaction. In this example, the last committed edit wins. Organizer workflows that reject stale edits also need revision checks.

```rust
use std::collections::BTreeMap;
use axum::{Json, extract::rejection::JsonRejection, http::StatusCode};
use sea_orm::{ActiveModelTrait, Set};
use crate::{
    audited, audit::{Audit, Scope},
    auth::policy::{Auth, OwnProfile}, entities::user, error::ApiError,
};

#[derive(serde::Deserialize, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
struct Rename { display_name: String }

async fn rename(
    auth: Auth<OwnProfile>,
    audit: Audit,
    input: Result<Json<Rename>, JsonRejection>,
) -> Result<StatusCode, ApiError> {
    let Json(input) = input.map_err(|_| ApiError::BadRequest("Invalid JSON request.".into()))?;
    let name = input.display_name.trim().to_owned();
    if name.is_empty() {
        return Err(ApiError::Validation(BTreeMap::from([
            ("display_name".into(), "Enter a display name.".into()),
        ])));
    }
    let changes = user::ActiveModel {
        id: Set(auth.data.user().id), display_name: Set(name), ..Default::default()
    };
    audited!(audit, Scope::Global, |db| {
        Ok(changes.update(db).await?)
    }).await?;
    Ok(StatusCode::NO_CONTENT)
}
```

Register the new handler with `.patch(rename)` and document it in OpenAPI in step 4. Return a response type or status to avoid exposing private database fields.

For privileged corrections and restoration, include an audit reason:

```rust
let saved = audited!(audit, Scope::Global, reason = "Correct misspelled name", |db| {
    Ok(changes.update(db).await?)
}).await?;
```

Every audit entry from the block receives that reason. Background jobs use `audited_automatic!` with the database pool and a required reason:

```rust
let saved = audited_automatic!(&state.db, Scope::Global, "Sync verified profile", |db| {
    Ok(changes.update(db).await?)
}).await?;
```

Import `audited_automatic` from `crate`. These entries have no human actor; blank reasons are rejected.

| Scope | Parent lock |
| -- | -- |
| `Scope::Global` | None; global records or creating a hackathon. |
| `Scope::Hackathon(id)` | Shared; ordinary hackathon writes. |
| `Scope::HackathonExclusive(id)` | Exclusive; settings, grants, invitation redemption, table allocation and restoration. |

For hackathon writes, the `Auth<...>` policy checks access before the handler runs. The `audited!` block acquires the scope's hackathon lock before running your code. Shared locks allow ordinary writes to run concurrently; an exclusive lock waits for other writers and blocks new ones.

Inside the block, lock any application and team rows the operation depends on before changing them or their related records. Every write must acquire these locks in the same order to avoid deadlocks:

1. Hackathon row, acquired by `audited!` using the selected scope.
1. Relevant application rows, sorted by UUID.
1. Relevant team rows, sorted by UUID.
1. Related records, such as team memberships.

Only lock rows involved in the operation; a profile rename does not need application or team locks.

While a request waits for a lock, the caller's role could be revoked, a team could fill up, or a deadline could pass. After acquiring the locks, use the block's `db` to reload and check the permissions, eligibility, capacity, and deadlines required for the write. Check deadlines against the current time after waiting. Return an `ApiError` if a check fails, before making changes.

For removal, use `PATCH` with `is_active: false`; do not add domain `DELETE` routes. Membership uses `removed`; join requests use `withdrawn` or `rejected`. Keep inactive records out of ordinary reads and authorization. Restoration must recheck permissions and constraints. Submitted projects cannot be withdrawn. Audit reads follow the resource's privacy rules. Keep secrets out of data and reasons. See [database lifecycle state](database.md#deactivation-and-history).

## 4. Register the route and its API description

In `crates/terrier-server/src/profile.rs`, `routes()` maps the URL to the handler. Paths here omit `/api`; the server adds that prefix.

```rust
pub fn routes() -> Router<AppState> {
    Router::new().route("/users/{user_id}", get(profile))
}
```

In the same file, put the OpenAPI annotation immediately above the handler. It describes the full URL, parameters, and responses for frontend types and API docs:

```rust
#[utoipa::path(get, path = "/api/users/{user_id}", tag = "users",
    params(("user_id" = String, Path, description = "Terrier user UUID")),
    responses((status = 200, body = Profile),
        (status = 400, body = crate::error::ErrorResponse),
        (status = 401, body = crate::error::ErrorResponse),
        (status = 403, body = crate::error::ErrorResponse),
        (status = 500, body = crate::error::ErrorResponse)))]
```

Also in `profile.rs`, list the documented handlers:

```rust
#[derive(OpenApi)]
#[openapi(paths(profile))]
pub struct ProfileApiDoc;
```

`paths(profile)` names the Rust function. `ProfileApiDoc::openapi()` builds its API description from the annotation above.

In `crates/terrier-server/src/routes.rs`, register the router and description as one array entry:

```rust
[(profile::routes(), profile::ProfileApiDoc::openapi())]
```

Registration merges each module's router and API description.

For a new module, declare it in `crates/terrier-server/src/lib.rs`\*\*, import it in \*\*`src/routes.rs`, and add its pair to this array. When adding a handler to an existing module, update that module's `routes()` and `paths(...)` lists instead. Use `request_body` for body inputs and include applicable [API errors](../reference/api-errors.md) in the annotation.

## 5. Test the route

Create `crates/terrier-server/tests/<domain>.rs`. Use the shared `TestApp` to test through the application router, including authentication and the route's policy. It supplies an isolated database and a local mock OIDC provider; no real account is needed.

Start with this profile example and change the request and assertions for your route:

```rust
mod common;

use axum::{
    body::{Body, to_bytes},
    http::{Request, StatusCode},
};
use common::TestApp;
use serde_json::Value;

#[tokio::test]
async fn member_reads_own_profile() -> anyhow::Result<()> {
    let app = TestApp::new().await?;
    let member = app.login("member").await?;
    let response = app
        .send(
            member
                .authorize(Request::get(format!("/api/users/{}", member.user_id)))
                .body(Body::empty())?,
        )
        .await?;

    assert_eq!(response.status(), StatusCode::OK);
    let body: Value = serde_json::from_slice(&to_bytes(response.into_body(), 4096).await?)?;
    assert_eq!(body["id"], member.user_id.to_string());

    app.close().await
}
```

Use `app.db` to create domain records and assign roles. Different login names create different users. `member.authorize(...)` supplies the session cookie and CSRF headers; omit it to test a signed-out request. End each test with `app.close().await` to remove its database schema.

Cover success, missing sessions, insufficient permissions, invalid input, and relevant database failures. Hackathon routes also need a case for access across hackathons. For writes, assert the resulting database state and that rejected requests leave it unchanged. See `tests/profile.rs` and `tests/auth.rs` for examples.

From the repository root, start PostgreSQL and run your suite, replacing `profile` with your test file name without `.rs`:

```sh
collar dev up postgres-setup
collar dev run -- cargo test -p terrier-server --test profile
```

## 6. Generate API types

Start the API on port 3000. It needs PostgreSQL, Valkey and Garage, which `collar dev up` starts:

```sh
cargo run -p terrier-server
```

Run this from `app/`:

```sh
deno task generate-api
```

Review the changes to `app/src/lib/api/schema.d.ts` and include them in your commit. The profile route generates entries for `/api/users/{user_id}` and `Profile`. Do not edit this file by hand. The [API reference](https://api.terrier.scottylabs.org/scalar) uses the same OpenAPI specification.

## 7. Add a usage example to the docs

Update the relevant guide in `sites/docs/src/guides/`, or create one such as `teams.md`. Include a request example, where its inputs come from, and what the caller should do on success or failure. For write requests, show the `x-csrf-token` header too.

Use the [profile guide](profiles.md) as an example. Link to [Authentication](authentication.md), [API errors](../reference/api-errors.md), and [Frontend development](frontend.md) for shared behavior. Update existing frontend callers when the API change affects them.

## 8. Try it and prepare for review

With the API running, run `deno task dev` from `app/`. Use `http://localhost:5173` for login and API requests so they share the session cookie. Visit `/api/auth/login?return_to=/`, then `/api/auth/status` for your user ID. Open `/api/users/<your-id>` to try the profile route.

Without a cookie, the same route should return 401:

```sh
curl -i http://localhost:3000/api/users/bdb2cef5-a417-4652-bb15-7c735463ee68
```

You can also inspect the operation at `http://localhost:3000/scalar` to check that its parameters, response type, and errors match the implementation.

Run your route's test suite from step 5, then check the staged files:

```sh
collar check
```

Use `collar check --files <changed-files>` before staging, or `collar check --all` for the whole repository. Hooks run the applicable tests, lints, formatting, and frontend checks.

Run `deno task build` from `app/` when frontend code or generated API types change. Run `docs` from the dev shell to preview documentation changes.

Before requesting review, check the policy entry and rendered docs links. New pages appear in the navigation automatically. Link between pages with relative paths to the Markdown file, such as `../reference/policies.md`.
