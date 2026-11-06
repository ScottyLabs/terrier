---
title: API routes and policies
description: Add a route with its policy, tests, and documentation.
---

To add an API route, you need a handler, an access policy, tests, and documentation.
This guide follows `GET /api/users/{user_id}` through each of those pieces. You can
find the route in `crates/terrier-server/src/profile.rs` and its policy in
`crates/terrier-server/src/auth/policy.rs`. All `src/` paths below refer to the
server crate, and commands assume the development shell is ready.

## 1. Choose the route and response

Start with what the route does and who should be able to use it. For the profile
route, the caller supplies a Terrier user UUID and may only request their own ID.
A successful request returns their ID, email, and display name, represented by
this type in **`crates/terrier-server/src/profile.rs`**:

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

`#[derive(...)]` generates the code needed to use this type with other libraries:
`Serialize` lets Axum write it as JSON, while `ToSchema` describes its fields in
OpenAPI. A separate response type lets you choose which database fields to expose.
Request types use `Deserialize` to read JSON into a struct.

Before adding fields, check the relevant RFC and the models in `src/entities/`.
The profile route uses existing columns, but a route that needs a schema change
also needs a migration. Apply it before running `generate-entities`, following
the [database guide](/guides/database/).

## 2. Choose the policy

Start with the [policy reference](/reference/policies/). Reuse a policy if it
already checks the required permissions, or combine existing policies:

- `slac::any!(A, B, C)` accepts the first policy that passes.
- `slac::all!(A, B, C)` requires every policy to pass.

A, B, and C stand for existing policy types. Give complex or repeated combinations
a type alias. When the handler needs to know which alternative passed, `policy!`
gives those branches names. For example, this pattern belongs beside the policies
in **`src/auth/policy.rs`**:

```rust
slac::policy! {
    pub enum Access for AppState {
        First = A,
        Second = B,
    }
}
```

Like `any!(A, B)`, it tries A first, then B if A fails. A handler using
`Auth<Access>` receives `Access::First(a)` or `Access::Second(b)` in `auth.data`,
rather than `Either::Left(a)` or `Either::Right(b)`. Each variant carries the
successful policy's output.

Only write a new check when existing policies cannot express the rule. `any!` and
`policy!` continue after every error, including `Internal`, so they must not be
used to hide database failures.

The profile route needs a new check because no existing policy compares the
requested ID with the current user. **`crates/terrier-server/src/auth/policy.rs`**
defines `OwnProfile` for that check:

```rust
use axum::{extract::{FromRequestParts, Path}, http::request::Parts};
use slac::Policy;
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

`Output` is what the handler receives after success, and `Error` is the failure
type. Here, the check returns the loaded user, 400 for an invalid UUID, or 403 for
another user's ID.

`FromRequestParts` reads request metadata without consuming the body. The first
call gets the user attached by middleware; the second parses and unwraps the path
UUID. `.await?` waits for a result and returns early on failure, while
`.map_err(...)` changes the parsing error into `BadRequest`.

Policies live under `src/auth/` and use `AppState.db` for queries. Return the
checked resource, with query failures reported as `ApiError::Internal`. Keep
Terrier types out of the generic `slac` crate.

**For hackathon routes**, check the caller's role for the requested hackathon,
then include that hackathon in the resource query. For example, inside a policy
in **`src/auth/policy.rs`**, with IDs parsed from the path:

```rust
use sea_orm::{ColumnTrait, EntityTrait, QueryFilter};
use crate::entities::team;

let team = team::Entity::find_by_id(team_id)
    .filter(team::Column::HackathonId.eq(hackathon_id))
    .one(&state.db)
    .await
    .map_err(|error| ApiError::Internal(error.into()))?
    .ok_or(ApiError::NotFound)?;
