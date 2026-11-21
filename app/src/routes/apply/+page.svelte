<script lang="ts">
  import { onMount } from "svelte";
  import { api } from "$lib/api/client";
  import type { components } from "$lib/api/schema";
  import { loadSession, loginUrl, type Session } from "$lib/auth";
  import FormRenderer from "$lib/forms/FormRenderer.svelte";
  import type { Answers, FieldErrors } from "$lib/forms/types";

  type Application = components["schemas"]["Application"];
  type Hackathon = components["schemas"]["Hackathon"];

  const OFFLINE =
    "Terrier could not be reached. Your answers are still on this page; try again when you are back online.";

  let loading = $state(true);
  let problem = $state<string | null>(null);
  let slug = $state("");
  let session = $state<Session | null>(null);
  let signedOut = $state(false);
  let hackathon = $state<Hackathon | null>(null);
  let application = $state<Application | null>(null);
  let answers = $state<Answers>({});
  let errors = $state<FieldErrors>({});
  let notice = $state<string | null>(null);
  let saveState = $state<"saved" | "unsaved" | "saving" | "failed">("saved");
  let submitting = $state(false);

  let saved = "";
  let timer: ReturnType<typeof setTimeout> | undefined;
  let queue: Promise<boolean> = Promise.resolve(true);

  const decided = $derived(
    application?.status != null && application.status !== "pending",
  );
  const closed = $derived(
    hackathon !== null &&
      new Date(hackathon.settings.application_deadline) <= new Date(),
  );
  const editable = $derived(!decided && !closed);
  const unsubmittedChanges = $derived(
    application?.status === "pending" &&
      canonical(answers) !== canonical(application.data),
  );

  /** JSON with sorted keys, because the database does not keep answer order. */
  function canonical(value: unknown): string {
    const answers = (value ?? {}) as Answers;
    return JSON.stringify(
      Object.keys(answers).toSorted().map((key) => [key, answers[key]]),
    );
  }

  function date(value: string | null | undefined): string {
    return value ? new Date(value).toLocaleString() : "";
  }

  function fieldErrors(fields: Record<string, string> | undefined): FieldErrors {
    return Object.fromEntries(
      Object.entries(fields ?? {})
        .filter(([key]) => key.startsWith("data."))
        .map(([key, message]) => [key.slice("data.".length), message]),
    );
  }

  onMount(() => {
    void load();
    const leave = (event: BeforeUnloadEvent) => {
      if (saveState !== "saved") {
        event.preventDefault();
      }
    };
    const hide = () => {
      if (document.visibilityState === "hidden" && saveState === "unsaved") {
        void persist(false);
      }
    };
    addEventListener("beforeunload", leave);
    document.addEventListener("visibilitychange", hide);
    return () => {
      clearTimeout(timer);
      removeEventListener("beforeunload", leave);
      document.removeEventListener("visibilitychange", hide);
    };
  });

  async function load() {
    slug = new URLSearchParams(location.search).get("hackathon") ?? "";
    if (!slug) {
      problem =
        "This link does not say which hackathon to apply to. Open the application link from the hackathon's page.";
      loading = false;
      return;
    }
    try {
      session = await loadSession();
      if (!session) {
        signedOut = true;
        return;
      }
      const found = await api.GET("/api/hackathons/{slug}", {
        params: { path: { slug } },
      });
      if (found.error) {
        problem = found.response.status === 404
          ? "This hackathon does not exist or is no longer active."
          : found.error.message;
        return;
      }
      hackathon = found.data;
      const mine = await api.GET(
        "/api/hackathons/{slug}/applications/{user_id}",
        { params: { path: { slug, user_id: session.user.id } } },
      );
      if (mine.error && mine.response.status !== 404) {
        problem = mine.error.message;
        return;
      }
      application = mine.data ?? null;
      // Questions removed from the form since the draft was saved would be rejected on save.
      const questions = new Set(
        (found.data.settings.application_schema.fields ?? []).map(
          (field) => field.id,
        ),
      );
      const stored = (application?.draft_data ?? application?.data ??
        {}) as Answers;
      answers = Object.fromEntries(
        Object.entries(stored).filter(([key]) => questions.has(key)),
      );
      saved = canonical(answers);
    } catch {
      problem = OFFLINE;
    } finally {
      loading = false;
    }
  }

  $effect(() => {
    const current = canonical(answers);
    if (loading || !editable || current === saved) {
      return;
    }
    saveState = "unsaved";
    clearTimeout(timer);
    timer = setTimeout(() => void persist(false), 800);
  });

  /** Sends saves one at a time, so a slow response cannot overwrite newer answers. */
  function persist(submit: boolean): Promise<boolean> {
    clearTimeout(timer);
    queue = queue.then(() => send(submit));
    return queue;
  }

  async function send(submit: boolean): Promise<boolean> {
    if (!session) {
      return false;
    }
    const body = $state.snapshot(answers) as Answers;
    const sent = canonical(body);
    if (!submit && sent === saved) {
      saveState = "saved";
      return true;
    }
    saveState = "saving";
    try {
      const result = await api.PUT(
        "/api/hackathons/{slug}/applications/{user_id}",
        {
          params: { path: { slug, user_id: session.user.id } },
          // The generated type for free-form answers is `Record<string, never>`.
          body: { data: body as Record<string, never>, submit },
          headers: { "x-csrf-token": session.csrfToken },
        },
      );
      if (result.error) {
        const status = result.response.status;
        errors = fieldErrors(result.error.fields);
        if (status === 401) {
          signedOut = true;
          notice = "You were signed out. Sign in again to keep saving.";
        } else if (status === 422) {
          notice = submit
            ? "Some answers need attention before you can submit."
            : "Some answers could not be saved. Fix the highlighted questions.";
        } else {
          notice = result.error.message;
        }
        if (submit && status === 422) {
          // Nothing was saved, so keep the draft current.
          saveState = sent === saved ? "saved" : "unsaved";
          if (saveState === "unsaved") {
            void persist(false);
          }
        } else {
          saveState = "failed";
        }
        return false;
      }
      application = result.data;
      saved = sent;
      errors = {};
      notice = null;
      saveState = canonical($state.snapshot(answers)) === sent
        ? "saved"
        : "unsaved";
      return true;
    } catch {
      saveState = "failed";
      notice = OFFLINE;
      return false;
    }
  }

  async function submit() {
    submitting = true;
    await persist(true);
    submitting = false;
  }
