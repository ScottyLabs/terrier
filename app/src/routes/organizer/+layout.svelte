<script lang="ts">
  import type { Snippet } from "svelte";
  import { scottylabsTheme, themeStyle } from "$lib/themes";

  let { children }: { children: Snippet } = $props();

  // Organizer pages use the ScottyLabs brand. Colors, fonts, and corner
  // radius live in src/lib/themes/index.ts; change them there.
  const theme = scottylabsTheme;
</script>

<svelte:head>
  {#each theme.fontsHrefs ?? [] as href (href)}
    <link rel="stylesheet" href={href} />
  {/each}
</svelte:head>

<div class="organizer" style={themeStyle(theme)}>
  <main class="content">
    {@render children()}
  </main>
</div>

<style>
  .organizer {
    min-height: 100vh;
    padding: 2rem;
    background: var(--bg);
    color: var(--text);
    font-family: var(--font-body);
  }

  .content {
    max-width: 40rem;
    margin: 0 auto;
  }

  .organizer :global(h1),
  .organizer :global(h2) {
    font-family: var(--font-heading);
  }

  .organizer :global(fieldset) {
    margin: 0 0 1.5rem;
    padding: 1rem 1.25rem;
    border: 1px solid var(--border);
    border-radius: var(--radius);
  }

  .organizer :global(legend) {
    padding: 0 0.5rem;
    color: var(--accent);
    font-weight: 600;
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
  .organizer :global(button:focus-visible) {
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
</style>