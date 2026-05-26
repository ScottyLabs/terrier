# RFC 0010: User Flow

- **Status:** Draft
- **Author(s):** @brucemcrooster
- **Created:** 2026-04-19

## Overview

This proposal lays out the full path through the app taken by all users, and what should be accessible to them at a given stage.
These roles will be enforced in the system by SLAC, as described in [RFC 0009](./0009-slac.md).

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
- This RFC does not cover the detailed mechanics of the judging system (pairwise comparison, deliberation, etc.). That will be addressed in a separate RFC (see RFC NNNN).
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

### "Roles"

The following are the various roles, a non-exclusive set of things which define what a user can do or see at a given point in time.

In general, roles with different prefixes should be thought of as additive permissions,
so an `applicant-submitted` who becomes a `participant` should continue to have the `applicant-submitted`-related permissions,
such as viewing and editing their application.
Roles with the same prefix should be considered mutually-exclusive.
This is done partially for not having to re-list every permission,
but would also be useful (if these roles become a concept in the final system) for statistics on participation counts.

---

#### `applicant-draft`

This represents a user who is submitting an application. This role should be given to anyone registering through the main website.
Once submitted, they should lose the `applicant-draft` role and move to `applicant-submitted`, since they no longer can "submit" an application
(their only option is to edit).

**Features:**
- View and edit all application form fields
- Save application drafts (partial completion persisted across sessions)
- Submit the completed application
- Optionally request travel reimbursement as part of the application (which grants `tr-applied` upon submission; see below)

---

#### `applicant-submitted`

This represents a potential participant who has submitted an application.
This role is given once an `applicant-draft` submits their application.
If their application is accepted, they will get the additional role `attendee-potential`.

**Features:**
- View submitted application
- Edit application fields that are flagged as editable post-submission (e.g., display name, dietary restrictions, shirt size)[^1]
- View application status (submitted, accepted, rejected)
- View travel reimbursement request status, if applicable (see `tr-applied`)

> **Note:** The application form system should support a per-field flag indicating whether a field is editable after submission.
> For changes to non-editable fields, applicants must contact the organizing team directly,
> and an `application-manager` can make the edit on their behalf.

[^1]: When an application is updated, it should be made clear to `application-manager`s,
perhaps through a separate section of the dashboard, and preferably with a diff if possible.

---

#### `applicant-rejected`

> **Note:** See Open Questions for discussion on whether this role is necessary.
> In the current design, there is no meaningful difference in accessible features between `applicant-submitted` and `applicant-rejected`.
> The distinction may be useful for statistics and for preventing further edits to the application, if desired.

---

#### `tr-applied`

This role is granted when a submitted application includes a travel reimbursement request with details filled out.
It is removed if the applicant later deselects the travel reimbursement option in their application
(since this is an editable field).

**Features:**
- View travel reimbursement request status (applied, accepted, denied)

> **Note:** Travel reimbursement decisions can be fluid — as accepted attendees decline or remove their TR requests,
> `application-manager`s may accept additional requests.
> Removing the TR request from the application should automatically remove any `tr-*` roles.

---

#### `tr-accepted`

This role is granted by an `application-manager` (or `admin`) when a travel reimbursement request is approved.

**Features:**
- Everything from `tr-applied`
- Fill out the reimbursement details form: upload receipts and enter expense amounts

---

#### `attendee-potential`

This represents a potential participant whose application has been accepted.
They should be able to confirm their attendance, in which case they switch (read: lose `attendee-potential`) to `attendee-confirmed`,
or decline their attendance, in which case they switch to `attendee-declined`.

**Features:**
- Confirm attendance (transitions to `attendee-confirmed`)
- Decline attendance (transitions to `attendee-declined`)

---

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
- Create a new team (automatically added as a member)
- Browse and search open teams (by team name or member names)
- View team details (description, members list)
- Send a request to join a team (if team is not full)
- Accept or reject join requests from others (if on a team)
- Invite members to their team
- Edit their team's name and description
- Leave their team
- Change attendance status to `attendee-declined`

