<script lang="ts">
  import { untrack } from "svelte";
  import { toLocalInput, fromLocalInput, browserTimeZone } from "$lib/datetime";
  import type { components } from "$lib/api/schema";
  type Settings = components["schemas"]["Settings"];

  let { mode, initial, onSubmit }: {
    mode: "create" | "edit";
    initial: Settings;
    // returns field errors keyed like "name", or null on success
    onSubmit: (settings: Settings, slug: string) => Promise<Record<string, string> | null>;
  } = $props();

  let slug = $state("");
  // Copy the starting values once; the edit page remounts this form via {#key} after saves/reloads.
  let form = $state(untrack(() => ({
    ...initial,
    application_deadline: toLocalInput(initial.application_deadline),
    team_formation_deadline: toLocalInput(initial.team_formation_deadline),
    submission_deadline: toLocalInput(initial.submission_deadline),
  })));
  let errors = $state<Record<string, string>>({});
  let saving = $state(false);

  function validate(): Record<string, string> {
    const e: Record<string, string> = {};
    for (const f of ["name", "description", "location", "contact_email"] as const) {
      const v = form[f].trim();
      if (!v || v.length > 255) e[f] = "Use 1 to 255 characters.";
    }
    if (mode === "create" && !/^[a-z0-9-]+$/.test(slug)) e.slug = "Use lowercase letters, numbers, and hyphens.";
    if (form.max_team_size <= 0 || form.max_team_size > form.max_participants)
      e.max_team_size = "Use a positive size that fits within the participant limit.";
    const [a, t, s] = [form.application_deadline, form.team_formation_deadline, form.submission_deadline];
    if (!a || !t || !s) e.deadlines = "Set all three deadlines.";
    else if (new Date(a) > new Date(t) || new Date(t) > new Date(s))
      e.deadlines = "Set the application, team formation, and submission deadlines in order.";
    return e;
  }

  async function submit(event: SubmitEvent) {
    event.preventDefault();
    errors = validate();
    if (Object.keys(errors).length) return;
    saving = true;
    const result = await onSubmit({
      ...form,
      application_deadline: fromLocalInput(form.application_deadline),
      team_formation_deadline: fromLocalInput(form.team_formation_deadline),
      submission_deadline: fromLocalInput(form.submission_deadline),
      application_schema: initial.application_schema, // pass through untouched
    }, slug);
    saving = false;
    errors = result ?? {};
  }
</script>

<form onsubmit={submit} novalidate>
  <fieldset>
    <legend>Basics</legend>
    {#if mode === "create"}
      <label for="slug">Slug</label>
      <input id="slug" bind:value={slug} />
      {#if errors.slug}<p>{errors.slug}</p>{/if}
    {/if}
    <label for="name">Name</label>
    <input id="name" bind:value={form.name} />
    {#if errors.name}<p>{errors.name}</p>{/if}

    <label for="description">Description ({form.description.length}/255)</label>
    <textarea id="description" bind:value={form.description} maxlength="255"></textarea>
    {#if errors.description}<p>{errors.description}</p>{/if}

    <label for="location">Location</label>
    <input id="location" bind:value={form.location} />
    {#if errors.location}<p>{errors.location}</p>{/if}

    <label for="contact_email">Contact email</label>
    <input id="contact_email" type="email" bind:value={form.contact_email} />
    {#if errors.contact_email}<p>{errors.contact_email}</p>{/if}
  </fieldset>

  <fieldset>
    <legend>Limits</legend>
    <label for="max_team_size">Max team size</label>
    <input id="max_team_size" type="number" min="1" bind:value={form.max_team_size} />
    {#if errors.max_team_size}<p>{errors.max_team_size}</p>{/if}

    <label for="max_participants">Max participants</label>
    <input id="max_participants" type="number" min="1" bind:value={form.max_participants} />
    {#if errors.max_participants}<p>{errors.max_participants}</p>{/if}
  </fieldset>

  <fieldset>
    <legend>Deadlines ({browserTimeZone})</legend>
    <label for="application_deadline">Applications close</label>
    <input id="application_deadline" type="datetime-local" bind:value={form.application_deadline} />
    <label for="team_formation_deadline">Teams lock</label>
    <input id="team_formation_deadline" type="datetime-local" bind:value={form.team_formation_deadline} />
    <label for="submission_deadline">Submissions close</label>
    <input id="submission_deadline" type="datetime-local" bind:value={form.submission_deadline} />
    {#if errors.deadlines || errors.application_deadline}<p>{errors.deadlines ?? errors.application_deadline}</p>{/if}
  </fieldset>

  <button type="submit" disabled={saving}>{saving ? "Saving..." : mode === "create" ? "Create" : "Save"}</button>
</form>