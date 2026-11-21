# Hackathon setup API

Instance admins can create hackathons. The creator receives the hackathon admin capability and can edit its settings. Get your session and CSRF token from [Authentication](authentication.md).

## Create a hackathon

Choose a unique lowercase slug. Send dates as RFC 3339 timestamps. The application schema is a JSON object that the registration form will read.

```http
POST /api/hackathons
Content-Type: application/json
x-csrf-token: <csrf-token>

{
  "slug": "spring-hacks",
  "settings": {
    "name": "Spring Hacks",
    "description": "A weekend hackathon",
    "location": "Campus",
    "contact_email": "events@example.com",
    "max_team_size": 4,
    "max_participants": 120,
    "application_deadline": "2027-01-01T00:00:00Z",
    "team_formation_deadline": "2027-01-02T00:00:00Z",
    "submission_deadline": "2027-01-03T00:00:00Z",
    "application_schema": {"fields": []}
  }
}
```

On 201, save the returned slug and version. On 409, pick a different slug.

## Read or change settings

Anyone can read an active hackathon at `GET /api/hackathons/{slug}`. Use its `settings` to populate the editor. To save, send `PATCH /api/hackathons/{slug}` with the latest `version`, the full edited `settings` object, and the `x-csrf-token` header.

On 200, keep the returned version for the next edit. On 409, read the error message. For a stale version, reload the settings before applying your edits again. Show field errors from 422 next to the relevant inputs.

To deactivate the hackathon, send `{"version": 1, "is_active": false}` with the current version and CSRF token. Public reads return 404 after deactivation.

See [API errors](../reference/api-errors.md) and [Frontend development](frontend.md) for client integration.
