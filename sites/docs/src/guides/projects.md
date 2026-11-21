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
