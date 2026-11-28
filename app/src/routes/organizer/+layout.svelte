<script lang="ts">
  import type { Snippet } from "svelte";
  import { page } from "$app/state";
  import { scottylabsTheme, themeStyle } from "$lib/themes";
  import scottylabsDog from "$lib/assets/scottylabs-dog-only.svg";

  let { children }: { children: Snippet } = $props();

  // Organizer pages use the ScottyLabs brand. Colors, fonts, and corner
  // radius live in src/lib/themes/index.ts; change them there.
  const theme = scottylabsTheme;

  // Tabs only show on pages for a specific hackathon (/organizer/hackathons/<slug>/...).
  const slug = $derived(page.params.slug);
  const base = $derived(`/organizer/hackathons/${slug}`);
  const tabs = $derived([
    { label: "Overview", href: `${base}/`, exact: true },
    { label: "Settings", href: `${base}/settings/`, exact: false },
    { label: "Application form", href: `${base}/form/`, exact: false },
  ]);

  function isActive(href: string, exact: boolean): boolean {
    const path = page.url.pathname;
    return exact ? path === href : path.startsWith(href);
  }
</script>

<svelte:head>
  {#each theme.fontsHrefs ?? [] as href (href)}
    <link rel="stylesheet" href={href} />
  {/each}
</svelte:head>

<div class="organizer" style={themeStyle(theme)}>
  <div class="container">
    <header class="topbar">
      <div class="brand-row">
        <p class="brand">TERRIER <span class="slash">/</span> ORGANIZER</p>
        <img class="mascot" src={scottylabsDog} alt="" />
      </div>
      {#if slug}
        <nav aria-label="Organizer pages">
          {#each tabs as tab (tab.href)}
            <a href={tab.href} aria-current={isActive(tab.href, tab.exact) ? "page" : undefined}>
              {tab.label}
            </a>
          {/each}
        </nav>
      {/if}
    </header>

    <main class="content">
      {@render children()}
    </main>
  </div>
</div>

<style>
  .organizer {
    min-height: 100vh;
    padding: 2rem;
    background: var(--bg);
    color: var(--text);
    font-family: var(--font-body);
  }

  .container {
    max-width: 67.5rem;
    margin: 0 auto;
  }

  /* Header with the ScottyLabs gradient as its bottom line. */
  .topbar {
    margin-bottom: 2rem;
    padding-bottom: 2px;
    background: var(--brand-gradient) bottom / 100% 2px no-repeat;
  }

  .brand-row {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 1rem;
    margin-bottom: 0.75rem;
  }

  .brand {
    margin: 0;
    font-family: var(--font-heading);
    font-size: 1.5rem;
    letter-spacing: 0.02em;
  }

  .slash {
    background: var(--brand-gradient);
    background-clip: text;
    color: transparent;
  }

  .mascot {
    display: block;
    height: 4rem;
    width: auto;
  }

  nav {
    display: flex;
    gap: 1.5rem;
  }

  nav a {
    padding: 0 0 0.75rem;
    color: var(--text-muted);
    text-decoration: none;
  }

  nav a:hover {
    color: var(--text);
  }

  nav a[aria-current="page"] {
    background: var(--brand-gradient) bottom / 100% 3px no-repeat;
    color: var(--text);
    font-weight: 600;
  }

  .organizer :global(h1),
  .organizer :global(h2) {
    font-family: var(--font-heading);
  }

  /* Form sections are gray rounded cards with fields in two columns. */
  .organizer :global(fieldset) {
    display: grid;
    grid-template-columns: repeat(2, minmax(0, 1fr));
    gap: 0 1.5rem;
    margin: 0 0 1.5rem;
    padding: 1.5rem 2rem 2rem;
    border: none;
    border-radius: var(--radius-lg);
    background: var(--card);
  }

  .organizer :global(fieldset.three) {
    grid-template-columns: repeat(3, minmax(0, 1fr));
  }

  .organizer :global(legend) {
    float: left;
    grid-column: 1 / -1;
    margin: 0 0 0.5rem;
    padding: 0;
    font-family: var(--font-heading);
    font-size: 1.25rem;
    font-weight: 700;
  }

  .organizer :global(.field.wide),
  .organizer :global(fieldset > p) {
    grid-column: 1 / -1;
  }

  .organizer :global(label) {
    display: block;
    margin-top: 0.75rem;
    font-weight: 500;
  }

  .organizer :global(input),
  .organizer :global(textarea) {
    box-sizing: border-box;
    display: block;
    width: 100%;
    margin-top: 0.25rem;
    padding: 0.5rem 0.75rem;
    border: 1px solid var(--border);
    border-radius: var(--radius);
    background: var(--surface);
    color: var(--text);
    font: inherit;
  }

  .organizer :global(input:focus-visible),
  .organizer :global(textarea:focus-visible),
  .organizer :global(button:focus-visible),
  .organizer :global(a:focus-visible) {
    outline: 2px solid var(--focus);
    outline-offset: 2px;
  }

  .organizer :global(button) {
    padding: 0.5rem 1.25rem;
    border: none;
    border-radius: 999px;
    background: var(--accent);
    color: var(--on-accent);
    font: inherit;
    font-weight: 600;
    cursor: pointer;
  }

  .organizer :global(button:disabled) {
    opacity: 0.5;
    cursor: not-allowed;
  }

  .organizer :global(fieldset p),
  .organizer :global([role="alert"]) {
    margin: 0.25rem 0 0;
    color: var(--error);
    font-size: 0.875rem;
  }

  .organizer :global([role="status"]) {
    color: var(--text-muted);
  }

  .organizer :global(#slug) {
    font-family: var(--font-mono);
  }

  /* Phones: one column, smaller dog. */
  @media (max-width: 40rem) {
    .organizer :global(fieldset),
    .organizer :global(fieldset.three) {
      grid-template-columns: minmax(0, 1fr);
    }

    .mascot {
      height: 3rem;
    }
  }
</style>