```

The filter prevents a team from another hackathon being returned, even if its ID
is valid. Use `Path<(Uuid, Uuid)>` for a path with both IDs, keep the hackathon
filter in writes, and check roles against that same `hackathon_id`. See the
[database guide](/guides/database/) for queries and transactions.

Add policies and named combinations to the [policy reference](/reference/policies/)
with their rule, output, errors, source, and tests. Use `OwnProfile` as the template.

## 3. Write the handler

The alias in **`crates/terrier-server/src/auth/policy.rs`** supplies Terrier's
state type for both individual and composed policies:

```rust
pub type Auth<P> = slac::Authorized<P, AppState>;
```

Middleware has already checked login, verified email, and CSRF for writes. Axum
then runs `P::check`: failure returns an error before the handler runs, while
success puts the checked data in `auth.data`.
In **`crates/terrier-server/src/profile.rs`**, import `Auth` and `OwnProfile` from
`crate::auth::policy`. The handler uses the returned user directly:

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

The handler wraps its response in `Json<Profile>`. Because `user()` borrows the
checked user, `.clone()` copies its strings into the response; `.to_string()`
formats the UUID. A handler that can fail would instead return
`Result<Json<Profile>, ApiError>`, using `Ok(Json(...))` on success.

For a request body, put `Json<Input>` last in the handler arguments because it
consumes the body. Map parsing failures to `ApiError::BadRequest`; use
`ApiError::Validation` for parsed input that fails a field rule. Axum's default
extractor errors do not automatically use Terrier's JSON error format.

## 4. Register the route and its API description

In **`crates/terrier-server/src/profile.rs`**, `routes()` maps the URL to the
handler. Paths here omit `/api`; the server adds that prefix.

```rust
pub fn routes() -> Router<AppState> {
    Router::new().route("/users/{user_id}", get(profile))
}
```

In the same file, put the OpenAPI annotation immediately above the handler. It
describes the full URL, parameters, and responses for frontend types and API docs:

```rust
#[utoipa::path(get, path = "/api/users/{user_id}", tag = "users",
    params(("user_id" = String, Path, description = "Terrier user UUID")),
    responses((status = 200, body = Profile),
        (status = 400, body = crate::error::ErrorResponse),
        (status = 401, body = crate::error::ErrorResponse),
        (status = 403, body = crate::error::ErrorResponse),
        (status = 500, body = crate::error::ErrorResponse)))]
```

Also in **`profile.rs`**, list the documented handlers:

```rust
#[derive(OpenApi)]
#[openapi(paths(profile))]
pub struct ProfileApiDoc;
```

`paths(profile)` names the Rust function. `ProfileApiDoc::openapi()` builds its
API description from the annotation above.

In **`crates/terrier-server/src/routes.rs`**, register the router and description
as one array entry:

```rust
[(profile::routes(), profile::ProfileApiDoc::openapi())]
```

Each pair contains the module's router and its API description. Registration
merges both, so no changes to `server.rs` are needed.

For a new module, declare it in **`crates/terrier-server/src/lib.rs`**, import it
in **`src/routes.rs`**, and add its pair to this array. When adding a handler to an
existing module, update that module's `routes()` and `paths(...)` lists instead.
Use `request_body` for body inputs and include applicable
[API errors](/reference/api-errors/) in the annotation.

## 5. Test the route through HTTP requests

Route tests should exercise the handler and policy together. For the profile
route, the existing test sends requests through the application router and checks:

| Request | Expected response |
| --- | --- |
| Signed-in user requests their own ID | 200 with their ID, email, and display name |
| Signed-in user requests another ID | 403 |
| Signed-in user supplies a malformed UUID | 400 |
| No session cookie | 401 |
| User has unverified email | 403 with `email_verification_required` |

You can read the setup and assertions in
`authenticated_requests_enforce_verification_and_csrf`, in
`src/auth/oidc/tests.rs`. It uses a local OIDC provider and an isolated PostgreSQL
schema, so it needs no real account. Run it from the repository root:

```sh
TEST_DATABASE_URL="$DATABASE_URL" cargo test -p terrier-server authenticated_requests_enforce_verification_and_csrf -- --ignored
```

For a new domain, put route tests in `crates/terrier-server/tests/<domain>.rs`,
using the module name for `<domain>`. These files import public code through
`use terrier_server::...`. Small helper tests can stay beside their implementation
in a `#[cfg(test)] mod tests` block.

