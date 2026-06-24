# Terrier

Open-source hackathon management platform for universities and organizations.

## Features

- **Registration and team management.** Customizable application forms, team formation, and attendee management.
- **Live judging.** Real-time expo-style judging with support for multiple scoring systems.
- **Multiple distribution methods.** Docker(-compose), Nix flakes, and standalone binaries are supported.
- **Enterprise SSO.** OIDC and SAML support for institutional authentication, [available to everyone](https://ssotax.org/).
- **Mobile app.** Native iOS and Android app for attendees, organizers, and judges.
- **Documentation.** Comprehensive documentation site with deployment guides and usage instructions.
- **AI-enabled.** MCP server integration and *tasteful* AI features for quality-of-life improvements.
- **Self-hosted.** You have full control over your data and infrastructure.

## Canonical Deployment Domains

Terrier uses the following production custom domains:

| Component | Kennel key | Domain |
| --- | --- | --- |
| Frontend site | N/A | `terrier.scottylabs.org` |
| API service | `scottylabs.kennel.services.terrier` | `api.terrier.scottylabs.org` |
| Documentation site | `scottylabs.kennel.sites.docs` | `docs.terrier.build` |

The API and documentation values are declared in `devenv.nix` and should be treated as the source of truth for deployment routing.

## Maintainers

Developed and maintained by [ScottyLabs](https://scottylabs.org) at Carnegie Mellon University.
