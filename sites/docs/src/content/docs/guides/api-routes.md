---
title: API routes and policies
description: Implement and document API routes with authentication, SLAC policies, and shared errors.
---

## Overview

Application routes use the shared authentication middleware and declare resource
permissions through SLAC policy extractors. `GET /api/users/{user_id}` is the
example: its policy checks ownership, and its handler returns the checked profile.

### Relevant files

Paths below are relative to the repository root.

| Purpose | File |
| --- | --- |
| Worked API route and response schema | `crates/terrier-server/src/profile.rs` |
| Terrier policy and `Auth<P>` alias | `crates/terrier-server/src/auth/policy.rs` |
| Route registration and OpenAPI assembly | `crates/terrier-server/src/server.rs` |
| Authentication, verification, and CSRF gate | `crates/terrier-server/src/auth/middleware.rs` |
| Authenticated local user extractor | `crates/terrier-server/src/auth/current_user.rs` |
| Shared errors and response schema | `crates/terrier-server/src/error.rs` |
| Shared application state | `crates/terrier-server/src/state.rs` |
| Database models and relations | `crates/terrier-server/src/entities/` |
| Database migrations and regeneration instructions | `crates/terrier-server/migration/README.md` |
| SLAC trait and authorized extractor | `crates/slac/src/policy.rs` |
| SLAC composition | `crates/slac/src/combinators.rs`, `crates/slac/src/macros.rs` |
| SLAC usage tests | `crates/slac/tests/integration.rs` |
| Auth and profile integration tests | `crates/terrier-server/src/auth/oidc/tests.rs` |
| Browser API client | `app/src/lib/api/client.ts` |
| Generated TypeScript API contract | `app/src/lib/api/schema.d.ts` |
| Generated documentation contract | `sites/docs/openapi.json` |

