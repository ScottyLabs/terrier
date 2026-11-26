// Temporary types until the shared application form types are available.

export type FieldType =
  | "text"
  | "long_text"
  | "email"
  | "url"
  | "number"
  | "single_choice"
  | "multi_choice"
  | "checkbox"
  | "file";

export type Option = {
  id: string;
  label: string;
};

export type Field = {
  id: string;
  label: string;
  type: FieldType;
  required: boolean;
  options?: Option[];
};

export type Form = {
  fields: Field[];
};

export function isChoiceType(type: FieldType): boolean {
  return type === "single_choice" || type === "multi_choice";
}

function isValidId(id: string): boolean {
  return /^[a-z][a-z0-9_]{0,63}$/u.test(id);
}

function isValidLabel(label: string): boolean {
  return label.trim() !== "" && Array.from(label).length <= 255;
}

function validateOptions(field: Field, name: string): string[] {
  const errors: string[] = [];
  const options = field.options ?? [];

  if (isChoiceType(field.type)) {
    if (options.length === 0) {
      errors.push(`${name}: At least one option is required.`);
    }
  } else if (options.length > 0) {
    errors.push(`${name}: Only choice questions can have options.`);
  }

  const optionIds = new Set<string>();

  for (const [i, option] of options.entries()) {
    if (!isValidLabel(option.label)) {
      errors.push(`${name}: Option ${i + 1} must use 1 to 255 characters.`);
    }

    if (!isValidId(option.id)) {
      errors.push(`${name}: Invalid option ID.`);
    }

    if (optionIds.has(option.id)) {
      errors.push(`${name}: Duplicate option ID.`);
    }

    optionIds.add(option.id);
  }

  return errors;
}

export function validateForm(form: Form): string[] {
  const errors: string[] = [];
  const fieldIds = new Set<string>();

  for (const [index, field] of form.fields.entries()) {
    const name = `Question ${index + 1}`;

    if (!isValidLabel(field.label)) {
      errors.push(`${name}: Use 1 to 255 characters.`);
    }

    if (!isValidId(field.id)) {
      errors.push(`${name}: Invalid question ID.`);
    }

    if (fieldIds.has(field.id)) {
      errors.push(`${name}: Duplicate question ID.`);
    }

    fieldIds.add(field.id);

    errors.push(...validateOptions(field, name));
  }

  return errors;
}
