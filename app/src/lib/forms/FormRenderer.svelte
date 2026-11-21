<script lang="ts">
  import Field from "./Field.svelte";
  import type { Answers, FieldErrors, Form, Upload } from "./types";

  let {
    form,
    answers = $bindable(),
    errors = {},
    readonly = false,
    upload,
  }: {
    form: Form;
    answers: Answers;
    errors?: FieldErrors;
    readonly?: boolean;
    /** Omit to show file questions as unavailable, as in a setup preview. */
    upload?: Upload;
  } = $props();
</script>

{#each form.fields ?? [] as field (field.id)}
  <Field
    {field}
    bind:value={answers[field.id]}
    error={errors[field.id]}
    {readonly}
    {upload}
  />
{/each}
