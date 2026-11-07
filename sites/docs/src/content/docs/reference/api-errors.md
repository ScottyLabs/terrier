---
title: API errors
description: Error responses and when to use each ApiError variant.
---

`crates/terrier-server/src/error.rs` defines `ApiError` and `ErrorResponse`.
Axum converts an `ApiError` into an HTTP status and JSON body through its
`IntoResponse` implementation.

## Returning errors

A handler that can fail returns a `Result`, such as `Result<Json<Profile>, ApiError>`.
Returning `Err(...)` produces a JSON response with this shape:

```json
{
  "code": "validation_failed",
  "message": "Request data is invalid.",
  "fields": { "name": "Name is required." }
}
```

`fields` is omitted when empty. Its keys are field paths and its values are messages.
Client-visible messages must not contain secrets or database details.

| Variant | Status / code | Use |
| --- | --- | --- |
| `BadRequest(message)` | 400 / `bad_request` | Malformed input or unknown enum value |
| `Unauthenticated` | 401 / `unauthenticated` | Missing, invalid, or expired authentication |
| `Forbidden` | 403 / `forbidden` | Caller lacks permission; also CSRF rejection |
| `NotFound` | 404 / `not_found` | Resource absent within the requested scope |
| `Conflict(message)` | 409 / `conflict` | Duplicate, stale revision, capacity, or workflow conflict |
| `Validation(fields)` | 422 / `validation_failed` | Valid input shape with invalid field values |
| `Internal(error)` | 500 / `internal_error` | Unexpected database, provider, or storage failure |

Database failures use `Internal`, whose details are logged once by
`ApiError::into_response` while the client receives a generic message. Keep secrets
out of that context and avoid logging the same failure in the handler or policy.
A failed query must remain an error rather than appearing as a missing resource.

## Examples

A policy can reject access directly:

```rust
return Err(ApiError::Forbidden);
```

For a database lookup, distinguish a failed query from a missing row:

```rust
let team = team::Entity::find_by_id(team_id)
    .filter(team::Column::HackathonId.eq(hackathon_id))
    .one(&state.db)
    .await
    .map_err(|error| ApiError::Internal(error.into()))?
    .ok_or(ApiError::NotFound)?;
```

For field validation:

```rust
return Err(ApiError::Validation(BTreeMap::from([
    ("name".into(), "Name is required.".into()),
])));
```

These fragments use `crate::error::ApiError`, `std::collections::BTreeMap`,
and, for the scoped query, `sea_orm::{ColumnTrait, EntityTrait, QueryFilter}`
and `crate::entities::team`.
Use `BadRequest` when input cannot be parsed; use `Validation` when it parses but
fails a field rule. Messages passed to `BadRequest`, `Conflict`, and `Validation`
are sent to the client. Internal error details stay in the server log.

## Frontend handling and API docs

Branch on `error.code`, not the message text. For `validation_failed`, use
`error.fields` to show messages beside the affected inputs. Network failures have
no API error body and need separate handling.

List applicable status codes with `ErrorResponse` in the route's Utoipa responses,
then regenerate the frontend types and docs schema. Axum's default `Path`, `Query`,
and `Json` rejections do not use this format automatically; map them to `ApiError`
when extracting input. See the [route guide](/guides/api-routes/) for registration
and generation steps.
