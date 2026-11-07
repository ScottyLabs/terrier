---
title: Authentication
description: Configure Terrier's Keycloak login and Valkey sessions.
---

Terrier manages OpenID Connect on the Rust backend. Keycloak handles Google and
InCommon sign-in. Tokens stay in Valkey; the browser receives an opaque session
cookie.

## Configuration

The server requires these settings before it opens the listener:

| Setting | Purpose |
| --- | --- |
| `DATABASE_URL` | PostgreSQL connection |
| `VALKEY_URL` | Session store connection |
| `APP_URL` | Public frontend/API origin, without a path |
| `KEYCLOAK_URL` | Keycloak base URL |
| `KEYCLOAK_REALM` | Realm name |
| `OIDC_CLIENT_ID` | Confidential Keycloak client |
| `OIDC_CLIENT_SECRET` | Client secret |
| `OAUTH_RELAY_URL` | Registered Ricochet callback URL |
| `INSTANCE_ADMIN_SUBJECTS` | JSON array of subjects under the configured issuer |

Use `[]` for no instance admins. Local development sets this explicitly. Deployed
instances must configure it through secretspec. Loading this setting does not
implement authorization checks.

HTTPS is required except for loopback development URLs. Cookies are host-only,
HttpOnly, SameSite=Lax, and Secure on HTTPS. Sessions expire after 48 hours of
inactivity. PostgreSQL, Valkey, and Keycloak discovery failures prevent startup.

## Sign-in

Navigate to `/api/auth/login?return_to=/dashboard`. The destination must be a
local application path. Terrier sends the browser through Keycloak and Ricochet
to `/api/auth/callback`, validates the response, rotates the session ID, and
completes local user synchronization before redirecting to the requested page.
Without a destination, sign-in returns to `/`.

Register `OAUTH_RELAY_URL` as the Keycloak client's redirect URI. Ricochet's state
contains the Terrier callback destination and a random value checked against the
server session. The flow uses PKCE and validates the ID token's signature,
issuer, audience, expiry, and nonce. Request the email and profile scopes, with
nonempty email and name claims mapped in Keycloak.

The frontend and API must share `APP_URL`. The Vite development server proxies
`/api` to port 3000 so the callback receives the same host cookie.

Rejected refresh tokens require a new sign-in. Provider and storage failures
return server errors. Login failures do not redirect silently to the home page.
Provider metadata and signing keys refresh every 15 minutes. Refresh failures
are logged; token validation continues against the cached keys until the next
successful refresh. A newly activated signing key may require up to 15 minutes
to become available without a restart.

## Logout

`POST /api/auth/logout` deletes the local session and leaves Keycloak SSO intact.
It requires the session-bound `x-csrf-token` header and rejects a foreign Origin.
Successful deletion returns 204; storage failures return 500.

## Request authentication

`GET /api/auth/status` returns `user` and `csrf_token`. Both are null when signed
out. The user contains its Terrier ID, email, and display name. Tokens and provider
identity details are never returned. Responses use `Cache-Control: no-store`.

Terrier trusts authentication from its configured Keycloak issuer and does not
require the `email_verified` claim or send a separate verification email. Identity
uses the validated issuer and subject; email is profile data, not an identity key.

Register application routes through `auth::middleware::application_routes`, then
nest them under `/api` within the session layer. Handlers extract `CurrentUser`
to access the local user. The middleware rejects anonymous requests with 401 and
unsafe requests without a matching CSRF token with 403. Database and session
failures return 500 and stop handler execution. Unknown API paths return JSON 404
responses, including when serving the frontend.

Health, auth status, login, callback, and logout remain outside the application
authentication gate. An explicit `return_to` path starts a fresh login.

See [API routes and policies](/guides/api-routes/) for the own-profile policy,
route registration, shared errors, and typed client usage.
