# RFC 0010: API Architecture

- **Status:** Draft
- **Author(s):** @abeni_hyz, @amzoeee, @Azzzz, @ethanli, @ipae, @JacenL, @jazang, @lucaswang08, @mdn2, @sbwang, @skancher, @kritdass
- **Created:** 2026-10-07
- **Updated:** 2026-10-07

## Overview

This RFC establishes the HTTP API architecture for Terrier, covering hackathon administration, registration, teams, submissions, judging, results, and communications. The API uses consistent resource routes, hackathon-scoped access control, and shared request and response contracts built on Axum, Keycloak, SeaORM, and SLAC.

## Motivation

Terrier needs to support multiple hackathons and serve participants, organizers, judges, and sponsors through the same web and mobile API. Each hackathon has its own applications, teams, submission requirements, prize tracks, and permissions. Clear resource boundaries and consistent access rules are essential for keeping these workflows independent as the platform grows.

A shared API contract lets frontend and backend contributors build features without making separate decisions about routing, validation, or errors. Generated OpenAPI types keep clients aligned with the backend, while explicit authorization and transactional state changes protect application data and preserve consistency during registration, submissions, and judging.

## Goals

- Establish consistent resource scoping, request handling, and responses
- Define the API surface and access requirements for each domain
- Give organizers manual controls to recover from workflow failures and override automation
- Build on Keycloak, SLAC, SeaORM, and generated OpenAPI types
- Identify database changes required by these workflows
- Resolve outstanding decisions through the normal RFC review process

## Non-Goals

- Replacing the technology stack or authentication architecture
- Redesigning the entire database model
- Choosing communication providers or a background job implementation
- Specifying a final judging algorithm or prize payment workflow
- Building the future hackathon setup UI
- Defining every request field before the corresponding product questions are answered

## Detailed Design

### Architecture

The API lives in `crates/terrier-server`. Axum handles HTTP requests, SeaORM accesses PostgreSQL, and `utoipa-axum` registers routes and generates the OpenAPI specification. The Svelte frontend and Tauri mobile client consume the same API through generated TypeScript types and `openapi-fetch`.

