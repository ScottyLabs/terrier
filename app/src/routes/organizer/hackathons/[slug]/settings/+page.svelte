<script lang="ts">
  import { goto } from "$app/navigation";
  import { onMount, untrack } from "svelte";
  import { api } from "$lib/api/client";
  import type { components } from "$lib/api/schema";
  import HackathonSettingsForm from "$lib/components/organizer/HackathonSettingsForm.svelte";
  import type { PageProps } from "./$types";
  type Settings = components["schemas"]["Settings"];

  let { data }: PageProps = $props();
  // Local copy so saves and reloads can replace it.
  let hackathon = $state(untrack(() => data.hackathon));

  // Short message at the top of the screen that disappears on its own.
  let toast = $state<{ text: string; kind: "success" | "error" } | null>(null);
  let toastTimer: ReturnType<typeof setTimeout> | undefined;

  function showToast(text: string, kind: "success" | "error"): void {
    toast = { text, kind };
    clearTimeout(toastTimer);
    toastTimer = setTimeout(() => {
      toast = null;
    }, 4000);
  }

  // Centered pop-up when the hackathon changed somewhere else.
  let conflictDialog: HTMLDialogElement | undefined = $state();
  let conflictTitle = $state("");
  let conflictMessage = $state("");

  function showConflict(title: string, text: string): void {
    conflictTitle = title;
    conflictMessage = text;
    if (!conflictDialog?.open) conflictDialog?.showModal();
  }

  // Check for newer saves when the tab regains focus and every 30 seconds.
  // Remember which version the organizer already dismissed so we don't nag.
  let dismissedVersion: number | null = null;

  async function checkForUpdates(): Promise<void> {
    if (conflictDialog?.open) return;
    const { data: latest } = await api.GET("/api/hackathons/{slug}", {
      params: { path: { slug: hackathon.slug } },
    });
    if (latest && latest.version !== hackathon.version && latest.version !== dismissedVersion) {
      dismissedVersion = latest.version;
      showConflict(
        "This hackathon was updated",
        "Someone else saved changes while you had this page open.",
      );
    }
  }

  onMount(() => {
    const checkIfVisible = () => {
      if (document.visibilityState === "visible") void checkForUpdates();
    };
    const interval = setInterval(checkIfVisible, 30_000);
    document.addEventListener("visibilitychange", checkIfVisible);
    return () => {
      clearInterval(interval);
      document.removeEventListener("visibilitychange", checkIfVisible);
    };
  });

  const stripPrefix = (fields?: Record<string, string>) =>
    Object.fromEntries(Object.entries(fields ?? {}).map(([k, v]) => [k.replace(/^settings\./, ""), v]));

  async function save(settings: Settings) {
    const { data: saved, error: err, response } = await api.PATCH("/api/hackathons/{slug}", {
      params: { path: { slug: hackathon.slug } },
      body: { version: hackathon.version, settings },
    });
    if (saved) {
      hackathon = saved;
      showToast("Hackathon saved.", "success");
      return null;
    }
    if (response.status === 422) return stripPrefix(err?.fields);
    if (response.status === 409) {
      showConflict("Couldn't save your changes", err?.message ?? "Someone else changed this hackathon.");
      return null;
    }
    showToast(
      response.status === 403 ? "You don't have permission to edit this hackathon." : "Couldn't save. Try again.",
      "error",
    );
    return null;
  }

  async function reload() {
    conflictDialog?.close();
    const { data: fresh } = await api.GET("/api/hackathons/{slug}", {
      params: { path: { slug: hackathon.slug } },
    });
    if (fresh) {
      hackathon = fresh;
      dismissedVersion = null;
      showToast("Loaded the latest version.", "success");
    } else {
      showToast("Couldn't reload. Try again.", "error");
    }
  }

  async function deactivate() {
    if (!confirm("Deactivate this hackathon? This hides it and can't be undone from this screen.")) return;
    const { data: saved, error: err, response } = await api.PATCH("/api/hackathons/{slug}", {
      params: { path: { slug: hackathon.slug } },
      body: { version: hackathon.version, is_active: false },
    });
    if (saved) {
      await goto("/");
    } else if (response.status === 409) {
      showConflict("Couldn't deactivate", err?.message ?? "Someone else changed this hackathon.");
    } else {
      showToast(err?.message ?? "Couldn't deactivate. Try again.", "error");
    }
  }
</script>

<h1>{hackathon.settings.name} settings</h1>

{#key hackathon.version}
  <HackathonSettingsForm mode="edit" initial={hackathon.settings} onSubmit={save} />
{/key}

<section class="danger-zone">
  <h2>Deactivate hackathon</h2>
  <p>Hides this hackathon from everyone. This can't be undone from this screen.</p>
  <button type="button" class="danger" onclick={deactivate}>Deactivate hackathon</button>
</section>

{#if toast}
  <div class="toast" class:error={toast.kind === "error"} role={toast.kind === "error" ? "alert" : "status"}>
    {toast.text}
  </div>
{/if}

<dialog bind:this={conflictDialog} class="modal" aria-labelledby="conflict-title">
  <h2 id="conflict-title">{conflictTitle}</h2>
  <p>{conflictMessage} Reloading shows the latest version and discards your unsaved edits.</p>
  <div class="actions">
    <button type="button" class="secondary" onclick={() => conflictDialog?.close()}>Keep editing</button>
    <button type="button" onclick={reload}>Reload</button>
  </div>
</dialog>