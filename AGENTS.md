# Repository guide

Terrier is a hackathon management platform. These instructions apply throughout the repository. Use [CONTRIBUTING.md](CONTRIBUTING.md) for contributor policy and [RFCs](rfcs/README.md) for design decisions. Read only what the task needs. RFCs describe intent; source and manifests establish current behavior.

## Where to look

| Area | Entry point and context |
| --- | --- |
| API | [terrier-server](crates/terrier-server/src/main.rs): Rust, Axum, Tokio, Utoipa OpenAPI; currently a health endpoint and optional static assets. |
| Database | [entities](crates/terrier-server/src/entities) and [migrations](crates/terrier-server/migration); PostgreSQL/SeaORM design in [RFC 0008](rfcs/0008-database-model.md). Entity files are not yet wired into the server entry point. |
| Shared Rust | [terrier-common](crates/terrier-common/src/lib.rs): shared runtime helpers. |
| Authorization | [SLAC](crates/slac/README.md): typed policy extractors; keep this crate independent of Terrier domain types. |
| SSO | [SAML proxy](crates/saml-proxy/README.md); currently absent from root workspace members, so workspace checks do not cover it. |
| Frontend | [app](app/package.json): Svelte 5, TypeScript, Vite, Deno; API client and generated types in `app/src/lib/api/`. |
| Documentation | [sites/docs](sites/docs/package.json): Astro/Starlight content in `sites/docs/src/content/docs/`. Legacy mdBook files and the flake's `buildMdbook` configuration also remain; check the relevant build path. |
| Development/deployment | [devenv.nix](devenv.nix), [devenv.yaml](devenv.yaml), [flake.nix](flake.nix), and [examples](example). The ScottyLabs module supplies shared tooling and services. Deployment routing is declared in `devenv.nix`. |
| RFC search | [fetch](crates/fetch/src/main.rs); usage and RFC creation in the [RFC guide](rfcs/README.md). |

## Environment and commands