This builds on [RFC 0001](https://git.cmu.dev/ScottyLabs/terrier/src/branch/main/rfcs/0001-core-architecture-tech-stack.md), [RFC 0004](https://git.cmu.dev/ScottyLabs/terrier/src/branch/main/rfcs/0004-documentation.md), [RFC 0005](https://git.cmu.dev/ScottyLabs/terrier/src/branch/main/rfcs/0005-saml-proxy-university-auth.md), [RFC 0006](https://git.cmu.dev/ScottyLabs/terrier/src/branch/main/rfcs/0006-observability.md), [RFC 0007](https://git.cmu.dev/ScottyLabs/terrier/src/branch/main/rfcs/0007-mobile-architecture.md), [RFC 0008](https://git.cmu.dev/ScottyLabs/terrier/src/branch/main/rfcs/0008-database-model.md), and [RFC 0009](https://git.cmu.dev/ScottyLabs/terrier/src/branch/main/rfcs/0009-slac.md).

The API exposes `/api/health` for health checks independently of application API versions, Swagger UI for interactive reference, and `/api/v{major}/openapi.json` for client and documentation generation. `/openapi.json` aliases the latest stable version's specification.

### Route Conventions

Canonical application API routes use `/api/v{major}`, starting with `/api/v1`. Hackathon resources use `/api/v1/hackathons/{slug}` with plural resource names throughout. Route examples in this RFC use v1.

Only breaking API contract changes increment the major version, such as `/api/v1` to `/api/v2`. Backward-compatible additions and fixes stay within the existing major version; minor and patch versions do not appear in URLs.

The unversioned `/api` prefix remains an alias for the latest stable major version: `/api/hackathons/{slug}` initially serves the same contract as `/api/v1/hackathons/{slug}` and switches to v2 when v2 becomes the latest stable version. The alias does not provide compatibility across major versions. Terrier's web and mobile clients pin an explicit major version, including generated client types, so existing installations and browser sessions remain compatible during deployments.

Previous major versions remain available during a documented migration window. Share business logic where contracts permit, while preserving each supported version's request, response, and behavioral contracts. Database changes must accommodate all supported versions. The support window and retirement policy remain [open question 1](#question-api-version-retirement).

`slug` resolves a hackathon; other resource IDs are UUIDs. User profiles are global, while applications, teams, projects, tracks, evaluations, and announcements belong to a hackathon.

Use `GET` for reads, `POST` to create resources or perform explicit actions, `PATCH` for partial updates, and `DELETE` for removal or withdrawal. Use `PUT` where the caller replaces a complete resource. Ordinary CRUD paths use resource names without `create` or `edit` suffixes.

Use projects as the submission resource and evaluations as the judging resource. Prize tracks are always `tracks`; hackathon sponsor associations are always `sponsors`. Team membership lives under `members`, pending requests under `join-requests`, and judge work under track `assignments`. Reads never use `POST`, and ordinary CRUD routes never end in `/create` or `/edit`. Routes use explicit user IDs instead of a `me` alias; clients select the relevant ID and the server enforces access using the authenticated identity.

Every resource lookup must check its parent scope. A project ID from another hackathon must not be usable by changing the slug in the URL. Track, team, and event references in a request must also belong to the resolved hackathon.

### Authentication and Authorization

Keycloak owns login and identity. Support external OAuth/OIDC provider sign-in alongside university SSO so applicants whose universities are outside InCommon/eduGAIN can register. University login goes through `saml-proxy` as described in [RFC 0005](https://git.cmu.dev/ScottyLabs/terrier/src/branch/main/rfcs/0005-saml-proxy-university-auth.md); external provider sign-in is brokered through Keycloak, outside the SAML proxy. [RFC 0005](https://git.cmu.dev/ScottyLabs/terrier/src/branch/main/rfcs/0005-saml-proxy-university-auth.md)'s exclusion of non-SAML providers applies to the proxy, not to Terrier registration. Protected requests follow the [RFC 0009](https://git.cmu.dev/ScottyLabs/terrier/src/branch/main/rfcs/0009-slac.md) integration: validate the OIDC identity, upsert the Terrier user, attach `CurrentUser`, and run a SLAC policy before entering the handler.

Both sign-in paths use the same Keycloak-issued OIDC identity at the Terrier API boundary; external provider access tokens are not Terrier API credentials. Login and account onboarding use the OIDC flow. Terrier derives the caller from the validated identity and manages the application profile separately. Caller-supplied user IDs, requested roles, and the client application do not establish identity or grant privileges.

Account registration and student-eligibility verification are separate. External provider sign-in does not prove student status. Applicants from unsupported universities must have a path to apply and obtain eligibility verification without university SSO. Provider selection, account linking, verification evidence, and whether applications can be submitted before verification completes remain [open question 4](#question-registration-and-privacy).

Policies live in `terrier-server`, while SLAC remains domain-free. A protected handler declares its access requirement through `Auth<P>` and uses the checked resource returned by the policy. List queries must still filter rows to the permitted scope; SLAC does not perform SQL filtering.

The proposed hackathon roles are `hacker`, `organizer`, `judge`, and `sponsor`. Whether sponsors need two permission tiers, and which organization or track actions each tier would allow, remains [open question 3](#question-administration). [RFC 0008](https://git.cmu.dev/ScottyLabs/terrier/src/branch/main/rfcs/0008-database-model.md) currently defines `hacker`, `organizer`, `judge`, and `sponsor`, with one role per user per hackathon; any change must be reconciled with that model. Acceptance is an application status, not another role. Hackathon management is a permission checked by a policy; its mapping to organizer access, multiple-role support, and the need for additional roles such as mentor remain open. The provisional team model has a designated leader with team-scoped membership approval permissions; leadership lifecycle and other permissions remain [open question 5](#question-teams).

Sponsor access must be checked against the relevant sponsor organization and track. Sponsor access does not grant global administration or access to every track.

The provisional approach is to configure global admins through deployment environment configuration, such as an environment file. Global administration is separate from hackathon roles. How configured admins map to authenticated identities, how access is revoked, and whether this replaces or coexists with [RFC 0008](https://git.cmu.dev/ScottyLabs/terrier/src/branch/main/rfcs/0008-database-model.md)'s `User.is_global_admin` remain [open question 3](#question-administration); the RFC does not yet establish two independent sources of admin access.

### Request and Response Contracts

Use typed request and response structs rather than returning database models directly. Public profiles must expose only approved public fields; private application data, resumes, internal evaluation data, and channel credentials need separate responses and access checks. JSONB fields are validated against the relevant application or submission requirements rather than accepted as arbitrary client data.

The API uses these shared response conventions:

| Status | Meaning |
| --------------------------- | ------------------------------------------------------------ |
| `200 OK` | Read or update succeeded and returns a representation |
| `201 Created` | Resource created; return its ID and representation |
| `202 Accepted` | Durable asynchronous work accepted, not delivered yet |
| `204 No Content` | Removal succeeded without a response body |
| `400 Bad Request` | Malformed request, invalid parameters, or unknown enum value |
| `401 Unauthorized` | Missing or invalid authentication |
| `403 Forbidden` | Authenticated caller lacks access or eligibility |
| `404 Not Found` | Requested Terrier resource does not exist in the scope |
| `409 Conflict` | Duplicate, capacity conflict, or incompatible workflow state |
| `422 Unprocessable Content` | Well-formed data fails field or schema validation |
| `429 Too Many Requests` | Rate limit exceeded |
| `500 Internal Server Error` | Unexpected server failure |

An invalid external submission link is a validation failure, not a missing Terrier resource. Whether link validation includes checking remote availability is [open question 7](#question-submission-schema).

Use one `ApiError` response shape across domains. A validation response has this form:

```json
{
  "code": "validation_failed",
  "message": "Submission data is invalid",
  "fields": {
    "data.github_url": "Expected a valid repository URL"
  }
}
```

Exact error fields, list pagination, and rate limits are [open question 2](#question-shared-contracts). Unexpected failures should be logged with request context using [RFC 0006](https://git.cmu.dev/ScottyLabs/terrier/src/branch/main/rfcs/0006-observability.md)'s instrumentation and returned without exposing credentials or private data.

### Hackathon Administration

| Method and path | Request or behavior | Access |
| -------------------------------------------------------- | --------------------------------------------------- | ------------------------------------------ |
| `POST /api/v1/hackathons` | Create a hackathon with its settings | Global-admin access |
| `GET /api/v1/hackathons/{slug}` | Read hackathon configuration visible to the caller | Public fields or management view |
| `PATCH /api/v1/hackathons/{slug}` | Update hackathon settings | Hackathon management |
| `DELETE /api/v1/hackathons/{slug}` | Remove or deactivate a hackathon | Hackathon management; semantics unresolved |
| `POST /api/v1/hackathons/{slug}/tracks` | Create a prize track | Management or authorized sponsor |
| `PATCH /api/v1/hackathons/{slug}/tracks/{track_id}` | Update a prize track | Management or owning sponsor |
| `POST /api/v1/hackathons/{slug}/sponsors` | Associate a sponsor organization with the hackathon | Hackathon management |
| `PATCH /api/v1/hackathons/{slug}/sponsors/{sponsor_org_id}` | Update the hackathon's sponsor association | Hackathon management |
| `POST /api/v1/hackathons/{slug}/organizers` | Add an organizer by user ID | Hackathon management |
| `DELETE /api/v1/hackathons/{slug}/organizers/{user_id}` | Remove an organizer | Hackathon management |

Sponsor organization data can be shared across hackathons. Updating an association must not implicitly edit the organization everywhere. Organizer and sponsor writes also need to keep their relationships and hackathon role assignments consistent. The global-admin restriction on hackathon creation is provisional; who can create hackathons, what sponsors can edit, and deletion behavior are [open question 3](#question-administration).

A future hackathon setup UI should let people create and configure their own hackathons without technical setup, including submission forms. This is a deferred feature; its design and implementation require follow-up. It does not settle who may create hackathons, which remains [open question 3](#question-administration).

### Organizer Recovery Controls

Organizers must be able to inspect failures and manually correct workflows within their hackathon when Terrier's automation or normal flows fail. This requirement applies across registration, teams, submissions, judging, results, and communications. Provide supported controls to correct state, override automated decisions, and retry or cancel failed operations where applicable. Each domain must define its recovery actions and permissions; the need for manual recovery is established, while the exact controls remain open.

Recovery actions must enforce hackathon scope and data integrity, preserve relevant history, and record who changed what and why. Automation must respect manual overrides until they are explicitly released under the domain's policy. Show whether a recovery action succeeded or still needs attention. Application controls cannot resolve a complete service outage; operational recovery and escalation procedures must be defined separately under [open question 3](#question-administration).

### Registration and Applications

| Method and path | Request or behavior | Access |
| ------------------------------------------------------------------- | -------------------------------------------------------- | ---------------------------- |
| `GET /api/v1/users/{user_id}` | Read the requested user's profile fields visible to the caller | Profile owner; other visibility unresolved |
| `PATCH /api/v1/users/{user_id}` | Update editable profile fields | Profile owner |
| `GET /api/v1/hackathons/{slug}/users` | List approved public participant profiles | Visibility policy unresolved |
| `PUT /api/v1/hackathons/{slug}/applications/{user_id}` | Submit or replace the specified user's application data | Authenticated applicant matching `user_id` |
| `GET /api/v1/hackathons/{slug}/applications/{user_id}` | Read the specified user's application and status | Application owner matching `user_id` |
| `GET /api/v1/hackathons/{slug}/applications` | List applications with status and other approved filters | Hackathon management |
| `PATCH /api/v1/hackathons/{slug}/applications/{user_id}/status` | Set an allowed application status | Hackathon management |

The frontend selects the relevant Terrier user ID and requests `/api/v1/users/{user_id}`, including for the signed-in user. The server derives the caller from the authenticated identity, checks profile visibility on reads, and restricts profile updates to the owner. Access to other users' profile fields remains [open question 4](#question-registration-and-privacy).

Applications and their status changes are scoped to a hackathon and addressed by the applicant's `user_id`. The server checks that applicant reads and submissions target the authenticated user; status changes require hackathon management access. Public participant lists and private applicant lists use separate resources and response types.

Validate application data against `Hackathon.application_schema`. Fields such as school, major, graduation year, shirt size, profile links, and resume information are configurable per hackathon. The server owns acceptance decisions and privileged fields.

Application statuses are `pending`, `accepted`, `rejected`, and `waitlisted`. Rejection does not imply deleting the user. Status changes should trigger the agreed notification flow after the database change commits. Profile visibility, application editing deadlines, admission capacity, and the notification contract need review under [open question 4](#question-registration-and-privacy).

### Teams

| Method and path | Request or behavior | Access |
| ---------------------------------------------------------------------------- | ----------------------------------------------------------------------- | -------------------------------------------------------- |
| `POST /api/v1/hackathons/{slug}/teams` | Create a team from a name; add the caller | Accepted hacker without an active team in this hackathon |
| `GET /api/v1/hackathons/{slug}/teams` | List visible teams; optionally filter teams seeking members | Eligible authenticated users |
| `GET /api/v1/hackathons/{slug}/teams/{team_id}` | Read team details, public members, and permitted submission information | Members, permitted judges, or management |
| `PATCH /api/v1/hackathons/{slug}/teams/{team_id}` | Update editable team fields | Team management policy unresolved |
| `POST /api/v1/hackathons/{slug}/teams/{team_id}/join-requests` | Request membership as the authenticated caller | Accepted hacker without an active team in this hackathon |
| `POST /api/v1/hackathons/{slug}/teams/{team_id}/join-requests/{user_id}/accept` | Accept a pending request | Team leader |
| `DELETE /api/v1/hackathons/{slug}/teams/{team_id}/join-requests/{user_id}` | Withdraw or reject a request | Request owner may withdraw; team leader may reject |
| `DELETE /api/v1/hackathons/{slug}/teams/{team_id}/members/{user_id}` | Leave or remove a member | Self, permitted team manager, or hackathon management |
| `DELETE /api/v1/hackathons/{slug}/teams/{team_id}` | Dismiss the team | Management policy unresolved |

Only accepted hackers can create or join teams, with at most one active team per hackathon. Membership in a different hackathon is independent. These constraints remain subject to [open question 5](#question-teams).

Creation and membership acceptance must be transactional. Two simultaneous requests must not put one user on two active teams or overfill a team. Capacity comes from `Hackathon.max_team_size`, not a client-selected limit.

The provisional membership flow is request-only: a participant requests to join an existing team, appears in its `pending_users`, and becomes an active member when the team leader accepts the request. The requester can withdraw a pending request, and the leader can reject it. Team invitations, invite codes, and open joining are outside this provisional flow.

Each team provisionally has one designated leader who approves or rejects join requests. How the initial leader is selected, how leadership transfers, and what happens when the leader leaves remain [open question 5](#question-teams). Editing team details, removing members, dismissing teams, and organizer overrides need separate permission decisions.

Pending users remain distinct from active members. The database representation of `pending_users`, whether pending requests reserve capacity, and when they expire or clear remain open. [RFC 0008](https://git.cmu.dev/ScottyLabs/terrier/src/branch/main/rfcs/0008-database-model.md)'s `TeamMember.status` values are `active`, `invited`, and `removed`; they need reconciliation with the pending-request flow and leader model.

Matching and discovery remain unresolved under [open question 5](#question-teams). They could help participants find teams seeking members or prospective teammates, but joining an existing team follows the request-and-leader-approval flow. Decide whether discovery uses searchable profiles and teams, suggested matches, an opt-in matching flow, or a combination before defining matching endpoints.

### Project Submissions

| Method and path | Request or behavior | Access |
| ----------------------------------------------------- | ------------------------------------------------------------ | -------------------------------------------------------- |
| `POST /api/v1/hackathons/{slug}/projects` | Create a submission with `event_id`, `data`, and `track_ids` | Accepted hacker on an active team |
| `GET /api/v1/hackathons/{slug}/projects/{project_id}` | Read the project, team, and submitted tracks | Owning team, permitted assigned judges, or management |
| `GET /api/v1/hackathons/{slug}/teams/{team_id}/projects` | List a team's submission, returning at most one project | Same resource access policy |
| `GET /api/v1/hackathons/{slug}/projects` | List submissions; optionally filter by track | Hackathon management; judge-scoped lists if approved |
| `PATCH /api/v1/hackathons/{slug}/projects/{project_id}` | Update allowed submission fields or track selections | Permitted team member before the submission deadline; management overrides unresolved |
| `DELETE /api/v1/hackathons/{slug}/projects/{project_id}` | Withdraw a submission without destroying judging history | Team leader or management; permission and timing unresolved |

The server derives the caller from authentication. It can derive the team from membership if the one-team rule is adopted; otherwise the request must identify a team and prove membership. `event_id` and every selected track must belong to the requested hackathon. Project creation and track associations are committed together.

Each hackathon configures its own submission fields and requirements rather than using a fixed form shared by every hackathon. Fields may include repository and video links. The frontend renders the configured form, and the server validates submissions against that hackathon's schema. The schema format, supported field types, and handling of schema changes after submissions exist remain [open question 7](#question-submission-schema). [RFC 0008](https://git.cmu.dev/ScottyLabs/terrier/src/branch/main/rfcs/0008-database-model.md) provides `Project.data` but no dedicated submission schema, draft state, withdrawal state, or lock field. Those features need explicit schema decisions.

Each team may create one submission per hackathon, consistent with [RFC 0008](https://git.cmu.dev/ScottyLabs/terrier/src/branch/main/rfcs/0008-database-model.md)'s one-project-per-team relationship. The team can edit that submission before the hackathon submission deadline, including its selected prize tracks; entering multiple tracks does not create additional submissions. Enforce uniqueness in the database and return `409 Conflict` for attempts to create a second submission. Drafts, withdrawal, edit permissions within the team, and organizer overrides remain [open question 6](#question-submission-lifecycle); [open question 7](#question-submission-schema) covers configurable fields and validation.

The server enforces the submission deadline for creation, edits, and track changes in the same transaction as the mutation, using server time. At or after the deadline, ordinary submission writes are rejected. The same deadline applies to the submission across its selected tracks. Withdrawal timing and organizer overrides remain [open question 6](#question-submission-lifecycle).

### Judging and Results

Judge-to-track access (`JudgeTrackAssign`), judge-to-project assignment, and a submitted evaluation are separate concepts. [RFC 0008](https://git.cmu.dev/ScottyLabs/terrier/src/branch/main/rfcs/0008-database-model.md) can represent a pending evaluation, but whether that also represents an assignment is unresolved.

| Method and path | Request or behavior | Access |
| --------------------------------------------------------------------------------- | ------------------------------------------------------------- | ------------------------------------------------------- |
| `GET /api/v1/hackathons/{slug}/tracks/{track_id}/judges/{judge_id}/assignments` | Read the specified judge's current project assignments | Authenticated judge matching `judge_id` and assigned to this track |
| `POST /api/v1/hackathons/{slug}/tracks/{track_id}/assignments` | Manually assign a judge to a project using `judge_id` and `project_id` | Hackathon management |
| `POST /api/v1/hackathons/{slug}/tracks/{track_id}/projects/{project_id}/evaluations` | Submit the caller's evaluation payload | Judge assigned to the track and project |
| `GET /api/v1/hackathons/{slug}/tracks/{track_id}/projects/{project_id}/evaluations` | Read internal evaluations | Hackathon management |
| `GET /api/v1/hackathons/{slug}/tracks/{track_id}/results` | Read results permitted by the publication gate | Management, permitted sponsors, or public after release |
| `POST /api/v1/hackathons/{slug}/tracks/{track_id}/results` | Publish results or schedule their release; payload unresolved | Hackathon management |
| `GET /api/v1/hackathons/{slug}/tracks/{track_id}/results/{project_id}` | Read permitted results for one project | Same publication policy |

The judge submitting an evaluation is always the authenticated caller, not an arbitrary `judge_id` in the body. Assignment creation must verify the judge's track access and the project's track entry. Repeated requests must not produce duplicate active assignments or unintentionally duplicate evaluations.

Judge-to-project assignments are generated automatically by a routing algorithm. Organizers must always be able to override routing decisions by assigning or reassigning judges within their hackathon. Routing must respect organizer overrides rather than silently replacing them on its next run. The override API, persistence, and handling of in-progress or completed evaluations remain [open question 8](#question-judging).

Routing balances project coverage, additional evaluations for stronger projects, and walking distance based on table proximity and layout. The algorithm, priority between these objectives, and behavior when a project has no judge are [open question 8](#question-judging). Tracks use the `scoring` or `expo` judging method from [RFC 0008](https://git.cmu.dev/ScottyLabs/terrier/src/branch/main/rfcs/0008-database-model.md); evaluation payloads, completion rules, skips, reassignment, and fairness requirements are resolved with the assignment policy.

The results resource serves internal and published views under the same publication policy. Organizers can publish results immediately or schedule their release for a specified time. Scheduled results remain private until that time; the server enforces the publication gate for both release modes. Sponsor access before release is limited to authorized tracks. Publishing winners does not automatically make raw evaluations, judge identities, or every team's scores public. [Open question 9](#question-results) defines the release policy and persistence model.

### Communications

Organizers need one place to manage hackathon communications. Email, text, Discord, and Slack are candidate channels; channel selection and the integration design remain [open question 10](#question-communications).

TODO: Define how external integrations work end to end, including provider adapters, account connection and authorization, credential storage, recipient or channel mapping, workflow triggers, outgoing delivery, incoming webhooks or status updates, and organizer recovery controls. The routes below describe the proposed domain interface; they do not settle provider-specific setup or integration behavior.

| Method and path | Request or behavior | Access |
| ------------------------------------------------------------ | ------------------------------------------------------------------------------ | ------------------------------------------------- |
| `POST /api/v1/hackathons/{slug}/announcements` | Create an announcement with title, body, audience, and optional scheduled time | Management or sponsor within its authorized track |
| `GET /api/v1/hackathons/{slug}/announcements` | List announcements relevant to the caller; management can inspect its scope | Authenticated recipient or management |
| `GET /api/v1/hackathons/{slug}/announcements/{announcement_id}` | Read one announcement permitted for the caller | Same audience policy |
| `POST /api/v1/hackathons/{slug}/channels` | Configure a supported channel using validated configuration | Hackathon management |
| `GET /api/v1/hackathons/{slug}/channels` | List channel configuration with secrets omitted | Hackathon management |
| `DELETE /api/v1/hackathons/{slug}/channels/{channel_id}` | Remove a configured channel | Hackathon management |

Audiences may select hackathon roles or a prize track. Sponsors cannot expand their audience beyond their authorized track. A channel identifier supplied by a caller must resolve inside the hackathon.

Persist the announcement and accepted delivery work before reporting success. The asynchronous contract returns `202 Accepted` with an announcement ID and delivery state; it does not return "sent" before delivery. Delivery must survive server restarts and work across instances, consistent with [RFC 0001](https://git.cmu.dev/ScottyLabs/terrier/src/branch/main/rfcs/0001-core-architecture-tech-stack.md). Provider failures, retries, and partial delivery must remain visible to organizers.

Store deployment secrets in environment configuration according to [RFC 0001](https://git.cmu.dev/ScottyLabs/terrier/src/branch/main/rfcs/0001-core-architecture-tech-stack.md). Channel responses and logs must not expose credentials. Runtime channel configuration needs a reviewed way to reference or manage secrets. Scheduling, consent, cancellation, audience timing, delivery tracking, and retry behavior are [open question 10](#question-communications).

### Data Model Integration

[RFC 0008](https://git.cmu.dev/ScottyLabs/terrier/src/branch/main/rfcs/0008-database-model.md) remains the database reference and must be updated as approved decisions in this RFC change its model. Reconcile its entity diagram, enum reference, relationships, and constraints with the required workflows before implementing dependent features. Keep those updates aligned with SeaORM migrations and regenerated entities; unresolved choices in this draft do not establish a final schema.

| Feature | Required schema decision |
| --------------------------------- | -------------------------------------------------------------------------------------------------------- |
| Identity mapping and admin access | Define the OIDC-to-user mapping and environment-admin precedence |
| Team matching and leadership | Define `pending_users` persistence and the designated leader relationship; matching and leadership lifecycle remain open |
| One active team per hackathon | Define a database-enforced constraint or transactional enforcement strategy |
| Submission lifecycle | No project status, withdrawal state, lock, or submission schema |
| Submission uniqueness | Enforce one submission per team per hackathon |
| Judge-to-project assignment | Decide whether pending evaluations represent assignments or need a separate entity |
| Results publication | No dedicated results snapshot or release gate |
| Communications | No announcement, channel, schedule, or delivery record entities |
| Hackathon sponsor association | Current sponsor organization and track relationships do not define a separate hackathon sponsor resource |

### Offline Connectivity

Terrier should remain usable during a Wi-Fi outage and automatically synchronize locally saved work as soon as connectivity returns and the client can run. Judges must be able to view previously downloaded assignments and project details and save evaluations locally while offline. Preserve pending evaluations across normal reloads and application restarts, then send them automatically when reconnected. Show distinct states for saved on this device, awaiting sync, confirmed by the server, and failed or needs attention; local persistence is not server acceptance.

This behavior is required for web and mobile, but the implementation remains a TODO. Define application and data caching, durable local storage, queued delivery, retry deduplication, conflict resolution, and recovery from storage or authentication failures. Offline use depends on previously available data; it cannot obtain new server assignments while disconnected. Which additional workflows work offline, and what synchronization is possible while a browser tab or app is closed or suspended, remain [open question 11](#question-real-time-and-retry-behavior).

### Documentation and Integration

Group routes by domain and register them through `utoipa-axum`. Document request bodies, response schemas, access requirements, validation failures, and workflow conflicts in OpenAPI. Publish a separate OpenAPI specification at `/api/v{major}/openapi.json` for each supported major version. Generate client types from the pinned version's specification, and regenerate affected client types and documentation specifications from the running server when contracts change. Document `/api` and `/openapi.json` as latest-stable aliases whose contracts can change across major versions.

HTTP remains the authoritative read/write interface. WebSocket notifications may tell clients that assignments, results, or announcements changed, consistent with [RFC 0001](https://git.cmu.dev/ScottyLabs/terrier/src/branch/main/rfcs/0001-core-architecture-tech-stack.md), but their event contract is [open question 11](#question-real-time-and-retry-behavior). Clients must be able to fetch current state after reconnecting.

## Alternatives Considered

### Separate conventions per domain

Separate domain conventions let route names, identity handling, roles, and statuses diverge. Shared conventions give clients a consistent contract and let contributors reuse validation and access patterns.

### Global user routes for hackathon applications

Putting applications and acceptance status under global user routes hides the hackathon scope. Separate application resources make it clear that someone can be accepted to one hackathon and pending at another.

### Local password registration

Managing passwords in Terrier would duplicate Keycloak's responsibility under [RFC 0005](https://git.cmu.dev/ScottyLabs/terrier/src/branch/main/rfcs/0005-saml-proxy-university-auth.md) and [RFC 0009](https://git.cmu.dev/ScottyLabs/terrier/src/branch/main/rfcs/0009-slac.md) and require a second account lifecycle.

### Permissions inside every handler

[RFC 0009](https://git.cmu.dev/ScottyLabs/terrier/src/branch/main/rfcs/0009-slac.md) already establishes SLAC policies in handler signatures. Repeating checks in each handler would lose the shared access contract. Resource-specific filtering and transactional state checks are still required.

### Synchronous multichannel announcement delivery

Sending every provider request before responding makes the API depend on the slowest provider and leaves partial delivery difficult to retry. Durable asynchronous delivery separates acceptance from delivery and records provider failures. The queue and provider design remain open.

## Open Questions

These are decisions for the RFC PR. Revise the detailed design as they are answered before acceptance; a deliberately deferred feature should have its scope and follow-up recorded.

1. <a id="question-api-version-retirement"></a> **API version retirement.** How long do we support a previous major version, how do we announce deprecation and retirement, and what upgrade path do unsupported mobile clients receive? Major-version routes, pinned web and mobile clients, and the latest-stable `/api` alias are established above.
1. <a id="question-shared-contracts"></a> **Shared contracts.** What exact error fields, pagination, filtering, body limits, and rate limits should each list and write endpoint use?
1. <a id="question-administration"></a> **Administration.** The proposed roles are `hacker`, `organizer`, `judge`, and `sponsor`. Do sponsors need two permission tiers, and which organization or track actions would each tier allow? Can users hold multiple hackathon roles, and are additional roles such as mentor needed? Who may create hackathons, and does organizer access imply hackathon-admin access? Should global admins remain in environment configuration, move to `User.is_global_admin`, or use an explicit bootstrap arrangement? How are configured identities validated and access changes or revocations applied? What can sponsors edit, and is hackathon deletion deactivation or permanent removal? Organizer recovery controls are required across workflows: which actions and permissions should each domain expose, how are changes recorded, and what recovery or escalation process applies during a complete service outage?
1. <a id="question-registration-and-privacy"></a> **Registration and privacy.** External OAuth/OIDC sign-in is required alongside university SSO. Which providers should be enabled? What is the exact OIDC onboarding contract and stable identity mapping, and how are external and university identities securely linked to the same account? How do applicants from unsupported universities verify student eligibility, what evidence or organizer review is required, and can applications be submitted while verification is pending? Which profile fields are public, who may browse them, and who may read private application data? When can applications be edited, how is admission capacity enforced, and which status changes send notifications?
1. <a id="question-teams"></a> **Teams.** The provisional choice is participant-initiated join requests, `pending_users`, and one team leader who approves or rejects requests. Matching and discovery still need an Events decision: should participants browse people, search teams seeking members, receive suggested matches, or use a dedicated opt-in matching flow? What profile information, preferences, and visibility controls are needed? Confirm accepted-only creation/joining and one active team per hackathon. Does the creator become leader, how is leadership transferred, and what happens when the leader leaves? Who may edit team details, remove members, or dismiss a team? How should pending users be stored and shown, and do requests reserve capacity? Can users have several pending requests, and when do these expire or clear after acceptance elsewhere? What happens when approvals race, eligibility changes, or the last member leaves? Which team recovery and override actions should organizers have?
1. <a id="question-submission-lifecycle"></a> **Submission lifecycle.** Each team has one submission per hackathon and may edit it before the submission deadline. Are drafts supported? Which team members may edit or withdraw? Is withdrawal reversible, and when is it allowed? How is the hackathon submission deadline configured, and how should organizer recovery controls handle deadline exceptions and submission corrections?
1. <a id="question-submission-schema"></a> **Submission schema.** Each hackathon defines its own submission fields and requirements. What schema format and field types should be supported, how is the configuration stored and updated, and how do changes affect existing submissions? How do projects select tracks? Does URL validation only validate format, or also inspect remote repositories/videos? What happens when an external service is unavailable?
1. <a id="question-judging"></a> **Judging.** A routing algorithm assigns judges to projects, with organizer override control. When does routing run, and how do judges receive the next assignment? How are organizer overrides recorded, retained across routing runs, and explicitly released? What API supports reassignment, and how does it handle evaluations already in progress or completed? Do pending evaluations double as assignments? What are scoring and expo payloads, repeat-evaluation rules, skip/reassignment behavior, and tie handling? How do minimum coverage, stronger-project sampling, conflicts of interest, and table proximity affect assignment fairness?
1. <a id="question-results"></a> **Results.** Both immediate organizer publication and scheduled release are supported. How are scheduled releases changed or canceled? Who can see results before release? Are we publishing winners, rankings, aggregate scores, or raw evaluations? How should organizers correct or revoke publication during recovery, and should results be stored as a snapshot?
1. <a id="question-communications"></a> **Communications and integrations.** Which channels and providers ship first, and how should their integrations work end to end? How do organizers connect accounts, grant permissions, and disconnect or replace an integration? What adapter interfaces, credential storage, recipient/channel mappings, and workflow triggers are needed? How do outgoing API calls and incoming webhooks or status updates interact with delivery records, and how are callbacks authenticated and deduplicated? Who configures integrations, and which setup, testing, failure inspection, and manual recovery controls do organizers need? Are scheduling, cancellation, and delivery-status reads required? Are recipients resolved when scheduled or when sent? What consent, retry, duplicate-send, and partial-failure rules apply? How do application and team notifications use this system?
1. <a id="question-real-time-and-retry-behavior"></a> **Offline connectivity, real-time, and retry behavior.** Offline access to downloaded judging data, locally saved evaluations, visible sync status, and automatic synchronization on reconnect are required. How should the web and mobile implementations cache application resources and data, persist pending work, and handle reloads, storage failures, and expired authentication? What works while clients are closed or suspended? Which workflows beyond judging should work offline? How should retries avoid duplicate evaluations and handle changed assignments or judging deadlines? Which changes need WebSocket events? How are subscriptions authorized and state recovered on reconnect? Which writes need idempotency keys or other retry guarantees beyond database uniqueness?
1. <a id="question-prize-distribution"></a> **Prize distribution.** The integration with Chihuahua remains open. Rust-to-Python FFI is under consideration, but no integration mechanism has been selected. Should Terrier hand published winners to Chihuahua through FFI, an API, or an export, and what data and failure-handling contract would that require? Chihuahua handles winner details and financial forms; the handoff contract remains to be defined. Keep payment and tax-form handling outside this RFC until that boundary is agreed.

## Implementation Phases

1. **RFC review.** Open a draft PR using the [standard RFC process](https://git.cmu.dev/ScottyLabs/terrier/src/branch/main/rfcs/README.md). Resolve the shared contract and product questions, revise this document, and record explicit deferrals before acceptance.
1. **Reconcile earlier RFCs.** Review affected accepted RFCs and amend them to reflect approved decisions here, linking the changes back to this RFC. Prioritize [RFC 0008](https://git.cmu.dev/ScottyLabs/terrier/src/branch/main/rfcs/0008-database-model.md) for identity/admin modeling, pending users and team leadership, configurable submission forms and deadlines, submission uniqueness, routing and organizer overrides, results publication, and communications. Review [RFC 0001](https://git.cmu.dev/ScottyLabs/terrier/src/branch/main/rfcs/0001-core-architecture-tech-stack.md), [RFC 0005](https://git.cmu.dev/ScottyLabs/terrier/src/branch/main/rfcs/0005-saml-proxy-university-auth.md), [RFC 0007](https://git.cmu.dev/ScottyLabs/terrier/src/branch/main/rfcs/0007-mobile-architecture.md), and [RFC 0009](https://git.cmu.dev/ScottyLabs/terrier/src/branch/main/rfcs/0009-slac.md) for affected architecture, authentication boundaries, offline web/mobile behavior, and authorization. Record unresolved items as follow-ups and complete required design amendments before dependent implementation.
1. **API foundation.** Integrate university SSO and external OAuth/OIDC sign-in through Keycloak, `CurrentUser`, SLAC policies, scoped lookups, shared errors, and OpenAPI generation. Implement one protected route end-to-end as the reference for other domains.
1. **Administration and registration.** Add hackathon, organizer, sponsor, track, profile, and application APIs. Complete the required schema changes and private/public response separation.
1. **Teams and submissions.** Implement the approved membership and submission lifecycle. Verify concurrent joins, capacity, duplicate submissions, cross-hackathon references, and edits racing with judging locks.
1. **Judging and results.** Implement automatic routing with organizer overrides, the approved assignment model and evaluation contracts, then internal results and publication gates. Verify that routing respects overrides, access stays scoped to the judge or sponsor, and results are not released early.
1. **Communications and integration.** Implement the approved persistent announcement and delivery model, initial channel adapters, and agreed real-time events. Verify restart recovery, visible partial failures, and organizer controls for retrying or canceling failed deliveries without unintended duplicate sends. Add a Chihuahua handoff only if separately specified.
1. **Offline support.** Implement the approved web/mobile caching and synchronization design, including downloaded judging data, durable pending evaluations, visible sync status, and safe replay.
1. **Documentation and release validation.** Regenerate API clients and reference documentation throughout implementation. Exercise the complete application-to-results flow on web and mobile, including registration without university SSO, access failures, safe retries, offline judging, persistence across reloads, synchronization on reconnect, and organizer recovery from workflow failures.
