<script lang="ts">
  import type { Field, Upload } from "./types";

  let {
    field,
    value = $bindable(),
    error,
    readonly = false,
    upload,
  }: {
    field: Field;
    value: unknown;
    error?: string;
    readonly?: boolean;
    upload?: Upload;
  } = $props();

  const id = $derived(`field-${field.id}`);
  const errorId = $derived(`${id}-error`);
  const text = $derived(typeof value === "string" ? value : "");
  const chosen = $derived(Array.isArray(value) ? value : []);

  let uploading = $state(false);
  let uploadError = $state<string | null>(null);

  async function chooseFile(input: HTMLInputElement) {
    const file = input.files?.[0];
    if (!file || !upload) {
      return;
    }
    uploading = true;
    uploadError = null;
    try {
      value = await upload(field, file);
    } catch (failure) {
      uploadError = failure instanceof Error
        ? failure.message
        : "The file could not be uploaded. Try again.";
    } finally {
      uploading = false;
      input.value = "";
    }
  }

  function toggle(option: string, checked: boolean) {
    value = checked
      ? [...chosen, option]
      : chosen.filter((choice) => choice !== option);
  }
</script>

<div class="field">
  {#if field.type === "checkbox"}
    <label>
      <input
        {id}
        type="checkbox"
        checked={value === true}
        disabled={readonly}
        aria-invalid={Boolean(error)}
        aria-describedby={error ? errorId : undefined}
        onchange={(event) => (value = event.currentTarget.checked)}
      />
      {field.label}{#if field.required}<span aria-hidden="true"> *</span>{/if}
    </label>
  {:else if field.type === "single_choice" || field.type === "multi_choice"}
    <fieldset aria-describedby={error ? errorId : undefined}>
      <legend>
        {field.label}{#if field.required}<span aria-hidden="true"> *</span>{/if}
      </legend>
      {#each field.options ?? [] as option (option.id)}
        <label>
          {#if field.type === "single_choice"}
            <input
              type="radio"
              name={id}
              value={option.id}
              checked={value === option.id}
              disabled={readonly}
              onchange={() => (value = option.id)}
            />
          {:else}
            <input
              type="checkbox"
              value={option.id}
              checked={chosen.includes(option.id)}
              disabled={readonly}
              aria-invalid={Boolean(error)}
              onchange={(event) =>
              toggle(option.id, event.currentTarget.checked)}
            />
          {/if}
          {option.label}
        </label>
      {/each}
    </fieldset>
  {:else}
    <label for={id}>
      {field.label}{#if field.required}<span aria-hidden="true"> *</span>{/if}
    </label>
    {#if field.type === "long_text"}
      <textarea
        {id}
        rows="5"
        maxlength="10000"
        value={text}
        readonly={readonly}
        aria-invalid={Boolean(error)}
        aria-describedby={error ? errorId : undefined}
        oninput={(event) => (value = event.currentTarget.value)}
      ></textarea>
    {:else if field.type === "number"}
      <input
        {id}
        type="number"
        value={typeof value === "number" ? value : ""}
        readonly={readonly}
        aria-invalid={Boolean(error)}
        aria-describedby={error ? errorId : undefined}
        oninput={(event) =>
        (value = event.currentTarget.value === ""
          ? null
          : event.currentTarget.valueAsNumber)}
      />
    {:else if field.type === "file"}
      {#if typeof value === "string" && value}
        <p>A file is attached.</p>
      {/if}
      {#if upload}
        <input
          {id}
          type="file"
          disabled={readonly || uploading}
          aria-invalid={Boolean(error || uploadError)}
          aria-describedby={error || uploadError ? errorId : undefined}
          onchange={(event) => chooseFile(event.currentTarget)}
        />
        {#if uploading}<p role="status">Uploading...</p>{/if}
      {:else}
        <p>File upload is not available yet. You can save your other answers now.</p>
      {/if}
    {:else}
      <input
        {id}
        type={field.type === "email"
        ? "email"
        : field.type === "url"
        ? "url"
        : "text"}
        maxlength={field.type === "text" ? 255 : undefined}
        value={text}
        readonly={readonly}
        aria-invalid={Boolean(error)}
        aria-describedby={error ? errorId : undefined}
        oninput={(event) => (value = event.currentTarget.value)}
      />
    {/if}
  {/if}
  {#if error || uploadError}
    <p id={errorId} class="error">{uploadError ?? error}</p>
  {/if}
</div>

<style>
  .field {
    margin-block: 1rem;
  }
  .error {
    color: #b00020;
  }
  fieldset {
    border: none;
    padding: 0;
  }
  label {
    display: block;
  }
</style>
