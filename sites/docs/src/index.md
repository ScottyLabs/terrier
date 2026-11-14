# Terrier

Terrier is a hackathon management platform. These docs cover development and the API.

- [API routes and policies](guides/api-routes.md): add a route with its access checks, tests, and docs.
- [Frontend development](guides/frontend.md): work on the Svelte app and use the API client.
- [Authentication](guides/authentication.md): configure sign-in and sessions.
- [Database](guides/database.md): find tables, write queries, and add migrations.
- [Projects](guides/projects.md): read project submissions from the frontend.
- [Authorization policies](reference/policies.md): find permission rules and policy outputs.
- [API errors](reference/api-errors.md): choose an error and handle its response.

See the [repository](https://git.cmu.dev/ScottyLabs/terrier) for source code, contributor instructions, and RFCs. Each endpoint's parameters, schemas, and responses are in the [API reference](https://api.terrier.scottylabs.org/scalar), which the server generates from its OpenAPI specification.