---

#### `attendee-declined`

A user who has been accepted but declined to come.
The dashboard should make it very clear that "We're sorry to hear you couldn't make it" and not show additional options beyond updating their status.

**Features:**
- View declined status message
- Change attendance status to `attendee-confirmed`

---

#### `application-manager`

This role will have access to the full list of applications, with the ability to accept or reject those applications.

**Features:**
- View the full list of applications, with search and filtering
    - Can view draft applications, but should be a distinct place from actual applications,
      to enable troubleshooting with participants who may not have submitted yet
- View individual application details
- Accept or reject applications
- Edit application details on behalf of applicants (for changes to non-editable fields)
- View a section highlighting recently updated applications (preferably with diffs)
- View all travel reimbursement requests
- Approve or deny travel reimbursement requests, specifying an approved amount
- Transition applicants between `tr-applied` and `tr-accepted`

---

#### `participant`

A user who has checked in to the event. They now have access to project submission for their team.

This role should be given to an `attendee-confirmed` who checks in to *any* event.
This simplifies the complexity of having to deal with people who arrived late and didn't get checked in to the main check-in session,
since they're now able to attend talks and be logged correctly.

**Features:**
- All team operations from `attendee-confirmed` (create, join, search, invite, leave, etc.)[^2].
  Probably simplest to just receive these transitively, since a `participant` should always also be an `attendee-confirmed`.
- Create a project submission for their team, including:
    - Project name
    - Project description
    - Repository URL
    - Zipped file upload (optional)
    - Presentation slides URL
    - Video URL (optional)
    - Prize track selection
- Edit project submission details and prize track selections (until frozen by a `judge-organizer`)
- Select a table number after submitting a project (manual entry or QR code scan; one team per table)
- Edit table number selection (until frozen by a `judge-organizer`)

[^2]: Team operations remain accessible after check-in.
Periodic backups of team membership data are recommended for the unlikely event of a member maliciously removing teammates.
These backups are likely already part of standard infrastructure.

---

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

---

#### `judge`

A judge will be assigned prize tracks as part of the custom link they authenticate by.

**Features:**
- View assigned prize tracks
- Select which of their assigned tracks to actively judge (UI should default to all selected, with a clear indication when not all are selected — e.g. a "Judge Selected" vs "Judge All" toggle)
- Judge assigned projects and submit evaluations (judging mechanics defined in RFC NNNN)
- Take notes on projects during judging
- "Take a break": triggers a prompt warning that their current team assignment may change, then removes them from the active judging rotation temporarily
- Re-enter judging (after a break): gets assigned a new team (which may be the same as before if the system decides) and returns to the rotation
- Participate in final judging deliberations (see RFC NNNN)
- View the schedule of events
- Role-specific resource links

---

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
- View final deliberation info (see RFC NNNN)
- Select the winner of a track
- Control when winner details are published
- Freeze project submission editing (per track) before the expo
- Start judging on a per-track basis (with a "Select All" option, since most tracks will start simultaneously)
- View the schedule of events
- Role-specific resource links

---

#### `sponsor`

A sponsor associated with a company and its prize tracks.

**Features:**
- View a list of projects (with details) that have applied to the company's prize track(s), including:
    - Project submission details (name, description, repo, slides, video)
    - Team member info (see Open Questions)
- View judging results and winners for the company's prize tracks (per-track summaries, easy access to slides/video/repo, clear metric descriptions)
- Self-select as a `judge` for any of the company's prize tracks (via scanning the main prize track QR code, with the option to judge only some of the prizes)
- Select the winner of their company's prize tracks
- View the schedule of events
- Role-specific resource links

---

#### `sponsor-organizer`

These are people from the organizing team in charge of working with sponsors.

**Features:**
- View a list of sponsors and their roles
- Manage sponsor roles
- Generate invitation links for specific companies, pre-configured with the company assignment so new sponsors are automatically associated on signup
- Select winners for sponsor prize tracks
- View judging results for sponsor prize tracks
- View the schedule of events
- Role-specific resource links

