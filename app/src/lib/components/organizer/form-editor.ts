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

export type ValidationError = {
  fieldIndex: number;
  optionIndex?: number;
  message: string;
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

function validateOptions(field: Field, fieldIndex: number): ValidationError[] {
  const errors: ValidationError[] = [];
  const options = field.options ?? [];

  if (isChoiceType(field.type)) {
    if (options.length === 0) {
      errors.push({
        fieldIndex,
        message: "At least one option is required.",
      });
    }
  } else if (options.length > 0) {
    errors.push({
      fieldIndex,
      message: "Only choice questions can have options.",
    });
  }

  const optionIds = new Set<string>();

  for (const [optionIndex, option] of options.entries()) {
    if (!isValidLabel(option.label)) {
      errors.push({
        fieldIndex,
        optionIndex,
        message: "Option must use 1 to 255 characters.",
      });
    }

    if (!isValidId(option.id)) {
      errors.push({
        fieldIndex,
        optionIndex,
        message: "Invalid option ID.",
      });
    }

    if (optionIds.has(option.id)) {
      errors.push({
        fieldIndex,
        optionIndex,
        message: "Duplicate option ID.",
      });
    }

    optionIds.add(option.id);
  }

  return errors;
}

export function validateForm(form: Form): ValidationError[] {
  const errors: ValidationError[] = [];
  const fieldIds = new Set<string>();

  for (const [fieldIndex, field] of form.fields.entries()) {
    if (!isValidLabel(field.label)) {
      errors.push({
        fieldIndex,
        message: "Question must use 1 to 255 characters.",
      });
    }

    if (!isValidId(field.id)) {
      errors.push({
        fieldIndex,
        message: "Invalid question ID.",
      });
    }

    if (fieldIds.has(field.id)) {
      errors.push({
        fieldIndex,
        message: "Duplicate question ID.",
      });
    }

    fieldIds.add(field.id);

    errors.push(...validateOptions(field, fieldIndex));
  }

  return errors;
}
