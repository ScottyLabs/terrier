# Registration development

Use a local database. Sign in as your reviewer, then copy `user.id` from `GET /api/auth/status`. From the dev shell at the repository root, run:

```sh
psql "$DATABASE_URL" -v reviewer_id='<reviewer UUID>' -f scripts/registration-fixture.sql
```

Use `registration-test` as the hackathon slug. Sign in with a different account in a separate browser profile to apply. The first draft save creates the applicant membership.

- **Form proposal:** Store an ordered `fields` array in `application_schema`. Each field has a stable `id`, `label`, `type`, and `required` flag. Use stable option IDs for choice fields. Keep IDs when rearranging or renaming questions.
- **Application API:** Send `{data: {school: "CMU"}, submit: false}` to `PUT /api/hackathons/{slug}/applications/{user_id}` to save a draft. Send `submit: true` to submit. Read the application and status from the same path with `GET`. Server-side form validation and version checks still need implementation.
- **Attachment proposal:** Upload the file through Terrier. Store the returned opaque attachment ID as the answer, for example `{"resume":"<attachment UUID>"}`. The server verifies that it belongs to the application and field. Keep object keys and credentials on the backend.

Use fake answers and files. The sample form and attachment shapes are proposals for the groups to agree on before wiring their endpoints together.
