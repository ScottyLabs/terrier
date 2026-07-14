// @ts-check

import * as starlightOpenAPI from "starlight-openapi";
import { defineConfig } from "astro/config";
import starlight from "@astrojs/starlight";

// https://astro.build/config
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
            // Each item here is one entry in the navigation menu.
            { label: "Example Guide", slug: "guides/example" },
          ],
          label: "Guides",
        },
        {
          autogenerate: { directory: "reference" },
          label: "Reference",
        },
        ...starlightOpenAPI.openAPISidebarGroups,
      ],
      social: [
        {
          href: "https://codeberg.org/ScottyLabs/terrier",
          icon: "codeberg",
          label: "Codeberg",
        },
      ],
      title: "Terrier Docs",
    }),
  ],
});
