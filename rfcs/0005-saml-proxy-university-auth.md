# RFC 0005: SAML Proxy for University Authentication

- **Status:** Accepted
- **Author(s):** @ap-1
- **Created:** 2026-02-18
- **Updated:** 2026-02-18

## Overview

This RFC proposes building a SAML-to-SAML proxy service (`crates/saml-proxy`) that enables university student authentication for Terrier and other hackathon platforms via InCommon/eduGAIN federation. The proxy sits between any SAML Service Provider (e.g., Keycloak) and university Identity Providers, dynamically fetching metadata via the MDQ protocol and providing a discovery interface for university selection.

## Motivation

Collegiate hackathons need verified student authentication to:

- Enforce eligibility rules (only current students)
- Gather accurate demographic data (university affiliation)
- Prevent registration fraud (self-reported schools, .edu email spoofing)

**Current problem:**

- Each hackathon organization joining InCommon separately is impractical (legal entity requirement, annual fees, administrative overhead)
- Existing federated auth solutions like Shibboleth are complex to deploy and don't integrate well with modern OIDC-based platforms
- Keycloak doesn't support MDQ (Metadata Query Protocol), forcing administrators to manually configure each university or download 10,000+ entity aggregates

**Our solution:**

Build shared authentication infrastructure that:

1. Registers as a single InCommon Service Provider (sponsored by CMU Computing Services)
1. Provides a SAML proxy that any hackathon platform can integrate with
1. Dynamically fetches university metadata on-demand via MDQ (no manual configuration)
1. Exposes university authentication to platforms via standard SAML

This allows Terrier and other hackathon platforms to offer "Sign in with your university" without each organization needing InCommon membership.

## Goals

- Provide federated university authentication for Terrier and other hackathon platforms
- Support any university in InCommon/eduGAIN (3,000+ institutions)
- Require zero manual configuration per university (dynamic via MDQ)
- Work as a generic SAML-to-SAML proxy (not tied to specific Service Providers)
- Deploy as a standalone service at `auth.terrier.build`
- Register as InCommon Service Provider with CMU sponsorship

## Non-Goals

- Building a full identity provider with user management, delegate to specific hackathons instead
- Supporting non-SAML authentication protocols (OAuth, LDAP, etc.)
- Implementing authorization/access control
- Replacing Keycloak (the proxy complements it)
- Supporting authentication providers outside InCommon/eduGAIN

## Detailed Design

### Architecture Overview

**Components:**

```
Service Provider (Keycloak, etc.)

↕ SAML

saml-proxy (auth.terrier.build)
  - IdP interface (receives from SPs)
  - SP interface (sends to universities)
  - Discovery UI
  - Session store
  - MDQ client

↕ SAML                  ↓ Fetch metadata

University IdPs         InCommon MDQ
(CMU, Pitt, etc.)       (mdq.incommon.org)
```

### Authentication Flow

This list describes the authentication flow between the Service Provider, saml-proxy, and the University IdP, using Terrier + ScottyLabs + our Keycloak IdP as an example:

1. Service Provider -> saml-proxy: User initiates auth via SAML

   - User clicks "Sign in" on Terrier
   - Keycloak redirects to `https://auth.terrier.build/saml/sso?SAMLRequest=...`
   - Proxy parses AuthnRequest, creates session, stores SP's ACS URL and request ID

1. saml-proxy -> User: Discovery UI for university selection

   - Proxy redirects user to `https://auth.terrier.build/discovery?session={id}`
   - User sees HTML form with search box
   - User types "carnegie" → JavaScript filters/searches entities
   - User selects their university and submits

1. saml-proxy -> InCommon MDQ: Fetch selected university metadata

   - Proxy uses `saml-mdq` crate to fetch university IdP metadata
   - Request: `GET https://mdq.incommon.org/entities/{percent-encoded entityID}`
   - Response: SAML EntityDescriptor with SSO endpoints
   - Metadata cached in-memory via `saml-mdq`'s built-in LRU cache (1,000 entries, 1 hour TTL)

1. saml-proxy -> University IdP: Forward auth request via SAML

   - Proxy builds new AuthnRequest for selected university
   - Redirects user to university's SSO endpoint: `https://idp.mit.edu/sso?SAMLRequest=...`
   - RelayState contains proxy session ID