Use `devenv shell` from the root, or `devenv shell -- <command>`. See [dev shell setup](CONTRIBUTING.md#dev-shell) for activation and cache trust. Declare tools in `devenv.nix`. Shell entry installs frozen Deno dependencies and syncs Astro. Use `devenv up` when configured services (PostgreSQL, Valkey, Garage) are needed.

Run these inside the development shell, from the indicated directory:

| Purpose | Directory | Command |
| --- | --- | --- |
| API server | root | `cargo run -p terrier-server` |
| Focused Rust tests | root | `cargo test -p <package> [test_filter]` |
| Workspace tests/lints | root | `cargo test --workspace`; `cargo clippy --workspace --all-targets` |
| Frontend dev/check/build | `app` | `deno task dev`; `deno task check`; `deno task build` |
| Frontend/docs lint | `app` or `sites/docs` | `deno task lint` |
| Docs dev/build | `sites/docs` | `deno task dev`; `deno task build` |
| Format changed files | root | `treefmt <changed-files>` |
| Check formatting | root | `treefmt --fail-on-change --no-cache <changed-files>` |
| Run applicable hooks | root | `pre-commit run --files <changed-files>` |

Cover behavior changes with appropriate unit/integration tests, including failure paths. Run affected tests and required component checks/hooks. For documentation-only edits, check formatting and links. Report results and blockers accurately. Hook configuration is generated; edit its Nix source, not `.pre-commit-config.yaml`.

For API contract changes, update Utoipa definitions, start the local API on port 3000, and run `deno task generate-api` in `app` and `sites/docs` as applicable. Validate the outputs and review diffs; do not hand-edit `app/src/lib/api/schema.d.ts` or `sites/docs/openapi.json`.

The `migration`, `migrate`, and `generate-entities` shell helpers currently reference stale `crates/migration` or `crates/entity` paths. Use the actual paths above and the [migrator instructions](crates/terrier-server/migration/README.md) for database work.

## Efficient context and tools

Use the dev shell's `rg` (ripgrep), `fd`, `jq`, and repository-specific `fetch`:

- Find paths with `rg --files app/src` or `fd -e rs . crates`. Include hidden configuration explicitly when relevant; skip dependencies, build output, and caches.
- Search with `rg -n 'symbol' crates/slac`; use `rg -l 'pattern' app/src` for filenames only. Read matching functions and relevant callers/tests.
- Use `fetch <keywords>` for accepted RFC sections, source lines, and excerpts; `fetch --status all <keywords>` includes other statuses. Read relevant sections before relying on excerpts. See the [RFC guide](rfcs/README.md) for creation and search options.
- Extract needed fields with `jq '.scripts' app/package.json` or `jq '.paths | keys' sites/docs/openapi.json`; avoid dumping lockfiles and generated schemas.
- Inspect `git status --short`, `git diff --stat`, then diffs for affected paths. Include untracked files in review. Batch independent searches; narrow truncated queries rather than ignoring omitted results.
- Save verbose check output to a temporary log, preserving exit status. Inspect failures and summaries without hiding diagnostics.

## Contribution and style rules

- Keep each PR focused on one change. Review generated code as carefully as handwritten code.
- Check relevant issues/RFCs and follow [claiming features](CONTRIBUTING.md#claiming-features) when coordinating work. Major features and architectural changes require an accepted RFC; small fixes and documentation improvements do not.
- Make user-facing behavior understandable to nontechnical users and update the documentation site when it changes. Record decisions and bugs in relevant RFCs/issues.
- Satisfy checks and documentation requirements before review; maintainers approve merges. Preserve linear history: rebase onto upstream rather than merging it into a feature branch.
- Create crates with `cargo init`, following workspace conventions. Use `cargo add` for Rust dependencies and Deno commands for web dependencies. Do not manually edit dependency sections or lockfiles, or add inline dependency comments.

## Code and writing quality

- Minimize code without sacrificing correctness or readability. Reuse existing helpers; avoid speculative features, unnecessary abstractions, wrappers, dependencies, and unrelated refactors. Do not shorten code into cryptic one-liners.
- Follow surrounding conventions with clear names, focused functions, and straightforward control flow. Validate at system boundaries; avoid checks for states already excluded by types or established invariants.
- Fail loudly and early, before TartanHacks. Validate startup requirements and refuse to start when unmet. Propagate runtime errors with actionable context; log unexpected failures at the handling boundary and return an explicit failure to the caller/UI. Do not log the same failure at every layer, expose secrets, or crash unrelated requests.
- Never convert errors into empty results, placeholder data, success responses, or silent defaults. Add fallback/recovery only for an explicit product requirement and a specific recoverable failure, with a visible signal and tests. Retries must be bounded, safe to repeat, and limited to transient failures. Preserve the original error when recovery fails.
- Comments are only for necessary, non-obvious constraints, rationale, or invariants. Prefer clearer code. Omit obvious narration, section banners, and boilerplate docstrings; retain useful public API contracts and license notices. Describe current behavior, without AI/chat references, change narratives, or stale bug descriptions.
- Write concise, concrete prose. No em dashes, decorative Unicode, emojis, filler, hype, canned introductions, or formulaic contrasts. Avoid stock phrases such as "delve", "leverage", and "seamlessly". Use ASCII punctuation; preserve required characters in user content, fixtures, and technical notation. In Markdown prose, use descriptive links for URLs.
- Use [conventional commits](https://www.conventionalcommits.org/) with short, specific subjects, aiming for at most 50 characters. Omit bodies for simple changes; otherwise use a few short lines for rationale or caveats. No long summaries, file inventories, or test transcripts.
- Never attribute authorship or contributions to AI tools/agents. No AI co-author trailers, signatures, badges, or attribution notices in commits, PRs, code, or documentation. Preserve human attribution, required third-party notices, and functional generated-file markers.

## Maintaining this guide

Keep `AGENTS.md` as the single agent guide. Add actionable rules for recurring mistakes; remove obsolete or duplicate guidance. Update commands and caveats when their sources change. Link to detailed designs and tutorials; keep task history and generated inventories out of this file.