</script>

<main>
  {#if loading}
    <p role="status">Loading your application...</p>
  {:else if problem}
    <p role="alert">{problem}</p>
  {:else if signedOut && !session}
    <h1>Apply</h1>
    <p>Sign in to start or continue your application.</p>
    <p><a href={loginUrl()} rel="external">Sign in</a></p>
  {:else if hackathon}
    <h1>Apply to {hackathon.settings.name}</h1>

    <section aria-label="Application status">
      {#if !application}
        <p>You have not started this application yet.</p>
      {:else if application.status == null}
        <p>Draft: not submitted yet.</p>
      {:else if application.status === "pending"}
        <p>
          Submitted {date(application.submitted_at)}. Waiting for a decision.
        </p>
      {:else if application.status === "accepted"}
        <p>Accepted.</p>
        {#if application.attendance_status === "pending"}
          <p>
            Confirm your attendance by {date(application.confirmation_deadline)}.
          </p>
        {:else if application.attendance_status}
          <p>Attendance: {application.attendance_status}.</p>
        {/if}
      {:else if application.status === "waitlisted"}
        <p>You are on the waitlist.</p>
      {:else if application.status === "rejected"}
        <p>Your application was not accepted.</p>
      {/if}
      {#if decided}
        <p>This application has been decided and can no longer be edited.</p>
      {:else if closed}
        <p>The application deadline has passed.</p>
      {:else}
        <p>
          Applications close {date(hackathon.settings.application_deadline)}.
        </p>
      {/if}
    </section>

    {#if notice}
      <p role="alert">
        {notice}
        {#if signedOut}<a href={loginUrl()} rel="external">Sign in</a>{/if}
      </p>
    {/if}

    <form
      onsubmit={(event) => {
        event.preventDefault();
        void submit();
      }}
    >
      <FormRenderer
        form={hackathon.settings.application_schema}
        bind:answers
        {errors}
        readonly={!editable}
      />

      {#if editable}
        <p role="status">
          {#if saveState === "saving"}
            Saving...
          {:else if saveState === "unsaved"}
            Unsaved changes
          {:else if saveState === "failed"}
            Not saved.
            <button type="button" onclick={() => void persist(false)}>
              Try again
            </button>
          {:else}
            All changes saved
          {/if}
        </p>
        {#if unsubmittedChanges}
          <p>Your changes are saved as a draft. Submit again to update your application.</p>
        {/if}
        <button type="submit" disabled={submitting}>
          {application?.status === "pending"
          ? "Submit changes"
          : "Submit application"}
        </button>
      {/if}
    </form>
  {/if}
</main>
