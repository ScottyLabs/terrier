# Profiles

Signed-in users can read their own profile with `GET /api/users/{user_id}`. The response contains their Terrier user ID, email, and display name. Other users' profiles are not accessible through this route.

## Read your profile

[Sign in](authentication.md#sign-in), then get your ID from the `user.id` field of `GET /api/auth/status`. A null `user` means you need to sign in.

Use that ID as `userId` in this request. Run it in the browser, such as in a Svelte `onMount` callback:

```ts
import { api } from "$lib/api/client";

const { data, error } = await api.GET("/api/users/{user_id}", {
  params: { path: { user_id: userId } },
});
```

The shared client sends the session cookie. On success, `data` contains `id`, `email`, and `display_name`; use these fields to display the profile. This read request does not require a CSRF token.

## Handle failures

When `error` is present, handle its `code` as follows:

| Status / code | Caller action |
| -- | -- |
| 400 / `bad_request` | Supply a valid user UUID. |
| 401 / `unauthenticated` | Ask the user to sign in. |
| 403 / `forbidden` | Use the signed-in user's ID; another user's profile is unavailable. |
| 500 / `internal_error` | Show that the profile could not be loaded. |

Network failures throw instead of returning `error`; catch them and show a connection error. See [API errors](../reference/api-errors.md) for the error format and [Frontend development](frontend.md#api-integration) for shared client conventions.
