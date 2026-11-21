# Prize tracks and results

Track routes use a hackathon slug and, where applicable, a track UUID. Sign in for writes and private result reads. Send the session's `x-csrf-token` header on writes. See [Authentication](authentication.md) and [API errors](../reference/api-errors.md).

## Create a track

An active hackathon admin may create a general track or one for an organization associated with the hackathon. An active sponsor may create a track only for their own associated organization.

```http
POST /api/hackathons/demo/tracks
Content-Type: application/json
x-csrf-token: <session token>

{"name":"Best App","description":"A sponsor prize","judging_method":"expo","sponsor_org_id":"bdb2cef5-a417-4652-bb15-7c735463ee68"}
```

The server returns `201` with the new track ID, public details, and `version`. `scoring` and `expo` are accepted judging methods. New tracks have empty criteria and eligibility and are not ready for judging until those settings are defined by a later contract.

## Edit or deactivate

Hackathon admins, judging managers, and sponsors of the track's organization may edit its name and description before judging starts. Only hackathon admins may deactivate. Send the latest `version` from the create or update response.

```http
PATCH /api/hackathons/demo/tracks/<track-id>
Content-Type: application/json
x-csrf-token: <session token>

{"version":1,"name":"Best Web App"}
```

To deactivate, send `{"version":1,"is_active":false}`. A stale version returns `409`; reload current track data before retrying. Restoration and changes to judging configuration or sponsor affiliation are not defined by this route.

## Read internal results

Hackathon admins, judging managers, and active sponsors of the track's organization can read the result list at `GET /api/hackathons/demo/tracks/<track-id>/results`. The private response contains submitted projects and their individual evaluation status, result payload, rubric, and judge ID. It does not calculate scores or rankings. `GET /api/hackathons/demo/tracks/<track-id>/results/<project-id>` returns one project's private evaluation data. An active project-track entry is required.

## Publish winners

Hackathon admins and judging managers publish a curated list. Each winner must be an active submitted project entered in the same track and hackathon. `release_at` is optional; omitting it releases immediately. A future timestamp schedules release. Publication stores a snapshot of the supplied list without changing the track's current `winners` selection.

```http
POST /api/hackathons/demo/tracks/<track-id>/results
Content-Type: application/json
x-csrf-token: <session token>

{"version":2,"winners":[{"project_id":"bdb2cef5-a417-4652-bb15-7c735463ee68"}],"release_at":"2027-01-01T12:00:00Z"}
```

Before release, unauthenticated callers receive `401` and unauthorized signed-in callers receive `403` from the result list. After release, they see only the curated `winners` and release time. Individual project results remain private. A second publication returns `409`; changing or revoking a publication needs a separate approved contract.
