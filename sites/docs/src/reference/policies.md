# Authorization policies

Policies live under `crates/terrier-server/src/auth/`. Reuse an existing policy when possible, or combine existing policies using the combinators below. Give a complex or reusable combination a named policy built from combinators and add an entry here. Write a new check only when existing policies cannot express the rule. Each entry describes the rule and what the handler receives. The [route guide](../guides/api-routes.md) explains how to add one.

## OwnProfile

| Property | Value |
| -- | -- |
| Source | `crates/terrier-server/src/auth/policy.rs` |
| Used by | `GET` and `PATCH /api/users/{user_id}` in `src/profile.rs` |
| Inputs | `CurrentUser` and one UUID path parameter, `user_id` |
| Permission | The path UUID must equal the authenticated Terrier user UUID |
| Output | The caller's `CurrentUser`, available as `auth.data` |
| Rejections | 400 for a malformed UUID; 403 for another user's ID; 401 if no current user is present |
| Admin access | No override |
| Tests | `tests/profile.rs`: success, ownership, malformed ID, missing session, and renames |

The user is already loaded by middleware. This policy makes no database query.

Errors use the shared [API error format](api-errors.md).

## OwnApplication

| Property | Value |
| -- | -- |
| Source | `crates/terrier-server/src/auth/policy.rs` |
| Used by | `PUT /api/hackathons/{slug}/applications/{user_id}` in `src/applications.rs` |
| Inputs | `CurrentUser`, the hackathon `slug` and the applicant `user_id` path parameters |
| Permission | The path UUID must equal the authenticated Terrier user UUID, and the hackathon must be active |
| Output | `Applicant`: the caller and hackathon IDs, and whether the caller already has a role there |
| Rejections | 400 for a malformed UUID; 403 for another user's ID; 404 for an unknown or inactive hackathon; 401 if no current user is present |
| Admin access | No override |
| Tests | `tests/registration.rs`: ownership, malformed ID, missing session, unknown hackathon |

## ReadApplication

| Property | Value |
| -- | -- |
| Source | `crates/terrier-server/src/auth/policy.rs` |
| Used by | `GET /api/hackathons/{slug}/applications/{user_id}` in `src/applications.rs` |
| Inputs | `CurrentUser`, the hackathon `slug` and the applicant `user_id` path parameters |
| Permission | The applicant, or a user with organizer access in the same hackathon (active `organizer` row or `organizer` role) |
| Output | `ApplicationTarget`: the hackathon and applicant IDs, and `ApplicationAccess::Owner`, `Manager` or `Organizer` |
| Rejections | 400 for a malformed UUID; 404 for an unknown or inactive hackathon; 403 for anyone else; 401 if no current user is present |
| Admin access | `application_management` and `hackathon_admin` capabilities give `Manager` access |
| Tests | `tests/registration.rs`: owner, manager and organizer reads; other hackers |

## ListApplications

| Property | Value |
| -- | -- |
| Source | `crates/terrier-server/src/auth/policy.rs` |
| Used by | `GET /api/hackathons/{slug}/applications` in `src/applications.rs` |
| Inputs | `CurrentUser` and the hackathon `slug` path parameter |
| Permission | Organizer access in the same hackathon (active `organizer` row or `organizer` role) |
| Output | The hackathon ID and `ApplicationAccess::Manager` or `Organizer` |
| Rejections | 404 for an unknown or inactive hackathon; 403 without organizer access; 401 if no current user is present |
| Admin access | `application_management` and `hackathon_admin` capabilities give `Manager` access |
| Tests | `tests/registration.rs`: manager and organizer views, filters, cross-hackathon access |

## BrowseDirectory

| Property | Value |
| -- | -- |
| Source | `crates/terrier-server/src/auth/policy.rs` |
| Used by | `GET /api/hackathons/{slug}/users` in `src/directory.rs` |
| Inputs | `CurrentUser` and the hackathon `slug` path parameter |
| Permission | Organizer access in the hackathon, or an active accepted application with confirmed attendance there |
| Output | The active `hackathon::Model` |
| Rejections | 404 for an unknown or inactive hackathon; 403 for anyone else; 401 if no current user is present |
| Admin access | Any organizer access |
| Tests | `tests/registration.rs`: attendee and organizer reads, unconfirmed applicant, missing session |

## ManageOrganizers

| Property | Value |
| -- | -- |
| Source | `crates/terrier-server/src/auth/policy.rs` |
| Used by | `POST /api/hackathons/{slug}/organizers` and `PATCH /api/hackathons/{slug}/organizers/{user_id}` in `src/organizers.rs` |
| Inputs | Authenticated `CurrentUser` and hackathon `slug` path parameter |
| Permission | The caller must have an active `Organizer` role with `HackathonAdmin` capability in an active hackathon |
| Output | The authorized `hackathon::Model`, available as `auth.data` |
| Rejections | 400 for invalid path parameters; 401 for unauthenticated requests; 403 for insufficient permissions; 404 for missing or inactive hackathons; 500 for database failures |
| Admin access | Requires `HackathonAdmin`; no global administrator override |
| Tests | `tests/organizers.rs`: authentication, permissions, successful creation and deactivation, duplicate creation, and missing hackathon |

