# RFC 0010: User Flow

- **Status:** Draft
- **Author(s):** @amzoeee, @anish, @ethanli, @evanfost, @ipae, @JacenL, @mdn2, @sbwang, @skancher, @kritdass
- **Created:** 2026-04-19
- **Updated:** 2026-11-04

## Overview

This proposal lays out the full path through the app taken by all users, and what should be accessible to them at a given stage.
The approved access requirements will be enforced through SLAC, as described in [RFC 0009](./0009-slac.md). Lifecycle labels describe user states and capabilities; they do not prescribe database roles.

## Motivation

The design and scoping of what is available to users at different stages is incredibly important.
It serves as a fundamental basis for both the functionality of the app and the clear and proper disclosure of the apps functionality as relevant.

## Goals

- Define the lifecycle of users coming from different starting points
- Determine what functionality (and, correspondingly, access) should be available to users at different stages in their lifecycle
- Lay out how users transition between states
- Provide a comprehensive feature reference for each role so the tech team can verify completeness during implementation

## Non-Goals

- This RFC does not intend to prescribe a specific permissioning structure.
  It does use a conceptual model reminiscent of role-based access control (RBAC),
  but could be implemented into the authentication system in another way.
- This RFC will not provide any code more in depth than, potentially, pseudo-code.
  Many implementations are possible that would conform to the specification set forth here,
  so it will be the job of a future RFC to set forth the actual plan for a concrete implementation.
- This RFC does not cover the in-app messaging/notification system. That will be addressed in a separate RFC.
- This RFC does not cover the detailed mechanics of the judging system (pairwise comparison, deliberation, etc.). That will be addressed in a separate judging RFC.
- This RFC does not cover the `sponsor-manager` role, which is deferred to a future RFC.

## Detailed Design

### Custom links

To simplify the distribution of permissions to non-hackers (organizers, judges, sponsors),
this RFC proposes custom invitation links as the method for doing this.

These could be emailed directly (which allows some email verification, if we think sponsors or judges will need a non-Google, non-edu sign-in options),
or generated via links.

Whatever the medium, these should be able to be created by:

- `admin`s
- `judge-organizer`s
- `sponsor-organizer`s

Any role below without an explicit explanation of how it is obtained should be assumed to be added through this method,
or some equivalent method by which organizers manually distribute proper permissions.

As an implementation note, users may not exist in the database upon clicking a link,
so their creation will need to be handled by the system in the process.

### "Roles"

The following labels describe a non-exclusive set of lifecycle states and capabilities that define what a user can do or see at a given point in time. An implementation may use the four main roles (`hacker`, `organizer`, `judge`, and `sponsor`) together with application, attendance, and reimbursement statuses and scoped permissions. Specialized organizer labels describe responsibilities whose permissions must be preserved in that mapping. A user may have overlapping responsibilities; no single primary role should discard the access required for another responsibility.

In general, roles with different prefixes should be thought of as additive permissions,
so an `applicant-submitted` who becomes a `participant` should continue to have the `applicant-submitted`-related permissions,
such as viewing their application and editing fields while permitted by its status and deadlines.
States with the same prefix are mutually exclusive for the same record. Reimbursement states apply per request, so one person can have requests in different states.
This is done partially for not having to re-list every permission,
but would also be useful (if these roles become a concept in the final system) for statistics on participation counts.

______________________________________________________________________

#### `applicant-draft`

This represents a user who is submitting an application. This role should be given to anyone registering through the main website.
Once submitted, they should lose the `applicant-draft` role and move to `applicant-submitted`, since they no longer can "submit" an application
(their only option is to edit).

**Features:**

- View and edit all application form fields
- Save application drafts (partial completion persisted across sessions)
- Submit the completed application
- Optionally request reimbursement as part of the application (which grants `reimbursement-applied` upon submission; see below)

______________________________________________________________________

#### `applicant-submitted`

This represents a potential participant who has submitted an application.
This role is given once an `applicant-draft` submits their application.
If their application is accepted, they will get the additional role `attendee-potential`.

**Features:**

- View submitted application
- Edit application fields that are flagged as editable post-submission (e.g., display name, dietary restrictions, shirt size)[^1]
- View application status (submitted, accepted, rejected, waitlisted)
- View reimbursement request status, if applicable (see `reimbursement-applied`)

> **Note:** The application form system should support a per-field flag indicating whether a field is editable after submission.
> For changes to non-editable fields, applicants must contact the organizing team directly,
> and an `application-manager` can make the edit on their behalf.

\[^1\]: When an application is updated, it should be made clear to `application-manager`s,
perhaps through a separate section of the dashboard, and preferably with a diff if possible.