1. University IdP -> saml-proxy: Return auth response via SAML

   - Student authenticates with university credentials
   - University IdP posts SAML Response to `https://auth.terrier.build/sp/acs`

1. saml-proxy -> Service Provider: Transform and return response via SAML

   - Proxy validates SAML Response signature
   - Extracts assertions (eduPersonPrincipalName, email, displayName, etc.)
   - Builds new SAML Response for original Service Provider
   - Auto-submits form posting to SP's ACS URL (from session)

1. Service Provider: Finishes the authentication flow

   - Keycloak receives SAML Response, validates signature
   - Creates user session
   - User is authenticated to Terrier

### Technology Stack

**Why samael fork:**

The upstream samael crate has a bug deserializing `<ContactPerson>` elements from InCommon metadata. When both `contactType` and `remd:contactType` attributes are present, quick-xml strips namespace prefixes during deserialization, causing duplicate field errors.

Our fork ([ap-1/samael](https://github.com/ap-1/samael/tree/fix/contact-person-deserialization)) implements a custom deserializer that handles duplicate keys by keeping the first value. This prevents crashes when fetching real-world InCommon metadata.

Since crates.io doesn't allow git dependencies, `saml-proxy` will remain git-only until either: (1) the fix is merged upstream, or (2) we fork samael permanently as a published crate.

**Why saml-mdq separation:**

The MDQ client is a separate crate because:

- MDQ protocol is not SAML-specific (can serve any metadata format)
- MDQ = fetching, samael = parsing/signing, saml-proxy = routing
- Reusable for other projects needing InCommon/eduGAIN integration
- Follows Rust ecosystem pattern of small, composable crates

### Configuration

The proxy is configured via environment variables:

**Required:**

- `SAML_PROXY_BASE_URL` - Public URL of the proxy (e.g., `https://auth.terrier.build`)
- `SAML_PROXY_ENTITY_ID` - SAML entity ID (e.g., `https://auth.terrier.build/saml/idp`)
- `SAML_PROXY_IDP_CERT_PATH` - Path to proxy's signing certificate
- `SAML_PROXY_IDP_KEY_PATH` - Path to proxy's private key

**Optional:**

- `SAML_PROXY_PORT` - Listen port (default: `8443`)
- `SAML_PROXY_HOST` - Listen address (default: `0.0.0.0`)

**Hardcoded:**

- InCommon MDQ base URL: `https://mdq.incommon.org`
- InCommon MDQ aggregate URL: `https://mdq.incommon.org/entities` (for federation index)
- InCommon MDQ signing certificate: embedded at compile time from `certs/incommon-mdq.pem`
- Federation index refresh interval: 6 hours

The InCommon MDQ signing certificate is embedded into the binary at compile time from `crates/saml-proxy/certs/incommon-mdq.pem`. It can be obtained from [here](https://spaces.at.internet2.edu/display/MDQ/production-mdq-signing-key).

### Session Management

Sessions use an in-memory DashMap (thread-safe HashMap) with the following structure:

```rust
pub struct AuthSession {
    pub relay_state: Option<String>,          // From original SP
    pub original_request_id: String,          // SP's AuthnRequest ID
    pub sp_acs_url: String,                   // Where to POST response
    pub sp_entity_id: String,                 // Original SP's entity ID
    pub selected_university: Option<String>,  // User's choice
    pub proxy_request_id: Option<String>,     // AuthnRequest ID sent to university
    pub created_at: DateTime<Utc>,            // For cleanup
}
```

The session ID is the DashMap key (a cryptographically random UUID), not stored in the struct itself. The `proxy_request_id` is set when the proxy sends an AuthnRequest to the university IdP and is used to validate `InResponseTo` in the university's SAML Response.

Sessions expire after 15 minutes (standard SAML timeout). A background task runs every 5 minutes to clean expired sessions.

**Why in-memory vs Redis:**

- Simplifies deployment (no external dependency)
- Sessions are short-lived (15 min max)
- Loss on restart is acceptable (user re-initiates auth)
- Can migrate to Redis later if multi-instance deployment needed

### Discovery UI

The discovery interface provides server-side search across all InCommon/eduGAIN entities:

**Search endpoint:** `GET /api/entities/search?q=carnegie`

- Searches organization display names (case-insensitive substring match)
- Returns top 20 matches as JSON
- Backed by an in-memory federation index

**Federation index:**

On startup, the proxy fetches the full InCommon aggregate from `https://mdq.incommon.org/entities`, splits the XML by `<EntityDescriptor>` boundaries, filters to IdP-only entries (those containing `IDPSSODescriptor`), and extracts each entity's ID and `OrganizationDisplayName`. This builds an in-memory index of ~6,000 IdPs. A background task refreshes the index every 6 hours.

**Client-side flow:**

- User types in search box
- JavaScript debounces input (300ms), sends query to `/api/entities/search`
- Backend filters the in-memory index
- Returns matching results to frontend
- User selects their university from results

### Attribute Mapping

The proxy extracts standard eduPerson attributes from university assertions:

- `eduPersonPrincipalName` → unique persistent ID
- `eduPersonScopedAffiliation` → student/faculty/staff
- `mail` → email address
- `displayName` → full name
- `eduPersonAffiliation` → unscoped affiliation

These are passed through to the Service Provider in the proxy's SAML Response. The SP (e.g., Keycloak) maps them to user attributes according to its own configuration.

### Endpoints

**IdP interface (for Service Providers):**

- `GET /saml/metadata` - IdP metadata XML
- `GET /saml/sso` - SSO endpoint (HTTP-Redirect binding)
- `POST /saml/sso` - SSO endpoint (HTTP-POST binding)

**Discovery:**

- `GET /discovery?session={id}` - University selection form
- `POST /discovery` - Process selection, redirect to SP initiation
- `GET /api/entities/search?q={query}` - Search InCommon entities by display name

**SP interface (for universities):**

- `GET /sp/initiate?session={id}` - Fetch metadata via MDQ, redirect to university
- `POST /sp/acs` - Assertion Consumer Service, receive university response

### Deployment

**Domain:** `auth.terrier.build`

**Build output:**

The proxy builds to a standalone binary: `saml-proxy`. This binary is deployed via NixOS module on ScottyLabs infrastructure.

**systemd service:**

```nix
# NixOS configuration
systemd.services.saml-proxy = {
  description = "SAML Proxy for University Authentication";
  wantedBy = [ "multi-user.target" ];
  after = [ "network.target" ];

  serviceConfig = {
    Type = "simple";
    ExecStart = "${pkgs.saml-proxy}/bin/saml-proxy";
    Restart = "always";
    RestartSec = "10s";
    
    # Security hardening
    DynamicUser = true;
    PrivateTmp = true;
    ProtectSystem = "strict";
    ProtectHome = true;
    NoNewPrivileges = true;
    
    # Configuration
    EnvironmentFile = config.age.secrets.saml-proxy-env.path;
    LoadCredential = [
      "idp.crt:${config.age.secrets.saml-proxy-cert.path}"
      "idp.key:${config.age.secrets.saml-proxy-key.path}"
    ];
  };
};
```

**Environment file** (`/run/agenix/saml-proxy-env`):

```bash
SAML_PROXY_BASE_URL=https://auth.terrier.build
SAML_PROXY_ENTITY_ID=https://auth.terrier.build/saml/idp
SAML_PROXY_IDP_CERT_PATH=$CREDENTIALS_DIRECTORY/idp.crt
SAML_PROXY_IDP_KEY_PATH=$CREDENTIALS_DIRECTORY/idp.key
SAML_PROXY_PORT=8443
```

Certificates are loaded via systemd's `LoadCredential` mechanism into a temporary directory that only the service can access. The `$CREDENTIALS_DIRECTORY` environment variable points to this directory.

**Logging:**

Structured JSON logs to stdout (captured by journald):

```rust
tracing_subscriber::registry()
    .with(tracing_subscriber::EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| "info".into()))
    .with(tracing_subscriber::fmt::layer().json())
    .init();
```

Logs include:

- Authentication attempts (session ID, selected university)
- MDQ fetches (entity ID, cache hit/miss, duration)
- SAML validation results (signature verification, timestamp checks)
- Errors with context

### InCommon Registration

**Entity ID:** `https://auth.terrier.build/saml/idp`

**Sponsorship:** CMU Computing Services will register the proxy under CMU's existing InCommon membership, without the need for a separate legal entity.

**Metadata submission:**

1. Generate SP metadata XML via `/saml/metadata` endpoint
1. Submit to Computing Services for them to submit via InCommon Federation Manager
1. Potentially request Research & Scholarship (R&S) category (for more attributes) and eduGAIN (for international universities)

### Security Considerations

**SAML Response validation:**

- Verify XML signatures using samael's xmlsec integration
- Check NotBefore/NotOnOrAfter timestamps
- Validate Audience (must match our entity ID)
- Validate InResponseTo (must match our request ID)
- Reject unsigned assertions

**Session security:**

- Session IDs are cryptographically random UUIDs passed as URL query parameters
- Short session lifetime (15 min)
- HTTPS-only in production (via reverse proxy)

**MDQ security:**

- Cache metadata to limit exposure to MDQ service availability
- InCommon MDQ signing certificate embedded at compile time
- Verify InCommon's XML signature on metadata responses
- Pin InCommon's signing certificate

**Rate limiting:**

- Per-IP rate limits via nginx (10 req/sec)
- Per-session rate limits (max 3 discovery attempts)

## Alternatives Considered

### SAML-to-OIDC Proxy

We initially considered building a SAML-to-OIDC proxy that would expose university authentication as OAuth/OIDC endpoints. This would eliminate the need for Keycloak to speak SAML.

**Rejected because:**

- Requires implementing full OIDC provider (token endpoint, userinfo, JWKS, discovery)
- Keycloak already handles OIDC, sessions, user management
- Significantly more complex than SAML-to-SAML proxying
- We'd be reimplementing Keycloak's IdP functionality

SAML-to-SAML keeps the proxy focused: it only handles federation complexity (MDQ + multi-IdP), delegating everything else to Keycloak.

### Keycloak Identity Brokering

Keycloak has built-in SAML identity provider brokering. We could configure Keycloak directly with university IdPs.

**Rejected because:**

- No MDQ support (must manually configure each university)
- Requires downloading 10,000+ entity aggregate or manual per-university setup
- No dynamic discovery UI (users must know their entityID)
- Aggregate files are massive and require periodic updates
- Doesn't scale to InCommon's size

The MDQ protocol exists specifically to solve this problem.

### SimpleSAMLphp

SimpleSAMLphp is a mature SAML implementation with MDQ support via modules.

**Rejected because:**

- PHP introduces another language to the stack
- Doesn't integrate with our Nix-based infrastructure
- Deployment complexity (Apache/nginx + PHP-FPM)
- Want to leverage Rust ecosystem and team expertise

### Shibboleth Service Provider

Shibboleth SP is the reference InCommon implementation.

**Rejected because:**

- Java-based, heavy deployment
- Designed for protecting web apps, not acting as a proxy
- Complex configuration (XML files, attribute mapping rules)
- Not designed for multi-IdP discovery

### Wait for Keycloak MDQ Support

We could wait for Keycloak to implement MDQ support in [#43737](https://github.com/keycloak/keycloak/issues/43737).

**Rejected because:**

- Has no timeline for implementation
- Blocks Terrier's authentication requirements

Building the proxy now unblocks development and provides value to the broader hackathon community immediately.

## Open Questions

1. **Attribute release:** Which eduPerson attributes do we request by default? Should this be configurable per SP, or hardcoded? InCommon's R&S bundle includes: eduPersonPrincipalName, eduPersonScopedAffiliation, mail, displayName, givenName, sn.

1. **Monitoring depth:** What metrics beyond request latency, MDQ cache hit rate, and error rate should we track? Per-university auth success rate? Discovery abandonment rate?

1. **Discovery UX enhancement:** Should we add university logos to discovery results? If so, how do we source and maintain logo assets for 3,000+ institutions?

## Implementation Phases

### Phase 1: Implementation

- Set up crates/saml-proxy/ structure and dependencies
- Session management, IdP metadata generation, SSO endpoint
- Discovery UI with client-side search and entity list endpoint
- SP initiation (MDQ integration, AuthnRequest building)
- ACS endpoint (response parsing, validation, transformation)
- End-to-end flow working locally

### Phase 2: InCommon Registration

- Generate production certificates
- Create InCommon metadata and privacy policy
- Submit to CMU Computing Services for registration
- Request R&S category and eduGAIN export
- Test with CMU Shibboleth

### Phase 3: Production Deployment

- Deploy to auth.terrier.build via NixOS module
- Configure nginx reverse proxy and monitoring
- End-to-end testing with real universities
