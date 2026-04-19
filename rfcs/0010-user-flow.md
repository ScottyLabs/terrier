# RFC 0010: User Flow

- **Status:** Draft
- **Author(s):** @brucemcrooster
- **Created:** 2026-04-19

## Overview

This proposal lays out the full path through the app taken by all users, and what should be accessible to them at a given stage.

## Motivation

The design and scoping of what is available to users at different stages is incredibly important.
It serves as a fundamental basis for both the functionality of the app and the clear and proper disclosure of the apps functionality as relevant.

## Goals

- Define the lifecycle of users coming from different starting points
- Determine what functionality (and, correspondingly, access) should be available to users at different stages in their lifecycle
- Lay out how users transition between states

## Non-Goals

- This RFC does not intend to prescribe a specific permissioning structure
    - It does use a conceptual model reminiscent of role-based access control (RBAC),
      but could be implemented into the authentication system in another way
- This RFC will not provide any code more in depth than, potentially, pseudo-code.
  Many implementations are possible that would conform to the specification set forth here,
  so it will be the job of a future RFC to set forth the actual plan for a concrete implementation.

## Detailed Design

### Custom links

To simplify the distribution of permissions to non-hackers (organizers, judges, sponsors),
this RFC proposes custom invitation links as the method for doing this.

These could be emailed directly (which allows some email verification, if we think sponsors or judges will need a non-Google, non-edu sign-in options),
or generated via links.

Whatever the medium, these should be able to be created by
- `admin`s
- `judge-organizer`s
- `sponsor-organizer`s
- `sponsor-manager`s (if implemented)

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

#### `applicant-draft`

This represents a user who is submitting an application. They should have the ability to edit their application and submit.

This role should be given to anyone registering through the main website.

Once submitted, they should lose the `applicant-draft` role and move to `applicant-submitted`, since they no longer can "submit" an application
(their only option is to edit).

#### `applicant-submitted`

This represents a potential participant who has submitted an application.
They should be able to update their application[^1] and view the status of their travel reimbursement requests, if applicable.

This role is given once an `applicant-draft` submits their application.

If their application is accepted, they will get the additional role `attendee-potential`.

[^1]: On a side note, when an application is updated, it should be made clear to `application-manager`s,
perhaps through a separate section of the dashboard, and preferably with a diff if possible.

#### `attendee-potential`

This represents a potential participant whose application has been accepted.
They should be able to confirm their attendance, in which case they switch (read: lose `attendee-potential`) to `attendee-confirmed`,
or decline their attendance, in which case they switch to `attendee-declined`.

#### `attendee-confirmed`

This represents a user who has accepted their spot, and should now have access to
- creating and joining teams
- their event check-in QR code (which upon scanning will add them as a `participant`)
- viewing the schedule of events
- hacking time countdown
- change attendance status to `attendee-declined`

At this stage, there should be a very clear message that they need to check in in-person (get their QR code scanned)
at some event to get access to submitting their project.
This should, of course, be worded in a way such that it’s also applicable before the event ever starts,
since almost all will reach this stage before the beginning of the event.

#### `attendee-declined`

A user who has been accepted but declined to come.
The dashboard should make it very clear that "We’re sorry to hear you couldn’t make it" and not show additional options.
They should have the option to update their attendance status, which will switch them to `attendee-confirmed`.

#### `participant`

A user who has checked in to the event. They now have access to project submission for their team.

This role should be given to an `attendee-confirmed` who checks in to *any* event.
This simplifies the complexity of having to deal with people who arrived late and didn’t get checked in to the main check-in session,
since they’re now able to attend talks and be logged correctly.

#### `application-manager`

This role will have access to the full list of applications, with the ability to accept or reject those applications.

They will also be able to see and approve/deny travel reimbursements, entering a certain amount to be given if approved.

#### `organizer`

An organizer can view
- participant details, including
    - any details included in their application
    - their current team, with a link to view that team’s details
- team details, including
    - members, with a link to view individual details
    - project submission (both status and information)
    - table number
- track winner info, before it is marked as "published"

They can also check people into events (any `attendee-confirmed`, regardless of `participant` status).
This can be done either by scanning that person’s QR code or by manually searching the person.

#### `admin`

This role should have access to every piece of data, with the ability to edit it.
Anything they can view, they should be able to edit (besides, perhaps, judging data).
And anything that is shown to anyone, they should be able to see.

They should also have any of the special permissions given to `*-organizer`s.

Any uncertain event configuration parameters
(like max number of teams per table, number of people per team (if we leave that dynamic))
should be configurable by them, with notes when we may be uncertain of the consequences (such as when team size may break judging).

#### `judge`

A judge will be assigned prize tracks as part of the custom link they authenticate by.
They should have the option to select their prize tracks
(by default all should be selected, and the UI should make it every clear when not all are selected (e.g. a "Judge Selected" vs "Judge All" button)).

They will be able to judge for that track, take notes, and then participate in the final judging deliberations (see RFC NNNN).

They should also be able to "take a break" while judging,
which will first show a prompt notifying them that their current team assignment may change when they come back,
then remove them temporarily from the judging systems understanding of who is available,
thus ensuring no team is stuck because their currently assigned judge went to the bathroom.
The judge should select something to re-enter, be assigned a new team (allowed to be their previous assignment if that’s what the system decided),
and come back into the rotation.

#### `judge-organizer`

This is an organizer in charge of managing judges.
They should be able to judge (if they want to) with all the functionality that judges have (like "taking breaks").

They also have the ability to edit information about judge track assignments, tracks themselves, and submission information (including table numbers).
They can create judge invitation links, and configure the tracks those using the invitation will be assigned to.
They should be able to see a list of all judges, judging results, and final judge deliberation info (see RFC NNNN).

They can select the winner of a track, and select when to publish those details.

#### `sponsor`

A sponsor will be able to see a list of projects (with details) that have applied to that company’s prize track(s).
They can also see the information on judging results and winners for those prize tracks.

They will be able to self-select themselves as a `judge` for any of the company’s prize tracks,
but will have to scan the main prize track QR code to be added to those, with the option to only judge some of the prizes.

#### `sponsor-manager`

This person is a `sponsor`, with all of the permissions that come with it,
with the additional ability to edit company prize track details (descriptions, rubric items, evaluation approach?).
They can also generate links to add new `sponsor`s to their company.

They can additionally select the winner of their prize tracks (something regular sponsors cannot do).

#### `sponsor-organizer`

These are people from the organizing team in charge of working with sponsors.
They are able to see a list of sponsors and their roles, and change them between `sponsor` and `sponsor-manager`.
They can also generate invitation links to specific companies, with the option to generate both a `sponsor` and `sponsor-manager` link.

## Alternatives Considered

### Verification status roles

`unverified` and `verified` roles were considered for ensuring the validity of email addresses entered.
However, since the project will enable school-based SSO and Google authentication,
which immediately indicates the validity of an email for that user, this unnecessarily complicates the states.

If email-based authentication is added, the verification step should be front loaded before ever being allowed to enter the app,
removing the necessity for the app to be aware of the possibility of unverified users.

## Open Questions

How will these roles be represented in the database? RBAC?
Or just specific models in the database for each type of role?

Is an `applicant-rejected` role worth considering? Are there significant differences?
Should they potentially no longer be allowed to edit their application?

## Implementation Phases

Implementation of different roles should be phased by timeline importance.

This means the `applicant` role’s functionality should be an initial target,
as well as the `application-manager` role, to allow for the rollout of applications potentially before the rest of the system is fully ready.

The rest of the application flow should be the next milestone, allowing for acceptance and confirmation of attendance.

