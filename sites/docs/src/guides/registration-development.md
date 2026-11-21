# Registration development

Use a local database. Sign in as your reviewer, then copy `user.id` from `GET /api/auth/status`. From the dev shell at the repository root, run:

```sh
psql "$DATABASE_URL" -v reviewer_id='<reviewer UUID>' -f scripts/registration-fixture.sql
```

Use `registration-test` as the hackathon slug. Sign in with a different account in a separate browser profile to apply. The first draft save creates the applicant membership.

- **Form:** `application_schema` follows the [application form format](hackathon-setup.md#application-form-format). Setup rejects malformed forms, and the application API checks answers against the form.
- **Application API:** Send `{data: {school: "CMU"}, submit: false}` to `PUT /api/hackathons/{slug}/applications/{user_id}` to save a draft. Send `submit: true` to submit. Read the application and status from the same path with `GET`. Version checks still need implementation.
- **Attachment proposal:** Upload the file through Terrier. Store the returned opaque attachment ID as the answer, for example `{"resume":"<attachment UUID>"}`. The server verifies that it belongs to the application and field. Keep object keys and credentials on the backend.

Use fake answers and files. The attachment shape is a proposal for the groups to agree on before wiring the upload endpoint. Until it exists, the fixture's required `resume` question blocks submission; drafts still save.
