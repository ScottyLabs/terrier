# saml-proxy

A SAML-to-SAML proxy that lets Service Providers authenticate users from any [InCommon](https://www.incommon.org/) university without registering individually with each institution. The proxy presents itself as an IdP to downstream SPs and as an SP to upstream university IdPs.

See [RFC 0005](../../rfcs/0005-saml-proxy-university-auth.md) for the full design.

## Configuration

| Variable | Required | Description |
|---|---|---|
| `SAML_PROXY_BASE_URL` | yes | Public base URL (e.g. `https://auth.terrier.build`) |
| `SAML_PROXY_ENTITY_ID` | yes | SAML entity ID (e.g. `https://auth.terrier.build/saml/idp`) |
| `SAML_PROXY_IDP_CERT_PATH` | yes | Path to the IdP signing certificate (PEM) |
| `SAML_PROXY_IDP_KEY_PATH` | yes | Path to the IdP signing private key (PEM) |
| `SAML_PROXY_HOST` | no | Bind address (default `0.0.0.0`) |
| `SAML_PROXY_PORT` | no | Bind port (default `8443`) |

## Endpoints

**IdP interface** (for Service Providers):

- `GET /saml/metadata` -- IdP metadata XML
- `GET /saml/sso` -- SSO (HTTP-Redirect binding)
- `POST /saml/sso` -- SSO (HTTP-POST binding)
- `POST /saml/slo` -- Single Logout

**SP interface** (for university IdPs):

- `GET /sp/metadata` -- SP metadata XML
- `GET /sp/initiate?session={id}` -- Fetch IdP metadata via MDQ, redirect to university
- `POST /sp/acs` -- Assertion Consumer Service
- `POST /sp/slo` -- Single Logout

**Discovery**:

- `GET /discovery?session={id}` -- University selection UI
- `POST /discovery` -- Process selection
- `GET /api/entities/search?q={query}` -- Search InCommon entities by name

## Testing

Run the unit and integration tests:

```
cargo test -p saml-proxy
```

To manually test the discovery UI with the real InCommon federation index:

```
cargo test -p saml-proxy -- --ignored manual_discovery_ui --nocapture
```

This starts a local server, fetches the full InCommon entity list, and prints a URL you can open in a browser to interact with the search and selection flow. Press Enter to stop the server.
