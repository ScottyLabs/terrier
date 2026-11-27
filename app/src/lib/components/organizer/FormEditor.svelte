<!-- should use same imported types later -->
<script lang="ts">
    import type { Form, Field, FieldType } from "./form-editor";
    import { isChoiceType, validateForm } from "./form-editor";

    let { schema = $bindable() }: { schema: Form } = $props();

    function generateId(prefix: string = "q"): string {
        return prefix + "_" + crypto.randomUUID().replaceAll("-", "");
    }

    function addField() {
        schema.fields.push({
            id: generateId(),
            label: "",
            type: "text",
            required: false,
        });
    }

    function removeField(id: string) {
        schema.fields = schema.fields.filter(
            (field) => field.id !== id
        );
    }

    function moveField(index: number, direction: -1 | 1) {
        const newIndex = index + direction;

        if (newIndex < 0 || newIndex >= schema.fields.length) {
            return;
        }

        const temp = schema.fields[index];
        schema.fields[index] = schema.fields[newIndex];
        schema.fields[newIndex] = temp;
    }

    function changeFieldType(field: Field, type: FieldType) {
        field.type = type;

        if (isChoiceType(type)) {
            if (field.options === undefined) {
                field.options = [];
            }
        } else {
            delete field.options;
        }
    }

    function addOption(field: Field) {
        if (!isChoiceType(field.type)) {
            return;
        }

        if (field.options === undefined) {
            field.options = [];
        }

        field.options.push({
            id: generateId("opt"),
            label: "",
        });
    }

    function removeOption(field: Field, optionId: string) {
        if (field.options === undefined) {
            return;
        }

        field.options = field.options.filter(
            (option) => option.id !== optionId
        );
    }

    let errors = $derived(validateForm(schema));
</script>


<h1>Application Form Editor</h1>

<p>{schema.fields.length} questions</p>

{#each schema.fields as field, index (field.id)}
    <fieldset>
        <legend>Question {index + 1}</legend>

        <label>
            Question:
            <input type="text" bind:value={field.label} />
        </label>
        <label>
            Input type:
            <select
                value={field.type}
                onchange={(event) =>
                    changeFieldType(field, event.currentTarget.value as FieldType)}
            >
                <option value="text">Short text</option>
                <option value="long_text">Long text</option>
                <option value="email">Email</option>
                <option value="url">URL</option>
                <option value="number">Number</option>
                <option value="single_choice">Single choice</option>
                <option value="multi_choice">Multiple choice</option>
                <option value="checkbox">Checkbox</option>
            </select>
        </label>

        {#if isChoiceType(field.type)}
            <div>
                <h4>Options</h4>

                {#each field.options ?? [] as option, optionIndex (option.id)}
                    <div>
                        <label>
                            Option {optionIndex + 1}:
                            <input
                                type="text"
                                bind:value={option.label}
                                placeholder="Enter an option"
                            />
                        </label>

                        <button
                            type="button"
                            onclick={() => removeOption(field, option.id)}
                        >
                            Remove option
                        </button>

                        {#each errors.filter(
                            (error) => error.fieldIndex === index &&
                                error.optionIndex === optionIndex
                        ) as error}
                            <p>{error.message}</p>
                        {/each}
                    </div>
                {/each}

                <button
                    type="button"
                    onclick={() => addOption(field)}
                >
                    + Add option
                </button>
            </div>
        {/if}


        <label>
            <input type="checkbox" bind:checked={field.required} />
            Required
        </label>

        <button
            type="button"
            onclick={() => moveField(index, -1)}
            disabled={index === 0}
        >
            Move up
        </button>

        <button
            type="button"
            onclick={() => moveField(index, 1)}
            disabled={index === schema.fields.length - 1}
        >
            Move down
        </button>

        <button type="button" onclick={() => removeField(field.id)}>
            Remove question
        </button>

        {#each errors.filter(
            (error) => error.fieldIndex === index &&
                error.optionIndex === undefined
        ) as error}
            <p>{error.message}</p>
        {/each}
    </fieldset>
{/each}

<button type="button" onclick={addField}>
    + Add question
</button>