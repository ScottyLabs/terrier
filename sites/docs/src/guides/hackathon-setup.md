# Hackathon setup API

Instance admins can create hackathons. The creator receives the hackathon admin capability and can edit its settings. Get your session and CSRF token from [Authentication](authentication.md).

## Create a hackathon

Choose a unique lowercase slug. Send dates as RFC 3339 timestamps. `application_schema` is the application form; see [Application form format](#application-form-format).

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

## Application form format

`application_schema` holds an ordered `fields` list. Applicants answer with an object keyed by each field's `id`.

```json
{
  "fields": [
    {"id": "school", "label": "School", "type": "text", "required": true},
    {"id": "shirt", "label": "Shirt size", "type": "single_choice", "required": false,
     "options": [{"id": "s", "label": "Small"}, {"id": "m", "label": "Medium"}]},
    {"id": "conduct", "label": "I agree to the code of conduct", "type": "checkbox", "required": true}
  ]
}
```

| Property | Rule |
| -- | -- |
| `id` | Up to 64 lowercase letters, digits and underscores, starting with a letter; unique in the form. Keep it when renaming or reordering a question, because answers are stored under it |
| `label` | The question text, 1 to 255 characters |
| `type` | One of the types below |
| `required` | `true` or `false`. Drafts may leave required questions blank; submitting may not |
| `options` | Only for `single_choice` and `multi_choice`: at least one `{id, label}`, with unique IDs following the `id` rule |

| `type` | Answer |
| -- | -- |
| `text` | One line, up to 255 characters |
| `long_text` | Up to 10,000 characters |
| `email` | An email address |
| `url` | A link starting with `http://` or `https://` |
| `number` | A number |
| `single_choice` | One option `id` |
| `multi_choice` | A list of distinct option `id`s |
| `checkbox` | `true` or `false`. A required checkbox must be checked |
| `file` | The attachment ID returned by the upload route |

A form that breaks these rules is rejected with 422. Each error key points at the problem, such as `settings.application_schema.fields[1].id`. An empty object `{}` is a form with no questions.

See [API errors](../reference/api-errors.md) and [Frontend development](frontend.md) for client integration.
