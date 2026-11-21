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

### File fields

`file_fields` sets which files applicants can upload. Each key is the application answer that will hold the file ID. If you omit `file_fields`, the hackathon accepts a `resume` PDF up to 5 MB:

```json
"file_fields": {
  "resume": {"types": ["application/pdf"], "max_bytes": 5242880},
  "portfolio": {"types": ["*/*"], "max_bytes": 20971520}
}
```

- `types` lists accepted MIME types. A field without `types` accepts PDFs only. Use `["*/*"]` to accept any file.
- `max_bytes` is the largest accepted file, up to 25 MB. The default is 5 MB.
- Keys use lowercase letters, numbers and underscores.

Types are checked against file contents, so list only types that have a recognizable format, such as `application/pdf` or `image/png`. Plain text and similar formats need `*/*`.

## Read or change settings

Anyone can read an active hackathon at `GET /api/hackathons/{slug}`. Use its `settings` to populate the editor. To save, send `PATCH /api/hackathons/{slug}` with the latest `version`, the full edited `settings` object, and the `x-csrf-token` header.

On 200, keep the returned version for the next edit. On 409, read the error message. For a stale version, reload the settings before applying your edits again. Show field errors from 422 next to the relevant inputs.

To deactivate the hackathon, send `{"version": 1, "is_active": false}` with the current version and CSRF token. Public reads return 404 after deactivation.

See [API errors](../reference/api-errors.md) and [Frontend development](frontend.md) for client integration.