The policy resolves the hackathon by slug and verifies that the caller has an active hackathon membership with `HackathonAdmin` capability. Both organizer handlers recheck authorization inside an audited transaction using `Scope::HackathonExclusive` before writing to the database.

## Hackathon setup

See `tests/hackathons.rs` for route coverage.

| Policy | Route | Permission and output |
| -- | -- | -- |
| `InstanceAdmin` | `POST /api/hackathons` | Signed-in user whose Keycloak subject appears in `INSTANCE_ADMIN_SUBJECTS`. Returns the creator's user ID. |
| `ManageOrganizers` | `PATCH /api/hackathons/{slug}` | Reuses the hackathon admin policy above. |

An unauthorized signed-in caller gets 403. Editing an inactive or unknown hackathon returns 404. The update rechecks access under the exclusive hackathon lock.

## CreateTrack

| Property | Value |
| -- | -- |
| Source | `crates/terrier-server/src/auth/policy.rs` |
| Used by | `POST /api/hackathons/{slug}/tracks` |
| Permission | Active hackathon admin, or active sponsor representing an associated organization in this hackathon |
| Output | Caller and active hackathon; the handler checks the requested sponsor organization again under the audited lock |
| Rejections | 401 without login; 403 for insufficient access or an unassociated organization; 404 for an inactive or missing hackathon |
| Tests | `tests/tracks.rs`: admin and sponsor creation, malformed input, and scope failures |

## EditTrack

| Property | Value |
| -- | -- |
| Source | `crates/terrier-server/src/auth/policy.rs` |
| Used by | `PATCH /api/hackathons/{slug}/tracks/{track_id}` |
| Permission | Active hackathon admin or judging manager, or an active sponsor representing the track's associated organization; only hackathon admins may deactivate |
| Output | Caller, hackathon, and active track scoped to that hackathon |
| Rejections | 401 without login; 403 for insufficient access; 404 for a missing or inactive track; 409 for a stale version or locked track |
| Tests | `tests/tracks.rs`: sponsor edit, stale version, and deactivation permission |

## PublishTrackResults

| Property | Value |
| -- | -- |
| Source | `crates/terrier-server/src/auth/policy.rs` |
| Used by | `POST /api/hackathons/{slug}/tracks/{track_id}/results` |
| Permission | Active hackathon admin or judging manager; sponsor access does not permit publication |
| Output | Caller, hackathon, and active track scoped to that hackathon |
| Rejections | 401 without login; 403 for insufficient access; 404 for a missing track; 409 for a stale version or existing publication |
| Tests | `tests/tracks.rs`: scheduled publication and public release gate |

The two result reads use `check_track_action` directly. Internal results require hackathon admin, judging management, or active sponsorship of the track's associated organization. Released winners are public only after `release_at` and while publication has not been revoked. The single-project result is always private.

## Adding an entry

Use the same fields as `OwnProfile`, including required roles and hackathon scope. For combined policies, include branch order and error handling so callers know which rule applies. Keep the entry updated when behavior changes.

## Combining policies

Combinators run existing policies in order. The policies must share an error type.

| Tool | Checks and result |
| -- | -- |
| `all!(A, B, C)` | Requires all checks; stops at the first failure. Returns `(a, (b, c))`. |
| `any!(A, B, C)` | Stops at the first success, identified by nested `Either::Left` / `Either::Right`. Returns the last error if all fail. |
| `policy!` | Tries named alternatives in order and returns the first successful variant, or the last error. |

A, B, and C represent existing policy types. They can be nested and given a name:

```rust
type Access = leash::any!(A, leash::all!(B, C));
```

This accepts A on its own, or both B and C. Use a type alias for a repeated combination. If the handler needs to distinguish alternatives, `policy!` gives them names:

```rust
leash::policy! {
    pub enum Access for AppState {
        First = A,
        Second = B,
    }
}
```

Here, `auth.data` is `Access::First(a)` or `Access::Second(b)`. With `any!(A, B)`, it would be `Either::Left(a)` or `Either::Right(b)`. These are illustrative patterns; the own-profile route only needs `Auth<OwnProfile>`.

`any!` and `policy!` continue on every error, including `Internal`. When a database failure must stop the request, write an explicit check that propagates it. Branches share request parts and do not undo changes, so checks should avoid side effects. The macros live in [Leash's `src/macros.rs`](https://git.cmu.dev/ScottyLabs/leash/src/branch/main/src/macros.rs), with examples in [`tests/integration.rs`](https://git.cmu.dev/ScottyLabs/leash/src/branch/main/tests/integration.rs).
