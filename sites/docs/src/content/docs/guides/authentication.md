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

## Logout

`POST /api/auth/logout` deletes the local session and leaves Keycloak SSO intact.
It requires the session-bound `x-csrf-token` header and rejects a foreign Origin.
Successful deletion returns 204; storage failures return 500.

The auth-status endpoint that exposes the CSRF token, application authentication
and email-verification gates, and frontend integration are not implemented yet.