______________________________________________________________________

#### `applicant-rejected`

A rejected applicant can view their submitted application, decision, and submission history, but cannot edit the application. Rejection preserves the account and application for later review. This is an application status checked by policy, not an additional database role.

#### `applicant-waitlisted`

A waitlisted applicant can view their application and waitlist status. Their application is read-only until an application manager reopens it or changes the decision. Waitlisting does not permit team formation. Acceptance moves the applicant to `attendee-potential` with an explicit confirmation deadline.

______________________________________________________________________

#### `reimbursement-applied`

This role is granted when a submitted application includes a reimbursement request with details filled out.
Requests are separate records attached to an applicant and hackathon. Each request identifies a category, such as travel or hotel. Multiple categories may be requested without implying approval of every category.
Applicants can withdraw a pending request. Withdrawal preserves its history; it does not delete the request or a prior decision.

**Features:**

- View reimbursement request status (pending, approved, denied, withdrawn)

Approval records the approved amount and currency for that request. Request status is independent of application acceptance and attendance confirmation. Declining attendance flags approved requests for organizer review rather than silently canceling an approval or payment.

#### `reimbursement-denied`

Applicants can read the request and denial but cannot edit it. An application manager may reopen it with a recorded reason. Pending, approved, denied, and withdrawn are request states, not hackathon roles. Receipts and expense amounts belong to a dedicated reimbursement form; they are never public profile fields. Payment processing is outside this RFC.

______________________________________________________________________

#### `reimbursement-accepted`

This role is granted by an `application-manager` (or `admin`) when a reimbursement request is approved.

**Features:**

- Everything from `reimbursement-applied`
- Fill out the reimbursement details form: upload receipts and enter expense amounts

______________________________________________________________________

#### `attendee-potential`

This represents a potential participant whose application has been accepted.
They should be able to confirm their attendance, in which case they switch (read: lose `attendee-potential`) to `attendee-confirmed`,
or decline their attendance, in which case they switch to `attendee-declined`.

**Features:**

- Confirm attendance (transitions to `attendee-confirmed`)
- Decline attendance (transitions to `attendee-declined`)

______________________________________________________________________

#### `attendee-confirmed`

This represents a user who has accepted their spot.

At this stage, there should be a very clear message that they need to check in in-person (get their QR code scanned)
at an event to get access to submitting their project.
This should, of course, be worded in a way such that it's also applicable before the event ever starts,
since almost all will reach this stage before the beginning of the event.

**Features:**

- View event check-in QR code
- View the schedule of events
- View hacking time countdown
- View resources (Discord server link, Hacker Guide)
- Create a new team (automatically added as a member and designated leader)
- Browse and search open teams (by team name or member names)
- View team details (description, members list)
- Request to join a team (if confirmed, without a current team, and if the team is not full; the pending-request limit remains open)
- Withdraw a pending join request
- Accept or reject join requests from others (if the team leader)
- Share a team link or QR code that opens the request-to-join page
- Edit their team's name and description (team leader only)
- Leave their team
- Change attendance status to `attendee-declined`

______________________________________________________________________

#### `attendee-declined`

A user who has been accepted but declined to come.
The dashboard should show the declined status and explain that organizers must reopen the attendance offer before the attendee can confirm again.

**Features:**

- View declined status message
- Ask organizers to reopen the attendance offer; confirmation requires organizer approval

______________________________________________________________________

#### `application-manager`

This role will have access to the full list of applications, with the ability to accept or reject those applications.

**Features:**

- View submitted applications, with search and filtering
  - Lists include submitted applications only. Draft contents are private to the applicant; troubleshooting access requires a separate, explicitly approved support flow.
- View individual application details
- Accept, reject, or waitlist submitted applications
- Edit application details on behalf of applicants (for changes to non-editable fields)
- View a section highlighting recently updated applications (preferably with diffs)
- View all reimbursement requests
- Approve, deny, or reopen individual reimbursement requests, specifying an approved amount and preserving decision history

______________________________________________________________________

#### `participant`

A user who has checked in to the event. They now have access to project submission for their team.

This role should be given to an `attendee-confirmed` who checks in to *any* event.
This simplifies the complexity of having to deal with people who arrived late and didn't get checked in to the main check-in session,
since they're now able to attend talks and be logged correctly.

**Features:**

- All team operations from `attendee-confirmed` (create, request to join, search, share links, leave, etc.)[^2].
  Probably simplest to just receive these transitively, since a `participant` should always also be an `attendee-confirmed`.
- Create a project submission for their team, including:
  - Project name
  - Project description
  - Repository URL
  - Zipped file upload (optional)
  - Presentation slides URL
  - Video URL (optional)
  - Prize track selection
