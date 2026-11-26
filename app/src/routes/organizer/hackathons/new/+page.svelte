<script lang="ts">
  import { goto } from "$app/navigation";
  import { api } from "$lib/api/client";
  import type { components } from "$lib/api/schema";
  import HackathonSettingsForm from "$lib/components/organizer/HackathonSettingsForm.svelte";
  type Settings = components["schemas"]["Settings"];

  let message = $state("");

  const blank: Settings = {
    name: "", description: "", location: "", contact_email: "",
    max_team_size: 4, max_participants: 100,
    application_deadline: "", team_formation_deadline: "", submission_deadline: "",
    application_schema: { fields: [] },
  };

  const stripPrefix = (fields?: Record<string, string>) =>
    Object.fromEntries(Object.entries(fields ?? {}).map(([k, v]) => [k.replace(/^settings\./, ""), v]));

  async function create(settings: Settings, slug: string) {
    message = "";
    const { data, error: err, response } = await api.POST("/api/hackathons", {
      body: { slug, settings },
    });
    if (data) {
      await goto(`/organizer/hackathons/${data.slug}/settings`);
      return null;
    }
    if (response.status === 422) return stripPrefix(err?.fields);
    if (response.status === 409) return { slug: "That slug is already taken." };
    message = response.status === 403
      ? "Only global admins can create hackathons."
      : "Couldn't create the hackathon. Try again.";
    return null;
  }
</script>

<h1>New hackathon</h1>
{#if message}<p role="status">{message}</p>{/if}
<HackathonSettingsForm mode="create" initial={blank} onSubmit={create} />