# Projects

A project is a team's submission to a hackathon. Each team has at most one. The [API architecture RFC](https://git.cmu.dev/ScottyLabs/terrier/src/branch/main/rfcs/0011-api-architecture.md) lists the planned project routes; this page covers the ones that exist.

## Read a project

`GET /api/hackathons/{slug}/projects/{project_id}` returns one project. Active members of the owning team and organizers of the hackathon can read it, including while it is a draft. See [`ProjectReader`](../reference/policies.md#projectreader) for the access rule.

```ts
import { api } from "$lib/api/client";

const { data, error } = await api.GET(
  "/api/hackathons/{slug}/projects/{project_id}",
  { params: { path: { slug, project_id: projectId } } },
);
```

| Field | Meaning |
| -- | -- |
| `id`, `team_id`, `event_id` | UUIDs of the project, its team, and the event it was submitted at |
| `data` | The submission's answers, as stored |
| `status` | `draft` or `submitted` |
| `submitted_at` | Submission time in RFC 3339 format, or `null` for a draft |
| `track_ids` | UUIDs of the prize tracks the project is entered in |
| `version` | Revision number of the project |

| Status | Cause | Suggested handling |
| -- | -- | -- |
| 400 | `project_id` is not a UUID | Treat as a broken link |
| 401 | No session | Start sign-in |
| 403 | The caller is not on the owning team and is not an organizer | Explain that the project is not available to them |
| 404 | No such active hackathon, or no such project in it | Show a not-found page |

Judges and track sponsors cannot read projects through this route yet.
