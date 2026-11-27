export type Theme = {
  name: string;
  // Optional stylesheets that load the theme's fonts
  fontsHrefs?: string[];
  vars: Record<string, string>;
};

// ScottyLabs brand, from scottylabs.org. Used for all organizer pages.
export const scottylabsTheme: Theme = {
  name: "ScottyLabs",
  fontsHrefs: [
    "https://api.fontshare.com/v2/css?f[]=satoshi@400,500,700&display=swap",
    "https://fonts.googleapis.com/css2?family=JetBrains+Mono:wght@400;500&display=swap",
  ],
  vars: {
    "--bg": "#ffffff",
    "--surface": "#f7f7f7",
    "--text": "#000000",
    "--text-muted": "#595959",
    "--border": "#e3e3e3",
    "--accent": "#1f4ce0",
    "--on-accent": "#ffffff",
    "--focus": "#1f4ce0",
    "--error": "#d92626",
    "--radius": "10px",
    "--font-body": "Satoshi, system-ui, sans-serif",
    "--font-heading": "Satoshi, system-ui, sans-serif",
    "--font-mono": '"JetBrains Mono", ui-monospace, monospace',
    "color-scheme": "light",
  },
};

// TartanHacks 2027: kaleidoscope. Colors from the design team's Figma.
// For hacker-facing pages (registration, teams, submissions).
const tartanhacks2027: Theme = {
  name: "TartanHacks 2027",
  // Fonts are waiting on the design team; system fonts until then.
  vars: {
    "--bg": "#121212",
    "--surface": "#1c1c1c",
    "--text": "#f5f5f5",
    "--text-muted": "#a3a3a3",
    "--border": "#3a3a3a",
    "--accent": "#ff5320",
    "--on-accent": "#121212",
    "--focus": "#ffa8e4",
    "--error": "#ff2702",
    "--radius": "8px",
    "--font-body": "system-ui, sans-serif",
    "--font-heading": "system-ui, sans-serif",
    "--font-mono": "ui-monospace, monospace",
    "color-scheme": "dark",
  },
};

// Add each new hackathon's theme here, keyed by its slug.
const themesBySlug: Record<string, Theme> = {
  "tartanhacks-2027": tartanhacks2027,
};

// Hackathon theme for hacker-facing pages; falls back to ScottyLabs.
export function themeFor(slug: string | undefined): Theme {
  return (slug === undefined ? undefined : themesBySlug[slug]) ?? scottylabsTheme;
}

export function themeStyle(theme: Theme): string {
  return Object.entries(theme.vars)
    .map(([name, value]) => `${name}: ${value}`)
    .join("; ");
}
