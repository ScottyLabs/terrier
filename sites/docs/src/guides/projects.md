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
