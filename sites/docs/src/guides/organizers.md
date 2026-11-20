# Organizer management

Terrier provides API endpoints for adding and deactivating hackathon organizers.

Both endpoints require an authenticated user with an active hackathon membership and the `HackathonAdmin` capability for the requested hackathon.

## Add an organizer

`POST /api/hackathons/{slug}/organizers`

Grants organizer capabilities to an existing user. An active hackathon membership retains its primary role.

### Request

```http
POST /api/hackathons/test-hackathon/organizers
Content-Type: application/json
Cookie: <session-cookie>
x-csrf-token: <csrf-token>

{
  "user_id": "550e8400-e29b-41d4-a716-446655440000",
  "capabilities": ["judging_management"]
}
```

The `user_id` is the Terrier user's UUID. Supported capabilities are:

- `application_management`
- `judging_management`
- `sponsorship_management`
- `hackathon_admin`

### Response

- `201 Created`: Organizer successfully added.
- `400 Bad Request`: Invalid input or the user already has an organizer record.
- `401 Unauthorized`: Not authenticated.
- `403 Forbidden`: Insufficient permissions.
- `404 Not Found`: Hackathon or target user not found.

The audited transaction creates an organizer record. If the user has no hackathon membership, it also creates one with the organizer role.

## Deactivate an organizer

`PATCH /api/hackathons/{slug}/organizers/{user_id}`

Deactivates an existing organizer without deleting their records.

### Request

```http
PATCH /api/hackathons/test-hackathon/organizers/550e8400-e29b-41d4-a716-446655440000
Content-Type: application/json
Cookie: <session-cookie>
x-csrf-token: <csrf-token>

{
  "is_active": false
}
```

### Response

- `204 No Content`: Organizer successfully deactivated.
- `400 Bad Request`: Invalid input or attempted reactivation.
- `401 Unauthorized`: Not authenticated.
- `403 Forbidden`: Insufficient permissions or attempted self-deactivation.
- `404 Not Found`: Hackathon or active organizer not found.

The audited transaction deactivates the organizer record. It also deactivates the membership when its primary role is organizer.

Restoration using `is_active: true` is not currently supported.

## Related documentation

- [Authentication](authentication.md)
- [API routes and policies](api-routes.md)
- [API errors](../reference/api-errors.md)
- [Authorization policies](../reference/policies.md)
