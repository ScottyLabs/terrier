# Applications

Applications are scoped to a hackathon and addressed by the applicant's Terrier user ID. Applicants save drafts and submit them with one route. Organizers read and list applications, limited by their capabilities. Confirmed attendees can browse a participant directory. See the [policy reference](../reference/policies.md) for the exact rules.

## Save or submit your application

Get your ID and CSRF token from `GET /api/auth/status`, then send your answers as a JSON object:

```ts
const { data, error } = await api.PUT("/api/hackathons/{slug}/applications/{user_id}", {
  params: { path: { slug: "tartanhacks", user_id: userId } },
  body: { data: answers, submit: false },
  headers: { "x-csrf-token": csrfToken },
});
```

- `submit: false` saves `data` as your working copy (`draft_data`). The submitted revision is unchanged.
- `submit: true` makes `data` the submitted revision and sets `status` to `pending`. Repeating a submission with unchanged answers does not create another revision.

The first save also adds you to the hackathon as a hacker. Saves are accepted until the application deadline while `status` is `null` (draft) or `pending`.

| Status / code | Caller action |
| -- | -- |
| 400 / `bad_request` | Send a JSON body with only `data` and `submit`. |
| 403 / `forbidden` | Use your own user ID and include the CSRF token. Your role in this hackathon may be deactivated. |
| 404 / `not_found` | The hackathon does not exist or is inactive. |
| 409 / `conflict` | Show `message`: the deadline passed, or the application was decided or deactivated. |
| 422 / `validation_failed` | `fields.data`: answers must be a JSON object. |

## Read an application

`GET /api/hackathons/{slug}/applications/{user_id}` returns one application. What you see depends on who you are:

| Caller | Sees |
| -- | -- |
| The applicant | Their application, including drafts and `draft_data` |
| Application manager or admin | Submitted applications; never drafts |
| Other organizers | Accepted applications with confirmed attendance only |

An application outside your view returns 404. `status` is `null` for a draft. Timestamps are RFC 3339 strings.

## List applications

Organizers list a hackathon's applications with `GET /api/hackathons/{slug}/applications`, filtered by the same visibility rules:

```ts
const { data } = await api.GET("/api/hackathons/{slug}/applications", {
  params: { path: { slug: "tartanhacks" }, query: { status: "pending" } },
});
```

The optional `status` filter narrows the list. Results are ordered by submission time. Users without organizer access in that hackathon get 403.

## Browse the participant directory

`GET /api/hackathons/{slug}/users` lists hackers who opted into the directory. Each entry has `user_id`, `display_name` and `profile`. `profile` holds only the fields named in the hackathon's `directory_fields`. Confirmed attendees and organizers of the hackathon may read it; others get 403.

See [Authentication](authentication.md), [API errors](../reference/api-errors.md), and [Frontend development](frontend.md) for shared behavior.