[RFC 0009](https://git.cmu.dev/ScottyLabs/terrier/src/branch/feat/auth-middleware/rfcs/0009-slac.md) describes the design. See [Authentication](./authentication.md) for configuration,
sign-in, session lifetime, and logout.

## Authentication and authorization

Application requests pass through these layers in order:

1. The session layer, which loads the opaque cookie's server-side session.
2. OIDC middleware, which obtains validated claims and refreshes credentials when
   needed. Rejected credentials leave the request anonymous; infrastructure
   failures return 500.
3. User resolution, which synchronizes the local PostgreSQL user and inserts
   `CurrentUser` into request extensions.
4. The application middleware, which requires authentication and verified email. Unsafe
   methods also require a session-bound CSRF token and reject a foreign Origin.
5. Axum extractors, including `Auth<YourPolicy>`, followed by the handler if all
   extraction succeeds.

`CurrentUser` identifies the caller. Each route's policy decides whether the caller
can access the requested resource.

Health, login, callback, status, and logout are registered outside the application
middleware. Logout has its own CSRF check. Unknown API paths return JSON 404 responses,
even when the server also serves frontend assets.

## Example: own profile

`GET /api/users/{user_id}` returns the caller's own Terrier profile:

```json
{
  "id": "bdb2cef5-a417-4652-bb15-7c735463ee68",
  "email": "student@andrew.cmu.edu",
  "display_name": "Student"
}
```

The UUID is Terrier's local user ID, available from `/api/auth/status`. The
response schema includes only the ID, email, and display name. OIDC subjects and
tokens stay on the server.

### Policy

`OwnProfile` in `auth/policy.rs` implements `slac::Policy<AppState>`:

```rust
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

- `Output = CurrentUser` is the data passed to the authorized handler.
- `Error = ApiError` gives rejections the shared JSON error format.
- `check(parts, state)` reads the authenticated user from request extensions,
  extracts the path as a UUID, and compares the two IDs.

A malformed UUID returns 400. Another UUID returns 403, including for an instance
admin. The middleware has already loaded the caller's profile, so the policy returns
that user without another database query.

The alias fixes the application state type:

```rust
pub type Auth<P> = slac::Authorized<P, AppState>;
```

Axum invokes its
`FromRequestParts` implementation, which calls `P::check`. If the check fails, Axum returns the error without running the handler. Otherwise,
`auth.data` contains the policy's output. `Authorized` has a private field, so
application code cannot construct it directly.

Policies can inspect path parameters, headers, extensions, and shared state.
They use request parts, so they cannot consume a JSON body. Return the authorized resource from the policy so the handler can use it.
If a write depends on submitted data or state that can change after the check,
validate those constraints in the write transaction.

### Handler and schema

The handler in `profile.rs` declares its requirement in its signature:

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

`Profile` derives `Serialize` (converts the struct to JSON) and `ToSchema`
(describes its fields in OpenAPI):

```rust
#[derive(Serialize, ToSchema)]
struct Profile {
    id: String,
    email: String,
    display_name: String,
}
```

The `#[utoipa::path(...)]` attribute documents the full API path, path parameter,
and success and error responses:

```rust
#[utoipa::path(get, path = "/api/users/{user_id}", tag = "users",
    params(("user_id" = String, Path, description = "Terrier user UUID")),
    responses((status = 200, body = Profile),
        (status = 400, body = crate::error::ErrorResponse),
        (status = 401, body = crate::error::ErrorResponse),
        (status = 403, body = crate::error::ErrorResponse),
        (status = 500, body = crate::error::ErrorResponse)))]
```

`ProfileApiDoc` derives `OpenApi` (collects the documented operations):

```rust
#[derive(OpenApi)]
#[openapi(paths(profile))]
pub struct ProfileApiDoc;
```

For routes with a request body, derive `Deserialize` (reads JSON into the struct)
and `ToSchema` on the request type,
document it with `request_body`, and put `Json<Input>` last in the handler's
extractors because it consumes the body. Map malformed input to `ApiError::BadRequest` to use the shared error format.
Axum's default extractor rejections do not use `ErrorResponse`.
Use `ApiError::Validation` for well-formed input that fails field validation.

### Registration

`profile::routes()` returns `Router<AppState>` with the relative path
`/users/{user_id}`. In `server::run`, this router is passed to
`application_routes(...)`, then nested under `/api` inside the session layer.
The route module registers the handler without the `/api` prefix:

```rust
pub fn routes() -> Router<AppState> {
    Router::new().route("/users/{user_id}", get(profile))
}
```

`server::run` applies the middleware before nesting the router:

```rust
let application = application_routes(
    terrier_server::profile::routes(),
    state.clone(),
    config.clone(),
    client.clone(),
);
```

`server::router` adds the operation to OpenAPI:

```rust
api.merge(terrier_server::profile::ProfileApiDoc::openapi());
```

For a new route module, merge its router with `profile::routes()` before
calling `application_routes`, and merge its OpenAPI document in `server::router`.
Both registrations are required: adding OpenAPI metadata does not mount a route.
Routes added after the application layers are applied bypass those checks.
Do not register protected routes alongside public auth or health routes.

## Adding policies

Keep Terrier policies in the server's `auth/policy.rs` or a domain policy module.
The SLAC crate stays independent of Terrier users, roles, and database models.
Use `AppState.db` for SeaORM queries; the generated entities describe persisted
fields and relations. Define request and response types separately from database models.

Scope lookups to the requested hackathon and resource. Check roles and ownership against the database, not values supplied by the
caller. Pass the checked resource to the handler; avoid a second unrestricted
lookup. Use `NotFound` when a resource is absent within that scope, and
`Forbidden` for an explicit access denial. If access denial should hide whether a resource exists, return `NotFound` and
test that behavior.

`INSTANCE_ADMIN_SUBJECTS` only configures identities. It does not grant an
automatic override; each policy must implement any required admin access.

Policies can be combined as follows:

| Tool | Evaluation and output |
| --- | --- |
| `All<A, B>` | Runs A, then B if A succeeds; returns both outputs as a tuple. |
| `Any<A, B>` | Tries A, then B on any rejection; returns `Either::Left` or `Either::Right`. |
| `policy!` | Tries named variants in source order; returns the first successful variant, or the last rejection. |

All branches must share an error type. `Any` and `policy!` currently continue on
every error, including `Internal`. Do not use them for database-backed
alternatives that must propagate infrastructure failures. Write an explicit
policy that distinguishes denial from failure in that case. Checks should avoid
side effects: branches share mutable request parts and do not roll back changes.

## Shared errors

Return `Result<_, ApiError>` from fallible handlers and use `ApiError` as policy
errors. `error.rs` defines the JSON response:

```json
{
  "code": "validation_failed",
  "message": "Request data is invalid.",
  "fields": { "name": "Name is required." }
}
```

`fields` is omitted when empty. Its keys are field paths and its values are
messages. Client-visible messages must not contain secrets or database details.

| Variant | Status / code | Use |
| --- | --- | --- |
| `BadRequest(message)` | 400 / `bad_request` | Malformed input or unknown enum value |
| `Unauthenticated` | 401 / `unauthenticated` | Missing, invalid, or expired authentication |
| `Forbidden` | 403 / `forbidden` | Caller lacks permission; also CSRF rejection |
| `EmailVerificationRequired` | 403 / `email_verification_required` | Verified-email gate failed |
| `NotFound` | 404 / `not_found` | Resource absent within the requested scope |
| `Conflict(message)` | 409 / `conflict` | Duplicate, stale revision, capacity, or workflow conflict |
| `Validation(fields)` | 422 / `validation_failed` | Valid input shape with invalid field values |
| `Internal(error)` | 500 / `internal_error` | Unexpected database, provider, or storage failure |

Return database failures as `Internal` with context that excludes secrets. Do not turn
query failures into missing resources, anonymous users, or empty results.
`ApiError::into_response` logs internal failures once and sends a generic message
to the client. Do not log the same failure again in the policy or handler.
Document each operation's applicable error responses using `ErrorResponse`,
including failures from middleware and extraction.

## API client and schemas

The typed client in `app/src/lib/api/client.ts` sends requests to the current
origin. For the profile example:

```ts
const result = await api.GET("/api/users/{user_id}", {
  params: { path: { user_id: userId } },
});
```

Read `result.data` on success and handle `result.error` on failure. For unsafe
methods, obtain `csrf_token` from `/api/auth/status` and send it in the
`x-csrf-token` header. Same-origin cookies accompany requests. A 401 requires
sign-in; `email_verification_required` requires the verification flow described
in [Authentication](./authentication.md). Do not automatically replay a failed
mutation after login.

Rust request and response types, together with Utoipa attributes, define the API. The running server
serves `/openapi.json` and `/swagger-ui/`. After changing a request, response,
parameter, or documented error:

1. Start the local API on port 3000.
2. Run `deno task generate-api` in `app` and in `sites/docs` through the dev shell.
3. Review `app/src/lib/api/schema.d.ts` and `sites/docs/openapi.json`, then run the
   affected component checks. Never edit these generated files by hand.

For database changes, add a migration and apply it before running
`generate-entities`. See the migration README listed above.

## Tests

Implement routes and tests together. Cover success, permission denial, invalid
input, and relevant database failures. Reuse the existing fixtures and middleware
tests.

The PostgreSQL-backed test
`authenticated_requests_enforce_verification_and_csrf` in `auth/oidc/tests.rs`
exercises the real OIDC middleware, application gate, and profile policy. It
covers anonymous access, unverified users, own-profile success, another user's
ID, malformed IDs, and CSRF behavior. It uses an isolated database schema and a
local mock OIDC provider; no real account is needed.

From the repository root, with services running via `collar dev up`:

```sh
collar dev run -- cargo test -p terrier-server
collar dev run -- sh -c 'TEST_DATABASE_URL="$DATABASE_URL" cargo test -p terrier-server authenticated_requests_enforce_verification_and_csrf -- --ignored'
```

The first command skips database tests. The second runs the profile integration
test against the development database. Run applicable hooks with
`collar check --files <changed-files>` before review.
