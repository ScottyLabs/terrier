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

export function themeStyle(theme: Theme): string {
  return Object.entries(theme.vars)
    .map(([name, value]) => `${name}: ${value}`)
    .join("; ");
}