The profile test's login fixture is currently private to `auth/oidc/tests.rs`.
The first domain suite that needs authenticated requests must extract that setup
into a shared helper; there is not yet a helper to import from `tests/common/`.
Keep this setup in test code, with no authentication bypass in the server.
`tests/database.rs` shows how to create and clean up an isolated database schema.

For each route, check the success response, permission failures, invalid input,
and query failures that affect its behavior. Hackathon routes also need a test
that attempts access across hackathons. The shared login tests already cover the
OIDC and CSRF details.

Run a domain suite with the following command, replacing `<domain>` with its test
file name without `.rs`:

```sh
TEST_DATABASE_URL="$DATABASE_URL" cargo test -p terrier-server --test '<domain>' -- --include-ignored
```

Mark database-dependent tests `#[ignore = "requires PostgreSQL and TEST_DATABASE_URL"]`
so ordinary test runs can skip them. Use a development database for these tests.

## 6. Generate API types and docs

Start the API on port 3000:

```sh
cargo run -p terrier-server
```

Run this from both `app/` and `sites/docs/`:

```sh
deno task generate-api
```

For the profile example, this adds `/api/users/{user_id}` and `Profile` to
`app/src/lib/api/schema.d.ts` and `sites/docs/openapi.json`. The frontend uses the
TypeScript types; the docs site builds its endpoint reference from the OpenAPI
schema. Review both generated files and include them in the change. Do not edit
them by hand.

## 7. Write the frontend usage notes

Step 6 generates the endpoint's types and API reference. This step explains how
to use it: put a short request example and any error-handling notes in
`sites/docs/src/content/docs/guides/<domain>.md`. The profile example is documented
here, so it does not need a separate usage page.

For the profile route, the caller gets their user ID from `/api/auth/status` and
passes it as the URL parameter:

```ts
import { api } from "$lib/api/client";

const result = await api.GET("/api/users/{user_id}", {
  params: { path: { user_id: userId } },
});
```

The client fills `{user_id}` with `userId` and sends the session cookie. A
successful response is in `result.data`; an API error is in `result.error`, with
the [shared error format](/reference/api-errors/). For this route, document that
401 requires sign-in and 403 means either unverified email or a different user's
ID. The error's `code` distinguishes those cases. Network failures throw rather
than returning an API error.

For a write, also show the `x-csrf-token` header using the token from
`/api/auth/status`. Link to [Authentication](/guides/authentication/) for the
login and verification flows, and do not automatically replay writes after login.

Update an existing frontend caller if the route changes its behavior. Shared
client conventions belong in [Frontend development](/guides/frontend/), while
route-specific examples belong in the domain guide. Documenting a backend route
does not require building a new UI.

## 8. Try it and prepare for review

With the API running, run `deno task dev` from `app/`. Use
`http://localhost:5173` for login and API requests so they share the session cookie.
Visit `/api/auth/login?return_to=/`, then `/api/auth/status` for your user ID.
Open `/api/users/<your-id>` to try the profile route.

Without a cookie, the same route should return 401:

```sh
curl -i http://localhost:3000/api/users/bdb2cef5-a417-4652-bb15-7c735463ee68
```

You can also inspect the operation at `http://localhost:3000/swagger-ui/` to check
that its parameters, response type, and errors match the implementation.

Run database tests explicitly, then check the staged files:

```sh
collar check
```

Use `collar check --files <changed-files>` before staging, or
`collar check --all` for the whole repository. Hooks include the applicable tests,
lints, formatting, and frontend checks; they skip ignored database tests.

Run `deno task build` from `app/` when frontend code or generated API types change,
and from `sites/docs/` when docs or their schema change.

Before requesting review, check the policy entry and rendered docs links. New
guides need an entry in the Development sidebar in `sites/docs/astro.config.mjs`;
Reference pages appear automatically. Use site paths such as
`/reference/policies/` for links, without `.md`.
