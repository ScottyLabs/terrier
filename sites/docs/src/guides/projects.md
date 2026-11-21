# Projects

A project is a team's submission to a hackathon. Each team has at most one. The [API architecture RFC](https://git.cmu.dev/ScottyLabs/terrier/src/branch/main/rfcs/0011-api-architecture.md) lists the planned project routes; this page covers the ones that exist.

## Read a project

`GET /api/hackathons/{slug}/projects/{project_id}` returns one project with its team and tracks. Active members of the owning team and organizers of the hackathon can read it, including while it is a draft. Sponsors of a track it is entered in and judges assigned to it can read it once it is submitted. See [`ProjectReader`](../reference/policies.md#projectreader) for the access rules.

```ts
import { api } from "$lib/api/client";

const { data, error } = await api.GET(
  "/api/hackathons/{slug}/projects/{project_id}",
  { params: { path: { slug, project_id: projectId } } },
);
```

| Field | Meaning |
| -- | -- |
| `id`, `event_id` | UUIDs of the project and the event it was submitted at |
| `team` | The owning team's `id` and `name` |
| `tracks` | The `id` and `name` of each active prize track the project is entered in, ordered by name |
| `data` | The submission's answers, as stored |
| `status` | `draft` or `submitted` |
| `submitted_at` | Submission time in RFC 3339 format, or `null` for a draft |
| `version` | Revision number of the project |

| Status | Cause | Suggested handling |
| -- | -- | -- |
| 400 | `project_id` is not a UUID | Treat as a broken link |
| 401 | No session | Start sign-in |
| 403 | The caller has no access to this project, or it is still a draft and they are a sponsor or judge | Explain that the project is not available to them |
| 404 | No such active hackathon, or no such project in it | Show a not-found page |

The response has no team member details.

## List a team's project

`GET /api/hackathons/{slug}/teams/{team_id}/projects` returns a list holding the team's project, or an empty list if the team has not started one. Each item has the same fields as [Read a project](#read-a-project). Use this route when you know the team but not the project ID, for example on a team page.

```ts
const { data, error } = await api.GET(
  "/api/hackathons/{slug}/teams/{team_id}/projects",
  { params: { path: { slug, team_id: teamId } } },
);
const project = data?.[0];
```

When the team has a project, the same people who can read that project can list it. When it has none, only the team's active members and organizers get the empty list; everyone else gets 403. See [`TeamProjectReader`](../reference/policies.md#teamprojectreader).

| Status | Cause | Suggested handling |
| -- | -- | -- |
| 400 | `team_id` is not a UUID | Treat as a broken link |
| 401 | No session | Start sign-in |
| 403 | The caller cannot read the team's project, or the team has none and the caller is not a member or organizer | Explain that the project is not available to them |
| 404 | No such active hackathon, or no such active team in it | Show a not-found page |

## List all projects

`GET /api/hackathons/{slug}/projects` returns every project in the hackathon, drafts included, ordered by team name. It is for organizers who manage judging. Add `track_id` to list only projects currently entered in that track.

```ts
const { data, error } = await api.GET("/api/hackathons/{slug}/projects", {
  params: { path: { slug }, query: { track_id: trackId } },
});
```

Each item has the same fields as [Read a project](#read-a-project). See [`ProjectManager`](../reference/policies.md#projectmanager) for who can use it.

| Status | Cause | Suggested handling |
| -- | -- | -- |
| 400 | `track_id` is not a UUID, or the query has another parameter | Fix the request |
| 401 | No session | Start sign-in |
| 403 | The caller is not an organizer with judging-management or admin capability | Explain that the list is for judging managers |
| 404 | No such active hackathon, or `track_id` is not an active track in it | Show a not-found page |

The list is not paginated.

## Create a project

`POST /api/hackathons/{slug}/projects` creates the caller's team's project. A team has at most one. The caller must have confirmed attendance, a valid check-in at an active event in the hackathon, and an active team membership; teammates do not need to have checked in. See [`ProjectSubmitter`](../reference/policies.md#projectsubmitter).

```ts
const { data, error } = await api.POST("/api/hackathons/{slug}/projects", {
  params: { path: { slug } },
  body: {
    event_id: eventId,
    data: { title, description },
    track_ids: selectedTrackIds,
    status: "draft",
  },
  headers: { "x-csrf-token": csrfToken },
});
```

| Field | Meaning |
| -- | -- |
| `event_id` | UUID of an active event in the hackathon |
| `data` | The team's answers, as a JSON object. The server does not yet check them against a configured form. |
| `track_ids` | UUIDs of active prize tracks to enter, each at most once; may be empty |
| `status` | `submitted` to submit now; omit it or send `draft` to save a draft |

A successful request returns 201 with the project in the [read format](#read-a-project). Its `version` already counts each track entry.

| Status | Cause | Suggested handling |
| -- | -- | -- |
| 400 | The body is not valid JSON, misses a required field, has an unknown field, or has an unknown `status` | Fix the request |
| 401 | No session | Start sign-in |
| 403 | Missing CSRF token, or the caller is not eligible to submit | Explain that a checked-in team member must submit |
| 404 | No such active hackathon | Show a not-found page |
| 409 | The team already has a project, the submission deadline has passed, or the caller's team changed during the request | Show the message; offer to open the existing project |
| 422 | `event_id` or a track is not active in this hackathon, a track repeats, or `data` is not an object | Show `fields` beside the inputs |

## Edit a project

`PATCH /api/hackathons/{slug}/projects/{project_id}` changes the project's answers, its prize tracks, or submits a draft. The same eligibility applies, and the caller must belong to the project's team. Organizers cannot edit projects through this route.

Send the `version` from the project you last read with at least one change:

```ts
const { data, error } = await api.PATCH(
  "/api/hackathons/{slug}/projects/{project_id}",
  {
    params: { path: { slug, project_id: projectId } },
    body: { version: project.version, track_ids: selectedTrackIds },
    headers: { "x-csrf-token": csrfToken },
  },
);
```

| Field | Meaning |
| -- | -- |
| `version` | Required. The `version` of the project the client last read |
| `data` | Replacement answers, as a JSON object |
| `track_ids` | Replacement track selection; tracks left out are withdrawn, and re-entering one restores its earlier entry |
| `status` | `submitted` submits a draft. A submitted project stays submitted and keeps its first `submitted_at` |

Submitted projects remain editable until the submission deadline. Sending the same `data` again does not create a new version.

| Status | Cause | Suggested handling |
| -- | -- | -- |
| 400 | Invalid JSON, a missing `version`, no change, an unknown field or `status`, or a `project_id` that is not a UUID | Fix the request |
| 401 | No session | Start sign-in |
| 403 | Missing CSRF token, the caller is not eligible, or the project belongs to another team | Explain that a checked-in member of the team must edit |
| 404 | No such active hackathon, or no such project in it | Show a not-found page |
| 409 `stale_revision` | Someone changed the project after the client read it | Reload the project, show the latest answers, and let the person reapply their change |
| 409 `conflict` | The deadline has passed, the request tried to return a submitted project to `draft`, or the caller's team changed | Show the message |
| 422 | A track is not active in this hackathon, a track repeats, or `data` is not an object | Show `fields` beside the inputs |