- Edit project submission details and prize track selections before the submission deadline; judging-related restrictions belong in the judging RFC
- Select a table number after submitting a project (manual entry or QR code scan; one team per table)
- Edit table number selection (until frozen by a `judge-organizer`)

\[^2\]: Team operations remain accessible after check-in.
Member removal and recovery follow the team membership rules below.

______________________________________________________________________

#### `organizer`

An organizer involved in the event but without full administrative access.

**Features:**

- View participant details, including:
  - Any details included in their application
  - Their current team, with a link to view that team's details
- View team details, including:
  - Members, with links to view individual details
  - Project submission (both status and information)
  - Table number
- View track winner info before it is marked as "published"
- Check people into events by scanning their QR code or by manually searching the person (any `attendee-confirmed`, regardless of current `participant` status)
- View the schedule of events (clicking on a schedule event provides access to the check-in interface for that event)
- Role-specific resource links

______________________________________________________________________

#### `judge`

A judge will be assigned prize tracks as part of the custom link they authenticate by.

**Features:**

- View assigned prize tracks
- Select which of their assigned tracks to actively judge (UI should default to all selected, with a clear indication when not all are selected, such as a "Judge Selected" vs "Judge All" toggle)
- Judge assigned projects and submit evaluations (judging mechanics defined in a future judging RFC)
- Take notes on projects during judging
- "Take a break": triggers a prompt warning that their current team assignment may change, then removes them from the active judging rotation temporarily
- Re-enter judging (after a break): gets assigned a new team (which may be the same as before if the system decides) and returns to the rotation
- Participate in final judging deliberations (see a future judging RFC)
- View the schedule of events
- Role-specific resource links

______________________________________________________________________

#### `judge-organizer`

This is an organizer in charge of managing judges.
They should be able to judge (if they want to) with all the functionality that judges have (like "taking breaks").

**Features:**

- All `judge` functionality (judging, breaks, notes, deliberation, track selection)
- Edit judge track assignments
- Edit prize track details (descriptions, judging criteria, event-attendance restrictions for eligibility)
- Edit submission information, including table numbers
- Create judge invitation links, pre-configured with the tracks those using the invitation will be assigned to
- View a list of all judges and their assignments
- View judging results (per-track summaries with clear metric descriptions, easy access to project slides/video/repo)
- View final deliberation info (see a future judging RFC)
- Select winners for non-sponsor tracks; sponsors select winners for their own tracks
- Control when winner details are published
- Freeze project submission editing (per track) before the expo
- Start judging on a per-track basis (with a "Select All" option, since most tracks will start simultaneously)
- View the schedule of events
- Role-specific resource links

______________________________________________________________________

#### `sponsor`

A sponsor associated with a company and its prize tracks.

**Features:**

- View a list of projects (with details) that have applied to the company's prize track(s), including:
  - Project submission details (name, description, repo, slides, video)
  - Team member information (see Open Questions for fields and visibility)
- View judging results and winners for the company's prize tracks (per-track summaries, easy access to slides/video/repo, clear metric descriptions)
- Activate judging access for any of the company's prize tracks (via scanning the main prize track QR code, with the option to judge only some of the prizes)
- Select the winner of their company's prize tracks
- View the schedule of events
- Role-specific resource links

______________________________________________________________________

#### `sponsor-organizer`

These are people from the organizing team in charge of working with sponsors.

**Features:**

- View a list of sponsors and their roles
- Manage sponsor roles
- Generate invitation links for specific companies, pre-configured with the company assignment so new sponsors are automatically associated on signup
- Help sponsors record their selected winners for sponsor prize tracks
- View judging results for sponsor prize tracks
- View the schedule of events
- Role-specific resource links

> **Note:** Judging organizers and admins can edit sponsor prize track details. Whether sponsors can edit their own tracks and invite colleagues remains an open question below.

______________________________________________________________________

#### `admin`

This label denotes hackathon administration, scoped to one hackathon, and includes the special permissions given to `*-organizer`s. Deployment-wide global administration is separate and belongs in RFC 0011. Administration does not imply unrestricted editing of private drafts or rewriting judging history.

**Features:**

- All permissions from `organizer`, `judge-organizer`, and `sponsor-organizer`
- View and correct managed data within their hackathon (participants, teams, projects, prize tracks, submitted applications, etc.), subject to privacy and history requirements
- Configure event parameters:
  - Maximum number of teams per table
  - Maximum number of people per team (configurable per hackathon)
  - Other event configuration as needed
