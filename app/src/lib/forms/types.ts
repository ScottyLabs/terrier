import type { components } from "$lib/api/schema";

export type Form = components["schemas"]["Form"];
export type Field = components["schemas"]["Field"];

/** Answers keyed by field `id`. */
export type Answers = Record<string, unknown>;

/** Error messages keyed by field `id`. */
export type FieldErrors = Record<string, string>;

/** Uploads a file for a `file` field and resolves to the attachment ID to store as its answer. */
export type Upload = (field: Field, file: File) => Promise<string>;
