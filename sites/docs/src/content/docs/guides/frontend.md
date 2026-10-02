---
title: Frontend development
description: Run and build the Terrier SvelteKit app.
---

Enter `devenv shell` from the repository root, then run these commands in `app`:

- `deno task dev` starts the development server.
- `deno task check` generates SvelteKit types and checks the app.
- `deno task lint` runs the frontend linter.
- `deno task build` builds the site into `dist`.
- `deno task preview` serves the production build locally.

Pages live in `src/routes`; the home page is `src/routes/+page.svelte`.
Shared components and the API client live in `src/lib`.

The app uses SvelteKit's [static adapter](https://svelte.dev/docs/kit/adapter-static).
The root layout enables prerendering, so pages are rendered at build time and
served as static files by the existing deployment. Runtime data comes from the
Rust API. Server routes and form actions that require a running SvelteKit server
are not supported by this deployment.