- Create custom invitation links for any role
- Send messages to participants (individual, team-specific, or broadcast; details in a future messaging RFC)
- Manage the schedule (create, edit, remove events)
- View the schedule of events
- Role-specific resource links

______________________________________________________________________

### Team membership

Terrier records teams and membership as the source of truth. Automated matching and suggested teammates are deferred; Discord channels and the team-finding mixer support finding teammates initially. Browsing and searching teams remain part of the flow above.

- The team creator becomes the leader and can approve or reject join requests.
- A user can belong to at most one team per hackathon. Whether multiple pending join requests are allowed remains open.
- To switch teams, a member must leave their current team before requesting another team or creating one.
- Team links and QR codes open the request-to-join page. Joining requires a request and leader approval, including when someone shares a link as an invitation.
- Pending applicants are distinct from members. Approval must respect eligibility, team capacity, and the one-team constraint even when requests are handled concurrently.

Pending requests do not reserve capacity. They close when withdrawn, rejected, accepted, the requester loses eligibility, or team formation closes. Approval rechecks confirmed attendance, capacity, and absence of an active membership atomically. Team size is configured per hackathon; lowering the maximum below an existing team's size requires resolving that team first.

The leader edits team details and can transfer leadership to another active member. When the leader leaves, the remaining member with the earliest membership start time automatically becomes leader, with user ID as a stable tie-breaker. The last member leaving deactivates the team, preserving submission and membership history. Leaders cannot unilaterally remove accepted members; members can leave voluntarily. An organizer can remove a member or replace an absent leader after reviewing the request, recording a reason and notifying affected members. The dispute and escalation policy remains an Events decision.

Declining attendance atomically removes active membership and pending requests. If the departing attendee was leader, the remaining member with the earliest membership start time succeeds them, with user ID as a stable tie-breaker. The same succession rule applies to voluntary departure and organizer correction of attendance. A team without remaining members is deactivated. How membership changes affect projects already being judged belongs in the judging RFC.

### All-role features

The following features are available to every authenticated user, regardless of role:

- View their own profile[^3] (name, school, major, graduation year, status)
- Edit editable (or all if did not apply) profile fields[^3] (display name, dietary restrictions, shirt size, and other fields flagged as post-submission editable)
- View the schedule of events

\[^3\]: Note these options will be initially entered for participants via the application, and should be sourced from there for their profile.
People coming in through other routes (organizers, judges, sponsors) will edit all of these directly in their profile view (participants can also edit editable details here).

### Application and attendance transitions

Application drafts are private working copies. Submitting creates an immutable revision with a server-assigned revision ID and timestamp; later edits create a new submitted revision. Managers can compare revisions and see which revision informed a decision. A saved working copy is not a new submitted revision.

Each application field has a post-submission edit flag and a configured edit deadline. The hackathon application deadline is the upper bound for applicant edits. Organizers can set earlier locks for decision-relevant fields, including rolling admissions. Rejected and waitlisted applications are read-only. Managers may correct submitted data with a reason and visible history; they cannot silently change an applicant's private draft. Name and other global profile edits do not rewrite the submitted application snapshot.

| Current state | Action | Result and guard |
| --- | --- | --- |
| Draft | Applicant submits | Submitted revision; required fields validated before the application deadline |
| Submitted | Manager decides | Accepted, rejected, or waitlisted; decision references a submitted revision |
| Accepted, attendance pending | Applicant confirms | Confirmed before the confirmation deadline |
| Accepted, attendance pending | Applicant declines, or offer expires | Declined or expired |
| Confirmed | Applicant declines | Declined; membership rules applied |
| Declined or expired | Manager reopens offer | Attendance pending with a new confirmation deadline |
| Confirmed | Organizer checks in at an event | Participant capability; duplicate scans do not duplicate attendance |

Organizers can accept applicants, including waitlisted applicants, and reopen attendance confirmation. Bulk organizer operations will be supported in a later phase; their API and per-item outcomes belong in RFC 0011. Attendance capacity allocation and reservation mechanisms are outside this RFC. Expiry closes an offer rather than treating silence as a personal decline. Attendance changes after check-in preserve the check-in history, but declining removes current submission eligibility.

### Submission and judging boundary

One currently confirmed and checked-in member is sufficient to submit for the team. Other members do not need to have checked in. All active members can save a project draft and edit the shared project before the hackathon submission deadline. A submitted project cannot be removed or withdrawn; it can only be edited within the applicable deadline and locks. The team retains one project record and its revision history.

The hackathon submission deadline governs ordinary project edits. Cross-track freezes, judging snapshots, and handling edits after judging starts are outside this RFC and belong in the judging RFC. Table selection remains editable until frozen by a judging organizer.

