# SLAC: ScottyLabs Access Control

A small, domain-free Rust crate that lifts authorization out of axum handler bodies and into the type system. Handlers declare their access requirements as a type parameter, and the framework runs the check before the handler body executes.

This is the implementation of [RFC 0009](../../rfcs/0009-slac.md). Read it for the design rationale; this README is the hands-on summary.

## At a glance

```rust
use axum::http::{request::Parts, StatusCode};
use slac::{policy, Authorized, Policy};

pub struct IsHackathonAdmin;

impl Policy<AppState> for IsHackathonAdmin {
    type Output = (User, Hackathon);
    type Error = ApiError;

    fn check(parts: &mut Parts, state: &AppState)
        -> impl std::future::Future<Output = Result<(User, Hackathon), ApiError>> + Send
    {
        async move {
            // pull the slug, look up the user_hackathon_role, decide
        }
    }
}

policy! {
    pub enum SettingsAccess for AppState {
        GlobalAdmin    = IsGlobalAdmin,
        HackathonAdmin = IsHackathonAdmin,
    }
}

pub async fn delete_team(
    Authorized { data, .. }: Authorized<SettingsAccess, AppState>,
) -> Result<(), ApiError> {
    let hackathon = match data {
        SettingsAccess::GlobalAdmin(h) => h,
        SettingsAccess::HackathonAdmin((_user, h)) => h,
    };
}
```

The handler signature is the entire authorization spec. Removing the extractor either breaks the body (no `data` to destructure) or reduces the function to something a reviewer would immediately reject.

## Contents

| Item | Purpose |
|---|---|
| \[`Policy`\] | Trait you implement once per atomic check. |
| \[`Authorized<P, S>`\] | Witness extractor; constructible only via the trait impl. |
| \[`Any<A, B>`\] / \[`All<A, B>`\] / \[`Either`\] | Generic OR / AND combinators. |
| \[`policy!`\] | Sum-type policy from a list of variants. |

There is no application-domain code in this crate (no `User`, `Hackathon`, or `Election`). Consumers bring their own state and entity types.

## Project-local alias

`Authorized<P, S>` defaults `S` to `()` so stateless examples read nicely. Stateful consumers should define a one-line alias next to their `AppState`:

```rust
pub type Auth<P> = slac::Authorized<P, AppState>;
```

Handler signatures then read `Auth<SettingsAccess>` instead of `Authorized<SettingsAccess, AppState>`.

## Tests

```bash
cargo test --package slac
```

The tests under [`tests/integration.rs`](tests/integration.rs) double as a worked example: each one mounts a tiny axum router with stub policies and dispatches a request through `tower::ServiceExt::oneshot`.
