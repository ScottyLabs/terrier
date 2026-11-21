<script lang="ts">
    import { goto } from "$app/navigation";
    import { untrack } from "svelte";
    import { api } from "$lib/api/client";
    import type { components } from "$lib/api/schema";
    import HackathonSettingsForm from "$lib/components/organizer/HackathonSettingsForm.svelte";
    import type { PageProps } from "./$types";
    type Settings = components["schemas"]["Settings"];

    let { data }: PageProps = $props();
    // Local copy so saves and reloads can replace it.
    let hackathon = $state(untrack(() => data.hackathon));
    let message = $state("");
    let conflict = $state(false);

  const stripPrefix = (fields?: Record<string, string>) =>
    Object.fromEntries(Object.entries(fields ?? {}).map(([k, v]) => [k.replace(/^settings\./, ""), v]));

  async function save(settings: Settings) {
    message = "";
    conflict = false;
    const { data: saved, error: err, response } = await api.PATCH("/api/hackathons/{slug}", {
      params: { path: { slug: hackathon.slug } },
      body: { version: hackathon.version, settings },
    });
    if (saved) {
      hackathon = saved;
      message = "Saved.";
      return null;
    }
    if (response.status === 422) return stripPrefix(err?.fields);
    if (response.status === 409) {
      conflict = true;
      message = err?.message ?? "This hackathon changed. Reload before saving.";
      return null;
    }
    message = response.status === 403
      ? "You don't have permission to edit this hackathon."
      : "Couldn't save. Try again.";
    return null;
  }

  async function reload() {
    const { data: fresh } = await api.GET("/api/hackathons/{slug}", {
      params: { path: { slug: hackathon.slug } },
    });
    if (fresh) {
      hackathon = fresh;
      conflict = false;
      message = "Reloaded. Your unsaved edits were discarded.";
    }
  }

  async function deactivate() {
    if (!confirm("Deactivate this hackathon? This hides it and can't be undone from this screen.")) return;
    const { data: saved, error: err } = await api.PATCH("/api/hackathons/{slug}", {
      params: { path: { slug: hackathon.slug } },
      body: { version: hackathon.version, is_active: false },
    });
    if (saved) await goto("/");
    else {
      conflict = true;
      message = err?.message ?? "Couldn't deactivate. Try again.";
    }
  }
</script>

<h1>{hackathon.settings.name} settings</h1>
{#if message}<p role="status">{message}</p>{/if}
{#if conflict}<button type="button" onclick={reload}>Reload</button>{/if}

{#key hackathon.version}
  <HackathonSettingsForm mode="edit" initial={hackathon.settings} onSubmit={save} />
{/key}

<button type="button" onclick={deactivate}>Deactivate hackathon</button>