Sponsors select the final winners for their own tracks. Judging scores inform that decision; organizer approval is not required for sponsor winner selections. Publishing is a separate organizer action, immediate or scheduled. Published winners do not expose raw evaluations, private notes, or participant email addresses. Broader score sharing and the judging algorithm remain separate design work.

### Recovery and initial communications

Organizer corrections, decisions, role grants, and deletions record actor, hackathon, time, reason where required, and the affected revision or state change in an append-only audit trail. Business deletions deactivate records and preserve their history. Restoring data is a new checked action, not an unconditional replay over newer changes. Secrets and private draft contents do not belong in the audit trail. Retention and personal-data erasure require a separate policy; soft deletion alone does not satisfy erasure.

The initial rollout provides authorized exports of applicant contact lists filtered by application and attendance status. Exports exclude private draft contents and are auditable. Organizers send decision and deadline messages through their existing tools until delivery integrations exist; the UI must not report that a message was sent. Integration priority is email, then text, then Discord, then Slack. Provider selection, consent, scheduling, delivery tracking, and retries belong in a communications RFC.

## Alternatives Considered

### Verification status roles

`unverified` and `verified` roles were considered for ensuring the validity of email addresses entered.
However, since the project will enable school-based SSO and Google authentication,
which immediately indicates the validity of an email for that user, this unnecessarily complicates the states.

If email-based authentication is added, the verification step should be front loaded before ever being allowed to enter the app,
removing the necessity for the app to be aware of the possibility of unverified users.

### `sponsor-manager` role

A `sponsor-manager` role was considered, granting sponsors the additional ability to edit their company's prize track details
and generate invitation links for other sponsors within their company.
The separate role remains deferred. Whether existing sponsors should have these capabilities remains an open question in this RFC.

## Open Questions

The rules above are proposed resolutions for review, not an accepted implementation mandate. The following decisions still need named owners before dependent tasks start:

- **Events: organizer permissions.** Confirm the action-to-capability matrix for application, judging, sponsorship, and hackathon administration. Preserve overlapping responsibilities without granting every organizer every private-data permission. RFC 0011 must reconcile this with the current one-role-per-hackathon schema.
- **Events: leadership recovery.** Oldest-member succession applies when a leader leaves or declines attendance. Only organizers can involuntarily remove members. Define how organizers verify an absent-leader request, notify members, and escalate disputes.
- **Events: pending join requests.** Allow one pending request at a time, or multiple requests with the first acceptance joining the team and closing the others? This remains unresolved.
- **Events: deadlines.** Choose default application, field-edit, attendance-confirmation, team-formation, and submission deadlines. Provide user-facing explanations when a spot expires or a change needs organizer help.
- **Sponsorship: privacy and capabilities.** Which team member fields, including email, should sponsors be able to see for projects entered in their tracks? This remains an open question in this RFC. Should sponsors be able to edit their own track descriptions and invite colleagues within their company and tracks? This also remains open in this RFC.
- **Finance: reimbursements.** Confirm categories, currencies, evidence requirements, receipt access, budget reservation rules, and retention. Issue #29 requires general reimbursements, including hotel and travel. Each category has an independent request and decision; payment and tax-form workflows are deferred.
- **Authentication: invitation lifecycle.** Define issuer capabilities, company/track scope, expiry, revocation, use limits, and redemption by a person without an account. Invitation links cannot grant access merely by being opened; RFC 0011 must specify authenticated redemption.
- **Judging: evaluation rules.** Define assignment, scoring, breaks, conflicts of interest, deliberation, cross-track freezes, and handling of project edits during judging in a separate RFC. Sponsor winner selection and organizer-controlled publication follow the boundary above.

## Implementation Phases

Implementation of different roles should be phased by timeline importance.

**1. Applications:**
The `applicant-draft` and `applicant-submitted` roles' functionality should be an initial target,
as well as the `application-manager` role, to allow for the rollout of applications potentially before the rest of the system is fully ready.
Reimbursement requests can follow alongside applications after Finance approves their evidence, budget, and access rules; they do not block the application launch.

**2. Attendance confirmation:**
The rest of the attendance flow (`attendee-potential`, `attendee-confirmed`, `attendee-declined`) should be the next milestone,
allowing for acceptance and confirmation of attendance, including team formation features.

**3. Event and participation:**
The `participant` role (check-in, project submission, table selection), `organizer` and `admin` event management,
and the full check-in system.

**4. Judging and sponsors:**
The `judge`, `judge-organizer`, `sponsor`, and `sponsor-organizer` roles,
dependent on a separate judging system RFC being finalized.
