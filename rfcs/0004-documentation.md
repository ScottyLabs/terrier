# RFC 0004: Documentation

- **Status:** Accepted
- **Author(s):** @kritdass, @ap-1
- **Created:** 2026-02-14
- **Updated:** 2026-02-17

## Overview

This RFC establishes the technologies we will use to host the documentation for Terrier: Starlight with Svelte components from Terrier's `app` directory. The documentation website will be hosted at [docs.terrier.build](https://docs.terrier.build).

## Motivation

Terrier is a hackathon-agnostic platform that must be used by multiple organizations. As such, having clear documentation that thoroughly explains usage is critical for adoption. This will also lower friction for developers who are contributing to Terrier.

## Goals

- Define technologies
- Optimize developer experience
- Enhance integration with Terrier

## Non-Goals

- Defining the Terrier application itself
- Provide documentation to end-users on how to use Terrier

## Detailed Design

We're using Starlight (Astro). It's simple with many features included out of the box, like search, Markdown/MDX support, and native Svelte integration. This allows us to import our components directly from the Terrier monorepo into documentation pages, ensuring that our previews always match the actual application state.

Astro's static site generation (SSG) support guarantees a high-performance, SEO-friendly site that compiles to lightweight HTML/CSS/JS. Starlight handles many of the standard documentation website features, removing the need to reinvent the wheel.

Furthermore, we can use the [starlight-openapi](https://github.com/HiDeoo/starlight-openapi) plugin to generate documentation from our OpenAPI specification.

### Nix Build

The documentation site is packaged as a Nix derivation (`terrierDocs`) in `flake.nix`, following the same `bun2nix` pattern used for `terrierApp`. It reads its version from `sites/docs/package.json`, fetches dependencies from `sites/docs/bun.nix`, and produces a static site output via `bun run build`.

### Topics

The documentation site should broadly cover the following topics:

- Installation and setup
- Configuration options
- API reference
- Usage examples
- Troubleshooting

The documentation site should not cover the purpose of a hackathon's specific user guide, meaning it should not provide detailed instructions on how to use Terrier for participants/other end-users.

## Alternatives Considered

- **Docusaurus**: The industry standard for documentation. Rejected because it is React-based, which would prevent us from directly importing our native Svelte components without maintaining a duplicate React library.

- **VitePress**: A high-performance alternative powered by Vue. Rejected for similar reasons—the tight coupling with Vue introduces friction for our Svelte-centric team.

- **GitBook / ReadTheDocs**: Rejected as these are primarily hosted services.

- **Plain Astro**: We could build a custom docs site on core Astro, but Starlight provides the necessary sidebar, search, and i18n boilerplate for free, saving weeks of development.

## Open Questions

N/A
