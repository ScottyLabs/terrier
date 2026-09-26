---
title: Search RFCs
description: Find technical decisions in Terrier's RFCs.
---

In a Terrier development shell, run `fetch` from any directory:

```bash
fetch session redis
fetch --status draft saml
fetch --status all
fetch
```

Search defaults to accepted RFCs. Use `--status draft` to select another status or `--status all` to include every status. All query words must occur in a section or its RFC title. Search ignores case and includes code blocks. Results show section links relative to the repository root and source line numbers. Omit the query to list RFCs matching the status filter, or use `--help` for usage.

Outside the development shell, run `devenv shell -- fetch session redis` from the repository root.

Search reads the RFC Markdown files directly. New and edited RFCs are available immediately.

Results include short excerpts and use color in supported terminals. Piped output is plain text. The development shell builds the Rust utility on first use.
