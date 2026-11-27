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

## Organizer screens

| Page | Purpose |
| -- | -- |
| `/organizer/hackathons/new` | Create a hackathon. Only global admins can create. |
| `/organizer/hackathons/[slug]/settings` | Edit name, description, location, contact email, limits, and deadlines. |

Both pages share `HackathonSettingsForm`, which checks the same rules as the server before sending: text fields use 1 to 255 characters, the team size fits within the participant limit, and deadlines are in order. Deadlines are entered and shown in the browser's time zone, which the form displays.

Saving sends every settings field with the `version` the page loaded. The form passes `application_schema` through unchanged so it never overwrites the form editor's work. New hackathons start with `{"fields": []}`.

| Response | What the page does |
| -- | -- |
| 200 | Shows "Saved." and reloads the form with the new version. |
| 409 | Shows the server's message and a Reload button. This covers both a stale version and a team size smaller than an existing team. Reloading discards unsaved edits. |
| 422 | Shows each field error under its input. |
| 403 | Shows a permission message. |

Deactivating sends only `version` and `is_active: false`, asks for confirmation first, and returns to the home page. The API cannot reactivate a hackathon, so this cannot be undone from the UI.
