// @ts-check

import starlight from "@astrojs/starlight";
import { defineConfig } from "astro/config";
import starlightOpenAPI, { openAPISidebarGroups } from "starlight-openapi";

// https://astro.build/config
export default defineConfig({
    integrations: [
        starlight({
            plugins: [
                starlightOpenAPI([
                    {
                        base: "api",
                        schema: "./openapi.json",
                    },
                ]),
            ],
            title: "Terrier Docs",
            social: [
                {
                    icon: "codeberg",
                    label: "Codeberg",
                    href: "https://codeberg.org/ScottyLabs/terrier",
                },
            ],
            sidebar: [
                {
                    label: "Guides",
                    items: [
                        // Each item here is one entry in the navigation menu.
                        { label: "Example Guide", slug: "guides/example" },
                    ],
                },
                {
                    label: "Reference",
                    autogenerate: { directory: "reference" },
                },
                ...openAPISidebarGroups,
            ],
        }),
    ],
});
