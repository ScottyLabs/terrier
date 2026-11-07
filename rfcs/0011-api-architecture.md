# RFC 0011: API Architecture

- **Status:** Accepted
- **Author(s):** @abeni_hyz, @amzoeee, @anish, @Azzzz, @ethanli, @evanfost, @ipae, @JacenL, @jazang, @lucaswang08, @mdn2, @sbwang, @skancher, @kritdass
- **Created:** 2026-10-07
- **Updated:** 2026-11-07

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
- Specifying a final judging algorithm
- Prize distribution, payment and tax-form workflows, and integrations with external prize-distribution systems
- Building the future hackathon setup UI
- Defining every request field before the corresponding product questions are answered

## Detailed Design

### Architecture

The API lives in `crates/terrier-server`. Axum handles HTTP requests, SeaORM accesses PostgreSQL, and `utoipa-axum` registers routes and generates the OpenAPI specification. The Svelte frontend and Capacitor mobile client consume the same API through generated TypeScript types and `openapi-fetch`.

This builds on [RFC 0001](https://git.cmu.dev/ScottyLabs/terrier/src/branch/main/rfcs/0001-core-architecture-tech-stack.md), [RFC 0004](https://git.cmu.dev/ScottyLabs/terrier/src/branch/main/rfcs/0004-documentation.md), [RFC 0005](https://git.cmu.dev/ScottyLabs/terrier/src/branch/main/rfcs/0005-saml-proxy-university-auth.md), [RFC 0006](https://git.cmu.dev/ScottyLabs/terrier/src/branch/main/rfcs/0006-observability.md), [RFC 0007](https://git.cmu.dev/ScottyLabs/terrier/src/branch/main/rfcs/0007-mobile-architecture.md), [RFC 0008](https://git.cmu.dev/ScottyLabs/terrier/src/branch/main/rfcs/0008-database-model.md), and [RFC 0009](https://git.cmu.dev/ScottyLabs/terrier/src/branch/main/rfcs/0009-slac.md).

User lifecycle requirements follow [RFC 0010](https://git.cmu.dev/ScottyLabs/terrier/src/branch/main/rfcs/0010-user-flow.md). The mobile client uses Capacitor to align with other ScottyLabs mobile applications. RFC 0007 defines the Capacitor wrapper, native integrations, and packaging requirements.

The API exposes `/api/health` for health checks, `/swagger-ui/` for interactive reference, and `/openapi.json` for client and documentation generation. These describe implemented endpoints; the domain routes below are proposed contracts, not a claim that those features have shipped.

### Route Conventions

Application API routes use the unversioned `/api` prefix. Hackathon resources use `/api/hackathons/{slug}` with plural resource names throughout. Each deployment serves one API contract; parallel major-version routes and latest-version aliases are not provided.

Clients and generated types must stay compatible with the deployment they connect to. Client compatibility checks and update behavior remain [open questions](#question-client-compatibility).

`slug` resolves a hackathon; other resource IDs are UUIDs. User profiles are global, while applications, teams, projects, tracks, evaluations, and announcements belong to a hackathon.

Use `GET` for reads, `POST` to create resources or perform explicit actions, and `PATCH` for partial updates, including lifecycle flags. Do not expose `DELETE` routes for domain resources or relationships. Use `PUT` where the caller replaces a complete resource. Ordinary CRUD paths use resource names without `create` or `edit` suffixes.

Use projects as the submission resource and evaluations as the judging resource. Prize tracks are always `tracks`; hackathon sponsor associations are always `sponsors`. Team membership lives under `members`, pending requests under `join-requests`, and judge work under track `assignments`. Reads never use `POST`, and ordinary CRUD routes never end in `/create` or `/edit`. Routes use explicit user IDs instead of a `me` alias; clients select the relevant ID and the server enforces access using the authenticated identity.

Every resource lookup must check its parent scope. A project ID from another hackathon must not be usable by changing the slug in the URL. Track, team, and event references in a request must also belong to the resolved hackathon.

### Authentication and Authorization

Keycloak owns login and identity. Support Google sign-in alongside university SSO for the initial release so applicants whose universities are outside InCommon/eduGAIN can register. Other external OAuth/OIDC providers can be configured through Keycloak. University login goes through `saml-proxy` as described in [RFC 0005](https://git.cmu.dev/ScottyLabs/terrier/src/branch/main/rfcs/0005-saml-proxy-university-auth.md); external provider sign-in is brokered through Keycloak, outside the SAML proxy. [RFC 0005](https://git.cmu.dev/ScottyLabs/terrier/src/branch/main/rfcs/0005-saml-proxy-university-auth.md)'s exclusion of non-SAML providers applies to the proxy, not to Terrier registration. Protected requests follow the [RFC 0009](https://git.cmu.dev/ScottyLabs/terrier/src/branch/main/rfcs/0009-slac.md) integration: validate the OIDC identity, upsert the Terrier user, attach `CurrentUser`, and run a SLAC policy before entering the handler.

Both sign-in paths use the same Keycloak-issued OIDC identity at the Terrier API boundary; external provider access tokens are not Terrier API credentials. Login and account onboarding use the OIDC flow. Terrier derives the caller from the validated identity and manages the application profile separately. Caller-supplied user IDs, requested roles, and the client application do not establish identity or grant privileges.

Identity uses the validated Keycloak issuer and subject, mapped to an independent Terrier user UUID. Email stays synchronized from Keycloak; display name is initialized at account creation and later Terrier edits are preserved. Terrier trusts the configured issuer without requiring `email_verified` or a separate email-verification step, consistent with RFCs 0009 and 0010. University sign-in is optional and authentication alone does not establish student eligibility. Secure linking of multiple provider identities is managed through Keycloak; the remaining account-linking policy is [an open question](#question-registration-and-privacy).

The shipped browser flow uses backend OIDC through Ricochet and opaque session cookies backed by Valkey. Sessions expire after 48 hours of inactivity; logout clears the Terrier session without ending Keycloak SSO. Unsafe authenticated requests require the session-bound `x-csrf-token` and reject foreign Origins. Frontend and API share the configured `APP_URL`; native origin, cookie, and callback handling must be defined before mobile authentication is implemented. See the [authentication guide](https://git.cmu.dev/ScottyLabs/terrier/src/branch/main/sites/docs/src/content/docs/guides/authentication.md) for current configuration and routes.

Public hackathon fields and published winners need explicitly public read routes outside the application authentication gate. Authenticated management views still require their resource policies and must not be exposed through the public response. Keep public and protected route registration distinct; public reads do not bypass authorization for mutations.

Policies live in `terrier-server`, while SLAC remains domain-free. A protected handler declares its access requirement through `Auth<P>` and uses the checked resource returned by the policy. List queries must still filter rows to the permitted scope; SLAC does not perform SQL filtering.

Each user has one primary role per hackathon: `hacker`, `organizer`, `judge`, or `sponsor`, consistent with RFC 0008. Scoped capabilities represent overlapping responsibilities without adding another primary role. For example, a sponsor can judge assigned tracks within their company, and an organizer can receive judging access. Those capabilities still require the same track and project assignment checks as any judge. Application, attendance, and reimbursement states are separate from primary roles.

SLAC policies check the relevant capability and resource scope rather than inferring every permission from the primary role. Organizer responsibilities such as application management, judging management, sponsorship management, and hackathon administration can be granted separately. The workflow permissions follow RFC 0010; their scoped-grant persistence model remains [an open question](#question-administration). Admin capabilities include application management. Ordinary organizers can read participant application details but cannot inspect drafts, make admission decisions, or read separate reimbursement evidence without application-management or admin capabilities. The one-primary-role rule is settled. Whether sponsors need different capability tiers also remains open. Team leadership is team-scoped and does not change the user's hackathon role.

Sponsor access must be checked against the relevant sponsor organization and track. Sponsor access does not grant global administration or access to every track.

Global admins are configured through deployment environment variables, as decided in [issue #21](https://git.cmu.dev/ScottyLabs/terrier/issues/21). Only global admins may create hackathons. Global administration is separate from hackathon roles, and environment configuration is the authority for granting it; `User.is_global_admin` must not provide an independent grant. `INSTANCE_ADMIN_SUBJECTS` is a JSON array of subjects under the configured Keycloak issuer, validated and loaded at startup. Configuration changes require restarting the server. Identity storage already uses issuer/subject and no longer includes the admin boolean; loading this configuration does not itself implement global-admin policies.

### Request and Response Contracts

Use typed request and response structs rather than returning database models directly. Public profiles must expose only approved public fields; private application data, resumes, internal evaluation data, and channel credentials need separate responses and access checks. JSONB fields are validated against the relevant application or submission requirements rather than accepted as arbitrary client data.

The API uses these shared response conventions:

| Status | Meaning |
| --------------------------- | ------------------------------------------------------------ |
| `200 OK` | Read or update succeeded and returns a representation |
| `201 Created` | Resource created; return its ID and representation |
| `202 Accepted` | Durable asynchronous work accepted, not delivered yet |
| `204 No Content` | Deactivation succeeded without a response body |
| `400 Bad Request` | Malformed request, invalid parameters, or unknown enum value |
| `401 Unauthorized` | Missing or invalid authentication |
| `403 Forbidden` | Authenticated caller lacks access or eligibility |
| `404 Not Found` | Requested Terrier resource does not exist in the scope |
| `409 Conflict` | Duplicate, capacity conflict, or incompatible workflow state |
| `422 Unprocessable Content` | Well-formed data fails field or schema validation |
| `429 Too Many Requests` | Rate limit exceeded |
| `500 Internal Server Error` | Unexpected server failure |

An invalid external submission link is a validation failure, not a missing Terrier resource. Whether link validation includes checking remote availability remains [an open question](#question-submission-schema).

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

Exact error fields, pagination, filtering, body limits, and rate limits are implementation details. Define them consistently in the shared API layer and document them in OpenAPI as endpoints are implemented. Unexpected failures should be logged with request context using [RFC 0006](https://git.cmu.dev/ScottyLabs/terrier/src/branch/main/rfcs/0006-observability.md)'s instrumentation and returned without exposing credentials or private data.

### Concurrent Organizer Edits

Mutable resources managed by organizers expose a server-issued revision. Updates, deactivations, and restoration actions must include the revision the organizer last read. Check that revision and apply the mutation atomically. If another mutation has changed the resource, return `409 Conflict` with a stable `stale_revision` error code and leave the resource unchanged.

The UI must explain that someone else updated the resource and let the organizer fetch and review the latest data before retrying with its current revision. Do not automatically overwrite the newer change or silently retry a stale edit. Record successful mutations in the audit history; a rejected stale request does not create a successful-change record.

### Deferred Bulk Operations

Bulk application decisions, judging assignments, and application-data export/decision import will be designed in a future RFC. That RFC will define batch routes, per-item outcomes, transaction boundaries, revision checks, retry behavior, and import validation. These workflows are not prerequisites for the initial application-management implementation. Initial contact-list exports remain in scope under RFC 0010.

### Workflow Coverage

The route tables cover the shared architecture and selected domain operations. Management access means the capability for that action under RFC 0010, not every organizer. The following RFC 0010 workflows also require contracts before their respective features are implemented; exact routes and payloads belong to that work.

| Workflow | Required behavior and access |
| --- | --- |
| Application drafts, revisions, and corrections | Applicants save drafts separately from submitted revisions; application managers/admins inspect drafts in a separate view and make recorded corrections |
| Attendance | Applicants confirm, decline, or reconfirm before the deadline; application managers/admins reopen expired offers; preserve check-in and decline history without restoring former team membership |
| Events and check-in | Organizers record event check-in for confirmed attendees; repeated scans must not duplicate attendance |
| Invitations | Authorized issuers grant only their permitted hackathon/company/track scope; recipients authenticate before redemption; define expiry, revocation, and use limits |
| Team leadership and table allocation | Leaders transfer leadership; admins recover absent leaders; table selection respects configured capacity and judging freezes |
| Project drafts and submission | Distinguish saving a draft from submitting it; confirmed, checked-in team members may edit within deadlines and locks |
| Reimbursements | Separate category-specific requests, owner status/evidence access, and application-manager/admin decisions; Finance rules and schema reconciliation precede implementation |
| Judge controls and winner selection | Judges select active tracks and take breaks; sponsors select their own track winners, judging organizers select non-sponsor winners, and publication is a separate organizer action |

### Hackathon Administration

| Method and path | Request or behavior | Access |
| -------------------------------------------------------- | --------------------------------------------------- | ------------------------------------------ |
| `POST /api/hackathons` | Create a hackathon with its settings | Global-admin access |
| `GET /api/hackathons/{slug}` | Read hackathon configuration visible to the caller | Public fields or management view |
| `PATCH /api/hackathons/{slug}` | Update hackathon settings or set `is_active: false` to deactivate it | Hackathon management |
| `POST /api/hackathons/{slug}/tracks` | Create a prize track | Management; sponsor creation access remains open |
| `PATCH /api/hackathons/{slug}/tracks/{track_id}` | Update a prize track | Management; sponsor editing access remains open |
| `POST /api/hackathons/{slug}/sponsors` | Associate a sponsor organization with the hackathon | Hackathon management |
| `PATCH /api/hackathons/{slug}/sponsors/{sponsor_org_id}` | Update the hackathon's sponsor association | Hackathon management |
| `POST /api/hackathons/{slug}/organizers` | Add an organizer by user ID | Hackathon management |
| `PATCH /api/hackathons/{slug}/organizers/{user_id}` | Set the organizer relationship to `is_active: false` | Hackathon management |

Sponsor organization data can be shared across hackathons. Updating an association must not implicitly edit the organization everywhere. Organizer and sponsor writes also need to keep their relationships and hackathon role assignments consistent. Hackathon creation requires environment-configured global-admin access. What sponsors can edit remains [an open question](#question-administration).

Completed hackathons must support a readable archive, as required by the multi-hackathon model. Completion is distinct from administrative deactivation; define archive discovery and field visibility before implementing lifecycle/list APIs. An archive must not expose private applications or evaluations.

A future hackathon setup UI should let global admins create hackathons and authorized organizers configure them without technical setup, including submission forms. This is a deferred feature; its design and implementation require follow-up. The UI must enforce the same creation and management permissions as the API.

### Organizer Recovery Controls

Organizers must be able to inspect failures and manually correct workflows within their hackathon when Terrier's automation or normal flows fail. This requirement applies across registration, teams, submissions, judging, results, and communications. Provide supported controls to correct state, override automated decisions, and retry or cancel failed operations where applicable. Each domain must define its recovery actions and permissions; the need for manual recovery is established, while the exact controls remain open.

Recovery actions must enforce hackathon scope and data integrity, preserve relevant history, and record who changed what and why. Automation must respect manual overrides until they are explicitly released under the domain's policy. Show whether a recovery action succeeded or still needs attention. Application controls cannot resolve a complete service outage; operational recovery and escalation procedures must be defined separately under [the open questions](#question-administration).

Persistent domain mutations, including deactivations and automated changes, must append an audit record in the same database transaction as the change. Record the actor or service, hackathon scope where applicable, timestamp, action, affected resource, and sufficient change history for authorized recovery. Privileged corrections and recovery actions also record a reason. Audit history is append-only: application credentials must not permit updating or deleting existing audit records. Secrets must not be copied into audit records.

Business deletion deactivates persistent domain records. Session/token cleanup and personal-data erasure require their own lifecycle policies rather than indefinite retention under this rule. Use `PATCH` on the resource or relationship to set `is_active: false`; join requests instead use an explicit `withdrawn` or `rejected` status. Preserve data and references, and do not use physical or cascading deletes. Ordinary field-edit permissions do not grant deactivation or restoration permission. Validate lifecycle transitions and dependent state in the same transaction as the flag update and audit record. Channel deactivation must stop new delivery work through that channel; handling already accepted work requires the communications policy. Ordinary reads and authorization must exclude inactive resources and relationships as appropriate. Authorized organizers and admins can inspect history and restore eligible records through explicit recovery actions that recheck permissions, uniqueness, and current workflow constraints. Restoration creates a new audit entry and must not overwrite intervening changes silently. Submitted projects have no deletion or withdrawal operation, as established in RFC 0010.

### Registration and Applications

| Method and path | Request or behavior | Access |
| ------------------------------------------------------------------- | -------------------------------------------------------- | ---------------------------- |
| `GET /api/users/{user_id}` | Read the requested user's profile fields visible to the caller | Profile owner; other visibility unresolved |
| `PATCH /api/users/{user_id}` | Update editable profile fields | Profile owner |
| `GET /api/hackathons/{slug}/users` | List approved public participant profiles | Visibility policy unresolved |
| `PUT /api/hackathons/{slug}/applications/{user_id}` | Submit application data or create a new submitted revision, preserving prior revisions | Authenticated applicant matching `user_id` |
| `GET /api/hackathons/{slug}/applications/{user_id}` | Read the specified user's application and status | Application owner; application management; organizers may read submitted participant applications |
| `GET /api/hackathons/{slug}/applications` | List applications with status and other approved filters | Application managers/admins; organizers limited to participant applications |
| `PATCH /api/hackathons/{slug}/applications/{user_id}/status` | Set an allowed application status | Application managers or admins |

The frontend selects the relevant Terrier user ID and requests `/api/users/{user_id}`, including for the signed-in user. The server derives the caller from the authenticated identity, checks profile visibility on reads, and restricts profile updates to the owner. Access to other users' profile fields remains [an open question](#question-registration-and-privacy).

Applications and their status changes are scoped to a hackathon and addressed by the applicant's `user_id`. The server checks that self-service reads and submissions target the authenticated user; application-management reads and status changes require application-management or admin capabilities. Ordinary organizers may read submitted participant applications as specified in RFC 0010, without gaining draft or admission-decision access. Managers can inspect drafts for troubleshooting through a separate, clearly labeled view. Drafts are excluded from submitted-application decision lists and cannot receive admission decisions. Public participant lists and private applicant lists use separate resources and response types.

Validate application data against `Hackathon.application_schema`. Fields such as school, major, graduation year, shirt size, profile links, and resume information are configurable per hackathon. The server owns acceptance decisions and privileged fields. Repeating the same application `PUT` must not create another submitted revision or duplicate downstream effects; define revision preconditions and retry handling before implementing this write.

Application statuses are `pending`, `accepted`, `rejected`, and `waitlisted`. Rejection does not imply deleting the user. When notification delivery is implemented, status changes trigger the agreed flow after the database change commits. Initial communications use contact exports; a status change must not claim that a message was sent. Application edit rules follow RFC 0010, including immutable submitted revisions and read-only rejected or waitlisted applications. Applicants who declined attendance may reconfirm before the confirmation deadline without organizer approval. Preserve the decline history and expose a clear reconfirmation indicator to application managers; expired offers still require reopening. For a previously checked-in attendee, reconfirmation restores eligibility using the existing check-in, but does not restore team membership. Profile visibility and the notification contract remain open; admission capacity allocation is outside this RFC.

### Teams

| Method and path | Request or behavior | Access |
| ---------------------------------------------------------------------------- | ----------------------------------------------------------------------- | -------------------------------------------------------- |
| `POST /api/hackathons/{slug}/teams` | Create a team from a name; add the caller | Confirmed attendee without an active team in this hackathon |
| `GET /api/hackathons/{slug}/teams` | List visible teams; optionally filter teams seeking members | Eligible authenticated users |
| `GET /api/hackathons/{slug}/teams/{team_id}` | Read team details, permitted member fields, and submission information | Confirmed attendees see discovery fields; members, permitted judges, and management see their authorized detail views |
| `PATCH /api/hackathons/{slug}/teams/{team_id}` | Update editable team fields; management may set `is_active: false` to dismiss the team, subject to the unresolved dismissal policy | Team leader or hackathon management |
| `POST /api/hackathons/{slug}/teams/{team_id}/join-requests` | Request membership as the authenticated caller | Confirmed attendee without an active team in this hackathon |
| `POST /api/hackathons/{slug}/teams/{team_id}/join-requests/{user_id}/accept` | Accept a pending request | Team leader |
| `PATCH /api/hackathons/{slug}/teams/{team_id}/join-requests/{user_id}` | Set request status to `withdrawn` or `rejected` | Request owner may withdraw; team leader may reject |
| `PATCH /api/hackathons/{slug}/teams/{team_id}/members/{user_id}` | Set membership to `is_active: false` to leave or remove a member | Self may leave; only hackathon management may remove another member |

Only confirmed attendees can create or join teams, with at most one active team per hackathon. Membership in a different hackathon is independent. These eligibility rules follow RFC 0010.

Creation and membership acceptance must be transactional. Two simultaneous requests must not put one user on two active teams or overfill a team. Capacity comes from `Hackathon.max_team_size`, not a client-selected limit.

The membership flow is request-only: a confirmed attendee requests to join an existing team, appears in its pending requests, and becomes an active member when the team leader accepts the request. Each user may have only one pending request per hackathon, enforced transactionally across teams. The requester must withdraw it or receive a rejection before requesting another team. Acceptance joins the user directly without another confirmation. The requester can withdraw a pending request, and the leader can reject it. Team invitations, invite codes, and open joining are outside this flow.

The creator becomes team leader and approves or rejects requests. The leader may edit team details or transfer leadership. When the leader leaves or declines attendance, the remaining member with the earliest membership start time becomes leader, with user ID as a stable tie-breaker. Members may leave voluntarily; only organizers can involuntarily remove a member. Members contact a hackathon admin to recover an unresponsive leader using existing management controls; no dedicated request or verification workflow is required. The last member leaving deactivates the team and preserves its history.

Declining attendance removes active membership and pending requests atomically. Pending requests do not reserve capacity, and approval rechecks eligibility, capacity, and the one-team constraint. Store requests separately from active membership; the representation must be reconciled with RFC 0008. Creating a team also requires no pending join request; the caller must withdraw it first.

Initial discovery supports browsing and searching teams. Automated matching and suggested teammates are deferred, as established in RFC 0010. Confirmed attendees can inspect team name, description, member display names, and remaining capacity before requesting membership. This discovery view excludes private application and contact details; broader profile visibility remains open.

### Project Submissions

| Method and path | Request or behavior | Access |
| ----------------------------------------------------- | ------------------------------------------------------------ | -------------------------------------------------------- |
| `POST /api/hackathons/{slug}/projects` | Create a submission with `event_id`, `data`, and `track_ids` | Confirmed, checked-in member of an active team |
| `GET /api/hackathons/{slug}/projects/{project_id}` | Read the project, team, and submitted tracks | Owning team or organizers for drafts; permitted assigned judges and authorized track sponsors for submitted projects |
| `GET /api/hackathons/{slug}/teams/{team_id}/projects` | List a team's submission, returning at most one project | Same resource access policy |
| `GET /api/hackathons/{slug}/projects` | List submissions; optionally filter by track | Hackathon management; judge-scoped lists if approved |
| `PATCH /api/hackathons/{slug}/projects/{project_id}` | Update allowed submission fields or track selections | Confirmed, checked-in team member before the submission deadline; management overrides unresolved |

The server derives the caller and their active team from authenticated identity and membership. One confirmed, checked-in member is sufficient to submit for the team; other members do not need to have checked in. `event_id` and every selected track must belong to the requested hackathon. Project creation and track associations are committed together. Organizers can read project drafts within their hackathon, as can the owning team. Judges and sponsors can read only submitted projects within their authorized assignments or tracks unless they separately hold organizer capabilities.

Each hackathon configures its own submission fields and requirements rather than using a fixed form shared by every hackathon. Fields may include repository and video links. The frontend renders the configured form, and the server validates submissions against that hackathon's schema. The schema format, supported field types, and handling of schema changes after submissions exist remain [open questions](#question-submission-schema). [RFC 0008](https://git.cmu.dev/ScottyLabs/terrier/src/branch/main/rfcs/0008-database-model.md) provides `Project.data` but no dedicated submission schema, draft state, or lock field. Those features need explicit schema decisions.

Each team may create one submission per hackathon, consistent with [RFC 0008](https://git.cmu.dev/ScottyLabs/terrier/src/branch/main/rfcs/0008-database-model.md)'s one-project-per-team relationship. The team can edit that submission before the hackathon submission deadline, including its selected prize tracks; entering multiple tracks does not create additional submissions. Enforce uniqueness in the database and return `409 Conflict` for attempts to create a second submission. Project drafts and editing before the deadline are restricted to confirmed, checked-in team members under RFC 0010. Submitted projects cannot be removed or withdrawn. Organizer overrides remain [an open question](#question-submission-lifecycle); [the submission-schema questions](#question-submission-schema) cover configurable fields and validation.

The server enforces the submission deadline for creation, edits, and track changes in the same transaction as the mutation, using server time. At or after the deadline, ordinary submission writes are rejected. The same deadline applies to the submission across its selected tracks. Organizer overrides remain [open questions](#question-submission-lifecycle).

### Judging and Results

Judge-to-track access (`JudgeTrackAssign`), judge-to-project assignment, and a submitted evaluation are separate concepts. [RFC 0008](https://git.cmu.dev/ScottyLabs/terrier/src/branch/main/rfcs/0008-database-model.md) can represent a pending evaluation, but whether that also represents an assignment is unresolved.

| Method and path | Request or behavior | Access |
| --------------------------------------------------------------------------------- | ------------------------------------------------------------- | ------------------------------------------------------- |
| `GET /api/hackathons/{slug}/tracks/{track_id}/judges/{judge_id}/assignments` | Read the specified judge's current project assignments | Authenticated judge matching `judge_id` and assigned to this track |
| `POST /api/hackathons/{slug}/tracks/{track_id}/assignments` | Manually assign a judge to a project using `judge_id` and `project_id` | Hackathon management |
| `POST /api/hackathons/{slug}/tracks/{track_id}/projects/{project_id}/evaluations` | Submit the caller's evaluation payload | Judge assigned to the track and project |
| `GET /api/hackathons/{slug}/tracks/{track_id}/projects/{project_id}/evaluations` | Read internal evaluations | Hackathon management |
| `GET /api/hackathons/{slug}/tracks/{track_id}/results` | Read internal judging results or the published winners list, subject to access | Management, permitted sponsors, or public after release |
| `POST /api/hackathons/{slug}/tracks/{track_id}/results` | Publish a manually curated winners list or schedule its release | Hackathon management |
| `GET /api/hackathons/{slug}/tracks/{track_id}/results/{project_id}` | Read internal results for one project | Management or permitted sponsors within their tracks |

The judge submitting an evaluation is always the authenticated caller, not an arbitrary `judge_id` in the body. Assignment creation must verify the judge's track access and the project's track entry. Repeated requests must not produce duplicate active assignments or unintentionally duplicate evaluations.

Judge-to-project assignments are generated automatically by a routing algorithm. Organizers must always be able to override routing decisions by assigning or reassigning judges within their hackathon. Routing must respect organizer overrides rather than silently replacing them on its next run. The override API, persistence, and handling of in-progress or completed evaluations remain [open questions](#question-judging).

Routing balances project coverage, additional evaluations for stronger projects, and walking distance based on table proximity and layout. The algorithm, priority between these objectives, and behavior when a project has no judge remain [open questions](#question-judging). Tracks use the `scoring` or `expo` judging method from [RFC 0008](https://git.cmu.dev/ScottyLabs/terrier/src/branch/main/rfcs/0008-database-model.md); evaluation payloads, completion rules, skips, reassignment, and fairness requirements are resolved with the assignment policy.

Sponsors select final winners for their own tracks without organizer approval. Organizers control publication; internal evaluations and rankings do not automatically determine or publish winners.

Internal judging results and the public winners list are distinct response views. Internal scores, rankings, raw evaluations, and judge identities remain restricted to authorized management and sponsors within their tracks. Public results contain only the winners list manually curated by organizers, incorporating sponsor selections for sponsor tracks; finishing judging never publishes results automatically. Organizers can publish results immediately or schedule their release for a specified time. Scheduled results remain private until that time; the server enforces the publication gate for both release modes. Sponsor access before release is limited to authorized tracks. Publishing winners does not expose raw evaluations, judge identities, rankings, or every team's scores. Broader public or team-specific score sharing is deferred. [The results questions](#question-results) define the release policy and persistence model.

### Communications

Organizers need one place to manage hackathon communications. Integration priority is email, then text, then Discord, then Slack. Initial communications use organizer contact exports as established in RFC 0010; provider integration and delivery mechanics remain [open questions](#question-communications).

TODO: Define how external integrations work end to end, including provider adapters, account connection and authorization, credential storage, recipient or channel mapping, workflow triggers, outgoing delivery, incoming webhooks or status updates, and organizer recovery controls. The routes below describe the proposed domain interface; they do not settle provider-specific setup or integration behavior.

| Method and path | Request or behavior | Access |
| ------------------------------------------------------------ | ------------------------------------------------------------------------------ | ------------------------------------------------- |
| `POST /api/hackathons/{slug}/announcements` | Create an announcement with title, body, audience, and optional scheduled time | Management or sponsor within its authorized track |
| `GET /api/hackathons/{slug}/announcements` | List announcements relevant to the caller; management can inspect its scope | Authenticated recipient or management |
| `GET /api/hackathons/{slug}/announcements/{announcement_id}` | Read one announcement permitted for the caller | Same audience policy |
| `POST /api/hackathons/{slug}/channels` | Configure a supported channel using validated configuration | Hackathon management |
| `GET /api/hackathons/{slug}/channels` | List channel configuration with secrets omitted | Hackathon management |
| `PATCH /api/hackathons/{slug}/channels/{channel_id}` | Set the configured channel to `is_active: false` | Hackathon management |

Audiences may select hackathon roles or a prize track. Sponsors cannot expand their audience beyond their authorized track. A channel identifier supplied by a caller must resolve inside the hackathon.

Persist the announcement and accepted delivery work before reporting success. The asynchronous contract returns `202 Accepted` with an announcement ID and delivery state; it does not return "sent" before delivery. Delivery must survive server restarts and work across instances, consistent with [RFC 0001](https://git.cmu.dev/ScottyLabs/terrier/src/branch/main/rfcs/0001-core-architecture-tech-stack.md). Provider failures, retries, and partial delivery must remain visible to organizers.

Store deployment secrets in environment configuration according to [RFC 0001](https://git.cmu.dev/ScottyLabs/terrier/src/branch/main/rfcs/0001-core-architecture-tech-stack.md). Channel responses and logs must not expose credentials. Runtime channel configuration needs a reviewed way to reference or manage secrets. Scheduling, consent, cancellation, audience timing, delivery tracking, and retry behavior remain [open questions](#question-communications).

### Data Model Integration

[RFC 0008](https://git.cmu.dev/ScottyLabs/terrier/src/branch/main/rfcs/0008-database-model.md) remains the database reference and must be updated as approved decisions in this RFC change its model. Reconcile its entity diagram, enum reference, relationships, and constraints with the required workflows before implementing dependent features. Keep those updates aligned with SeaORM migrations and regenerated entities; unresolved choices in this draft do not establish a final schema.

| Feature | Required schema decision |
| --------------------------------- | -------------------------------------------------------------------------------------------------------- |
| Deactivation and audit history | Preserve resources and relationships with inactive/deleted state; add append-only mutation history and checked restoration |
| Identity mapping and admin access | Issuer/subject identity and environment admin configuration are implemented; update RFC 0008 to reflect them and define the remaining admin policies |
| Team requests and leadership | Define request persistence and the leader relationship using RFC 0010 lifecycle rules; automated matching is deferred |
| Application revisions and reimbursements | Preserve submitted application revisions and category-specific reimbursement requests under RFC 0010; reconcile with issue #29 |
| Scoped capabilities | Keep one primary role per user per hackathon; define separate capability grants and their resource scope |
| One active team per hackathon | Define a database-enforced constraint or transactional enforcement strategy |
| Submission lifecycle | Define project draft/submitted state, submission schema, and deadline enforcement; submitted projects cannot be withdrawn |
| Submission uniqueness | Enforce one submission per team per hackathon |
| Judge-to-project assignment | Decide whether pending evaluations represent assignments or need a separate entity |
| Results publication | No dedicated results snapshot or release gate |
| Communications | No announcement, channel, schedule, or delivery record entities |
| Hackathon sponsor association | Current sponsor organization and track relationships do not define a separate hackathon sponsor resource |

### Offline Connectivity

Terrier should remain usable during a Wi-Fi outage and automatically synchronize locally saved work as soon as connectivity returns and the client can run. Judges must be able to view previously downloaded assignments and project details and save evaluations locally while offline. Preserve pending evaluations across normal reloads and application restarts, then send them automatically when reconnected. Show distinct states for saved on this device, awaiting sync, confirmed by the server, and failed or needs attention; local persistence is not server acceptance.

While offline, show a persistent, non-blocking banner explaining that work is saved on this device and that new assignments and updates cannot be retrieved until connectivity returns. Show when cached assignments and project details were last refreshed. Place the banner in or near the root layout and share connectivity and synchronization state through the API client layer so the behavior is consistent across workflows. If local saving fails, report the failure explicitly rather than claiming work was saved.

This behavior is required for web and mobile, but the implementation remains a TODO. Define application and data caching, durable local storage, queued delivery, retry deduplication, conflict resolution, and recovery from storage or authentication failures. Offline use depends on previously available data; it cannot obtain new server assignments while disconnected. Which additional workflows work offline, and what synchronization is possible while a browser tab or app is closed or suspended, remain [open questions](#question-real-time-and-retry-behavior).

### Documentation and Integration

Group routes by domain and register them through `utoipa-axum`. Document request bodies, response schemas, access requirements, validation failures, and workflow conflicts in OpenAPI. Publish the deployment's API specification at `/openapi.json`. Generate client types from that specification, and regenerate affected client types and documentation specifications from the running server when contracts change.

HTTP remains the authoritative read/write interface. WebSocket notifications may tell clients that assignments, results, or announcements changed, consistent with [RFC 0001](https://git.cmu.dev/ScottyLabs/terrier/src/branch/main/rfcs/0001-core-architecture-tech-stack.md), but their event contract remains [an open question](#question-real-time-and-retry-behavior). Clients must be able to fetch current state after reconnecting.

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

The shared conventions and RFC 0010 workflows are the basis for implementation. Remaining questions block only their dependent features; record the responsible domain team and follow-up before starting that work. Judging, communications, reimbursements, and native/offline details may be deferred without blocking the application phase.

1. <a id="question-client-compatibility"></a> **Client compatibility and updates.** API routes are unversioned and each deployment serves one contract. How do clients detect incompatibility and obtain an update? How are cached browser sessions and installed mobile clients handled when the server contract changes?
1. <a id="question-administration"></a> **Administration.** Each user has one primary hackathon role plus scoped capabilities. How are RFC 0010 capabilities persisted for overlapping responsibilities? Do sponsors need different capability tiers, and which organization or track actions would each tier allow? Ordinary organizer access does not imply hackathon administration. Global admins are configured through environment variables and alone may create hackathons. What can sponsors edit? Business deletion means deactivation with preserved history. What are archive discovery and visibility rules? Organizer recovery controls are required across workflows: which actions and permissions should each domain expose, and what recovery or escalation process applies during a complete service outage?
1. <a id="question-registration-and-privacy"></a> **Registration and privacy.** External OAuth/OIDC sign-in is required alongside university SSO. Google ships initially alongside university SSO; other external providers are configurable through Keycloak. Issuer/subject mapping and backend OIDC sessions follow RFC 0009; no separate email-verification gate is required. How should Keycloak securely link external and university identities to one account? Authentication does not establish student eligibility. Profile visibility remains open: which fields may be public, whether publication is opt-in, and who may browse profiles? Application access follows RFC 0010. Sponsor-specific team-member visibility also remains open in RFC 0010. Application edit rules follow RFC 0010. Which status changes send notifications? Admission capacity allocation is outside this RFC.
1. <a id="question-teams"></a> **Teams.** Lifecycle rules follow RFC 0010: confirmed attendees form teams, the creator leads, the oldest remaining member succeeds a departing leader, only organizers remove members involuntarily, and empty teams deactivate. Automated matching is deferred. One pending join request per user per hackathon is required. Discovery fields follow RFC 0010; broader profile visibility remains separate. Absent-leader recovery uses hackathon admin assistance through existing controls. Request persistence and concurrent enforcement are implementation details to reconcile with RFC 0008.
1. <a id="question-submission-lifecycle"></a> **Submission lifecycle.** Each team has one submission per hackathon and may edit it before the submission deadline. Project drafts are supported and confirmed, checked-in team members may edit before the deadline. Submitted projects cannot be removed or withdrawn. How is the hackathon submission deadline configured, and how should organizer recovery controls handle deadline exceptions and submission corrections?
1. <a id="question-submission-schema"></a> **Submission schema.** Each hackathon defines its own submission fields and requirements. What schema format and field types should be supported, how is the configuration stored and updated, and how do changes affect existing submissions? How do projects select tracks? Does URL validation only validate format, or also inspect remote repositories/videos? What happens when an external service is unavailable?
1. <a id="question-judging"></a> **Judging.** A routing algorithm assigns judges to projects, with organizer override control. When does routing run, and how do judges receive the next assignment? How are organizer overrides recorded, retained across routing runs, and explicitly released? What API supports reassignment, and how does it handle evaluations already in progress or completed? Do pending evaluations double as assignments? What are scoring and expo payloads, repeat-evaluation rules, skip/reassignment behavior, and tie handling? How do minimum coverage, stronger-project sampling, conflicts of interest, and table proximity affect assignment fairness?
1. <a id="question-results"></a> **Results.** Both immediate organizer publication and scheduled release are supported. How are scheduled releases changed or canceled? Public release contains only a manually curated winners list; broader score sharing is deferred. What is the exact internal-results access matrix for management and sponsors? How should organizers correct or revoke publication during recovery, and should results be stored as a snapshot?
1. <a id="question-communications"></a> **Communications and integrations.** Channel priority is email, text, Discord, then Slack, with contact exports for the initial rollout. Which providers should implement these channels, and how should their integrations work end to end? How do organizers connect accounts, grant permissions, and disconnect or replace an integration? What adapter interfaces, credential storage, recipient/channel mappings, and workflow triggers are needed? How do outgoing API calls and incoming webhooks or status updates interact with delivery records, and how are callbacks authenticated and deduplicated? Who configures integrations, and which setup, testing, failure inspection, and manual recovery controls do organizers need? Are scheduling, cancellation, and delivery-status reads required? Are recipients resolved when scheduled or when sent? What consent, retry, duplicate-send, and partial-failure rules apply? How do application and team notifications use this system?
1. <a id="question-real-time-and-retry-behavior"></a> **Offline connectivity, real-time, and retry behavior.** Offline access to downloaded judging data, locally saved evaluations, visible sync status, a persistent non-blocking offline banner, cached-data refresh times, and automatic synchronization on reconnect are required. How should the web and mobile implementations cache application resources and data, persist pending work, and handle reloads, storage failures, and expired authentication? What works while clients are closed or suspended? Which workflows beyond judging should work offline? How should retries avoid duplicate evaluations and handle changed assignments or judging deadlines? Which changes need WebSocket events? How are subscriptions authorized and state recovered on reconnect? Which writes need idempotency keys or other retry guarantees beyond database uniqueness?

## Implementation Phases

1. **RFC review.** Open a draft PR using the [standard RFC process](https://git.cmu.dev/ScottyLabs/terrier/src/branch/main/rfcs/README.md). Confirm the shared conventions and user-flow requirements, and record the remaining domain questions as explicit follow-ups before dependent implementation. Shared-contract details are specified during implementation.
1. **Reconcile earlier RFCs.** Review affected accepted RFCs and amend them to reflect approved decisions here, linking the changes back to this RFC. Prioritize [RFC 0008](https://git.cmu.dev/ScottyLabs/terrier/src/branch/main/rfcs/0008-database-model.md) for identity/admin modeling, pending users and team leadership, configurable submission forms and deadlines, submission uniqueness, routing and organizer overrides, results publication, and communications. Review [RFC 0001](https://git.cmu.dev/ScottyLabs/terrier/src/branch/main/rfcs/0001-core-architecture-tech-stack.md), [RFC 0005](https://git.cmu.dev/ScottyLabs/terrier/src/branch/main/rfcs/0005-saml-proxy-university-auth.md), [RFC 0007](https://git.cmu.dev/ScottyLabs/terrier/src/branch/main/rfcs/0007-mobile-architecture.md), and [RFC 0009](https://git.cmu.dev/ScottyLabs/terrier/src/branch/main/rfcs/0009-slac.md) for affected architecture, authentication boundaries, offline web/mobile behavior, and authorization. Record unresolved items as follow-ups and complete required design amendments before dependent implementation.
1. **API foundation.** Backend Keycloak OIDC, Valkey sessions, `CurrentUser`, shared errors, OpenAPI generation, and the own-profile SLAC example are implemented. Reuse this foundation; add public/protected route boundaries, global-admin and domain policies, and any error variants needed by these workflows. The proposed `stale_revision` code requires extending the current generic conflict response before clients depend on it.
1. **Organizer safety.** Add deactivation, append-only mutation audit records, checked restoration, and stale-revision protection. Verify that failed mutations leave no partial changes. Bulk transaction and retry contracts belong in the future bulk-operations RFC.
1. **Administration and registration.** Add hackathon, organizer, sponsor, track, profile, and application APIs. Complete the required schema changes and private/public response separation.
1. **Teams and submissions.** Implement the approved membership and submission lifecycle. Verify concurrent joins, capacity, duplicate submissions, cross-hackathon references, and edits racing with judging locks.
1. **Judging and results.** Implement automatic routing with organizer overrides, the approved assignment model and evaluation contracts, then internal results and publication gates. Verify that routing respects overrides, access stays scoped to the judge or sponsor, and results are not released early.
1. **Communications and integration.** Implement the approved persistent announcement and delivery model, initial channel adapters, and agreed real-time events. Verify restart recovery, visible partial failures, and organizer controls for retrying or canceling failed deliveries without unintended duplicate sends.
1. **Offline support.** Implement the approved web/mobile caching and synchronization design, including downloaded judging data, durable pending evaluations, visible sync status, and safe replay.
1. **Documentation and release validation.** Regenerate API clients and reference documentation throughout implementation. Exercise the complete application-to-results flow on web and mobile, including registration without university SSO, access failures, safe retries, offline judging, persistence across reloads, synchronization on reconnect, and organizer recovery from workflow failures.
