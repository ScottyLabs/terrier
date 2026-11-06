// @ts-check

import * as starlightOpenAPI from "starlight-openapi";
import { defineConfig } from "astro/config";
import starlight from "@astrojs/starlight";

export default defineConfig({
  integrations: [
    starlight({
      plugins: [
        starlightOpenAPI.default([
          {
            base: "api",
            schema: "./openapi.json",
          },
        ]),
      ],
      sidebar: [
        {
          items: [
            { label: "Frontend development", slug: "guides/frontend" },
            { label: "Authentication", slug: "guides/authentication" },
            { label: "Database", slug: "guides/database" },
            { label: "API routes and policies", slug: "guides/api-routes" },
          ],
          label: "Development",
        },
        {
          autogenerate: { directory: "reference" },
          label: "Reference",
        },
        ...starlightOpenAPI.openAPISidebarGroups,
      ],
      social: [
        {
          href: "https://git.cmu.dev/ScottyLabs/terrier",
          icon: "external",
          label: "Repository",
        },
      ],
      title: "Terrier Docs",
    }),
  ],
});
