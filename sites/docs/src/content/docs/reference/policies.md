---
title: Authorization policies
description: Terrier policies, their permission rules, and the data they return.
---

Policies live under `crates/terrier-server/src/auth/`. Reuse an existing policy
when possible, or combine existing policies using the combinators below. Give a
complex or reusable combination a named policy built from combinators and add
an entry here. Write a new check only when existing policies cannot express the
rule. Each entry describes the rule and what the handler receives.
The [route guide](/guides/api-routes/) explains how to add one.

## OwnProfile

| Property | Value |
| --- | --- |
| Source | `crates/terrier-server/src/auth/policy.rs` |
| Used by | `GET /api/users/{user_id}` in `src/profile.rs` |
| Inputs | `CurrentUser` and one UUID path parameter, `user_id` |
| Permission | The path UUID must equal the authenticated Terrier user UUID |
| Output | The caller's `CurrentUser`, available as `auth.data` |
| Rejections | 400 for a malformed UUID; 403 for another user's ID; 401 if no current user is present |
| Admin access | No override |
| Tests | `authenticated_requests_enforce_verification_and_csrf` in `src/auth/oidc/tests.rs` |

The user is already loaded by middleware. This policy makes no database query.

Errors use the shared [API error format](/reference/api-errors/).

## Adding an entry

Use the same fields as `OwnProfile`, including required roles and hackathon scope.
For combined policies, include branch order and error handling so callers know
which rule applies. Keep the entry updated when behavior changes.

## Combining policies

Combinators run existing policies in order. The policies must share an error type.

| Tool | Checks and result |
| --- | --- |
| `all!(A, B, C)` | Requires all checks; stops at the first failure. Returns `(a, (b, c))`. |
| `any!(A, B, C)` | Stops at the first success, identified by nested `Either::Left` / `Either::Right`. Returns the last error if all fail. |
| `policy!` | Tries named alternatives in order and returns the first successful variant, or the last error. |

A, B, and C represent existing policy types. They can be nested and given a name:

```rust
type Access = slac::any!(A, slac::all!(B, C));
```

This accepts A on its own, or both B and C. Use a type alias for a repeated
combination. If the handler needs to distinguish alternatives, `policy!` gives
them names:

```rust
slac::policy! {
    pub enum Access for AppState {
        First = A,
        Second = B,
    }
}
```

Here, `auth.data` is `Access::First(a)` or `Access::Second(b)`. With `any!(A, B)`,
it would be `Either::Left(a)` or `Either::Right(b)`. These are illustrative patterns;
the own-profile route only needs `Auth<OwnProfile>`.

`any!` and `policy!` continue on every error, including `Internal`. When a database
failure must stop the request, write an explicit check that propagates it.
Branches share request parts and do not undo changes, so checks should avoid
side effects. The macros live in `crates/slac/src/macros.rs`, with examples in
`crates/slac/tests/integration.rs`.
