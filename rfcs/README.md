# Terrier RFCs

This directory contains Request for Comments (RFC) documents that guide the technical design and architecture of Terrier.

## What are RFCs?

RFCs are design documents that propose and document significant technical decisions. They provide a consistent and controlled path for new features and architectural changes to enter the project.

## When to write an RFC

Write an RFC when you want to propose:

- Major architectural decisions
- New features that affect multiple parts of the system
- Breaking changes to existing APIs or data models
- Changes to development processes or tooling

Small bug fixes, documentation improvements, and minor features don't need RFCs.

## RFC Process

1. **Draft** - Author creates RFC document and opens a pull request
1. **Review** - Team discusses in PR comments, author revises based on feedback
1. **Accepted/Rejected** - RFC is either merged (accepted) or closed (rejected)

Accepted RFCs live permanently in this directory as historical documentation of why decisions were made. When implementing an RFC, if there are any implementation details that need to be discussed or changed, the RFC should be updated to reflect those details.

## RFC Format

Each RFC should include:

```markdown
# RFC ####: Title

- **Status:** Draft | Accepted | Rejected
- **Author(s):** @username
- **Created:** YYYY-MM-DD
- **Updated:** YYYY-MM-DD

## Overview

Brief 2-3 sentence summary of the proposal.

## Motivation

Why are we doing this? What problem does it solve?

## Goals

What are we trying to achieve?

## Non-Goals

What is explicitly out of scope?

## Detailed Design

The meat of the RFC. Explain the design in enough detail that:
- Its interaction with other features is clear
- It's reasonably clear how to implement
- Corner cases are discussed

Include code examples, diagrams, or data models where helpful.

## Alternatives Considered

What other approaches were considered and why weren't they chosen?

## Open Questions

What parts of the design still need to be figured out?

## Implementation Phases

If this is a large change, how should it be broken down?
```

## Naming Convention

RFCs are numbered sequentially (zero-padded to 4 digits for consistent sorting) and use kebab-case:

```
####-short-descriptive-title.md
```

When submitting RFCs, create branches with the `rfc/` prefix, e.g. `rfc/0002-dev-environment-ci`. This makes it easy to identify RFC branches in the repository.

The filename should match the branch name, e.g. `0002-dev-environment-ci.md`.

## Search

In the development shell, search RFCs from any directory:

```bash
fetch session redis
fetch --status draft saml
fetch --status all
fetch
```

Search defaults to accepted RFCs and ignores case. Use `--status draft` to select another status or `--status all` to include every status. All query words must appear in a section or its RFC title. Results include the matching section's Markdown link (relative to the repository root) and source line. Code blocks are searchable too. With no query, the command lists RFCs matching the status filter.

Matches are ranked by relevance: RFCs whose title matches every query word are shown first, then RFCs are ordered by how many sections matched. Ties (including the "no query" listing) keep filename order.

If an RFC declares a `- **Supersedes:**` or `- **Related:**` header bullet, that relationship is printed alongside its search results, so you can see how RFCs connect without opening both files. The `- **Updated:**` date, if present, is shown next to the RFC's status. An RFC with no parseable `- **Status:**` line still doesn't appear in status-filtered searches, but `fetch` now warns about it on stderr instead of silently dropping it.

Results include short excerpts and use color in supported terminals. Piped output is plain text. The development shell builds the Rust utility on first use.

Outside the development shell, use `devenv shell -- fetch session redis` from the repository root.

The utility reads the Markdown files directly, so there is no index to rebuild. Run its tests from the repository root with `cargo test -p fetch`.

## Creating a new RFC

`fetch --new "Title"` creates a new Draft RFC and prints the path it wrote:

```bash
fetch --new "Rate Limiting"
```

This picks the next sequential RFC number, slugifies the title into the `####-kebab-case-title.md` filename described below, fills in today's date for **Created** and **Updated**, and writes out the exact template from the [RFC Format](#rfc-format) section. The format itself doesn't change. Rename the file if you want the branch name and filename to match exactly, and fill in the template's remaining sections before opening a pull request.

## Current RFCs

| Number | Title | Status |
|--------|-------|--------|
| 0001 | [Core Architecture & Tech Stack](./0001-core-architecture-tech-stack.md) | Accepted |
| 0002 | [Development Environment & CI](./0002-dev-environment-ci.md) | Accepted |
| 0003 | [Build Performance & Developer Workflow](./0003-build-performance-dev-workflow.md) | Accepted |
| 0004 | [Documentation](./0004-documentation.md) | Accepted |
| 0005 | [SAML Proxy for University Authentication](./0005-saml-proxy-university-auth.md) | Accepted |
| 0006 | [Observability](./0006-observability.md) | Accepted |
| 0007 | [Mobile Architecture](./0007-mobile-architecture.md) | Accepted |
| 0008 | [Database Model](./0008-database-model.md) | Accepted |
| 0009 | [SLAC](./0009-slac.md) | Accepted |

*(This index will be updated as RFCs are added)*

## Questions?

Open an issue or ask in the ScottyLabs [Discord](https://go.scottylabs.org/discord) or [Slack](https://go.scottylabs.org/slack).