> **Note:** With the `sponsor-manager` role deferred to a future RFC, editing of sponsor prize track details
> (descriptions, rubrics, event-attendance restrictions) currently falls to `judge-organizer`s and `admin`s.
> This responsibility may shift when `sponsor-manager` is introduced.

---

#### `admin`

This role should have access to every piece of data, with the ability to edit it.
Anything they can view, they should be able to edit (besides, perhaps, judging data).
And anything that is shown to anyone, they should be able to see.
They should also have any of the special permissions given to `*-organizer`s.

**Features:**
- All permissions from `organizer`, `judge-organizer`, and `sponsor-organizer`
- View and edit all data across the system (participants, teams, projects, prize tracks, applications, etc.)
- Configure event parameters:
    - Maximum number of teams per table
    - Number of people per team (see Open Questions)
    - Other event configuration as needed
- Create custom invitation links for any role
- Send messages to participants (individual, team-specific, or broadcast; details in a future messaging RFC)
- Manage the schedule (create, edit, remove events)
- View the schedule of events
- Role-specific resource links

---

### All-role features

The following features are available to every authenticated user, regardless of role:
- View their own profile[^3] (name, school, major, graduation year, status)
- Edit editable (or all if did not apply) profile fields[^3] (display name, dietary restrictions, shirt size, and other fields flagged as post-submission editable)
- View the schedule of events

[^3]: Note these options will be initially entered for participants via the application, and should be sourced from there for their profile.
      People coming in through other routes (organizers, judges, sponsors) will edit all of these directly in their profile view (participants can also edit editable details here).

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
This has been deferred to a future RFC to keep this document focused on clear must-haves.

## Open Questions

- **Database representation of roles:** How will these roles be represented in the database? RBAC? Or specific models for each type of role?

- **`applicant-rejected` role:** Is a distinct `applicant-rejected` role worth including?
  Are there meaningful differences in what should be accessible (e.g., should they no longer be able to edit their application)?
  Or is a status indicator within `applicant-submitted` sufficient?
  A similar question applies to `tr-applied` vs. a hypothetical `tr-denied` — currently there is no feature distinction.

- **Sponsor visibility of team member info:** Should sponsors be able to see team member details (currently scoped as name and email) for projects applying to their prize tracks?
  How much detail is appropriate? This needs further discussion with the sponsorship team.

- **Team size configuration:** Should the "number of people per team" remain a configurable parameter for `admin`s?
  In previous years, there was uncertainty about whether changing this value would break things on both the app side and the logistics side.
  It may be safer to hardcode this or remove the option entirely.

- **Travel reimbursement form complexity:** The travel reimbursement request is part of the main application, but the post-acceptance reimbursement form (receipts, amounts) may be complex enough that it isn't dynamically configurable in the same way other editable fields are.
  This may require a dedicated form rather than being managed through the general application field-editability system.

## Implementation Phases

Implementation of different roles should be phased by timeline importance.

**Phase 1 — Applications:**
The `applicant-draft` and `applicant-submitted` roles' functionality should be an initial target,
as well as the `application-manager` role, to allow for the rollout of applications potentially before the rest of the system is fully ready.
The `tr-applied` and `tr-accepted` roles should be implemented alongside this phase.

**Phase 2 — Attendance confirmation:**
The rest of the attendance flow (`attendee-potential`, `attendee-confirmed`, `attendee-declined`) should be the next milestone,
allowing for acceptance and confirmation of attendance, including team formation features.

**Phase 3 — Event and participation:**
The `participant` role (check-in, project submission, table selection), `organizer` and `admin` event management,
and the full check-in system.

**Phase 4 — Judging and sponsors:**
The `judge`, `judge-organizer`, `sponsor`, and `sponsor-organizer` roles,
dependent on the judging system RFC (RFC NNNN) being finalized.
