<script lang="ts">
    import { getAuthContext } from "@/lib/auth.svelte";

    import Checkbox from "@/components/form-elements/checkbox.svelte";
    import Dropdown from "@/components/form-elements/dropdown.svelte";
    import Signature from "@/components/form-elements/signature.svelte";
    import SingleLineText from "@/components/form-elements/single-line-text.svelte";

    import LongResponse from "@/components/form-elements/long-response.svelte";
    import MultiCheckbox from "@/components/form-elements/multi-checkbox.svelte";
    import formSchemaImport from "./mock_form.json";
    const formSchema: FormSchema = formSchemaImport as FormSchema;

    const auth = getAuthContext();

    // Stateful form data
    let formData: Record<string, string | boolean> = $state({});

    type SingleLineTextQuestion = {
        id: string;
        type: "single-line-text";
        question: string;
        description: string | null;
        maxLength: number | null;
        required: boolean;
        condition?: { id: string; value: string } | null;
    };
    type LongResponseQuestion = {
        id: string;
        type: "long-response";
        question: string;
        description: string | null;
        maxLength: number | null;
        required: boolean;
        condition?: { id: string; value: string } | null;
    };
    type DropdownQuestion = {
        id: string;
        type: "dropdown";
        question: string;
        description: string | null;
        options: string[];
        required: boolean;
        condition?: { id: string; value: string } | null;
    };
    type CheckboxQuestion = {
        id: string;
        type: "checkbox";
        question: string;
        description: string | null;
        required: boolean;
        condition?: { id: string; value: string } | null;
    };
    type MultiCheckboxQuestion = {
        id: string;
        type: "multi-checkbox";
        question: string;
        description: string | null;
        options: string[];
        required: boolean;
        condition?: { id: string; value: string } | null;
    };
    type SignatureQuestion = {
        id: string;
        type: "signature";
        question: string;
        description: string | null;
        required: boolean;
        condition?: { id: string; value: string } | null;
    };
    type FormSchema = Record<
        string,
        (
            | SingleLineTextQuestion
            | DropdownQuestion
            | CheckboxQuestion
            | SignatureQuestion
            | MultiCheckboxQuestion
            | LongResponseQuestion
        )[]
    >;

    // const formSchema: FormSchema = await fetch(
    //     `/api/h/${auth.hackathonId}/application/form-schema`
    // ).then((res) => res.json());
</script>

<div
    class="min-w-1/4 max-w-128 mt-10 mx-auto flex flex-col gap-10 justify-evenly"
>
    <div class="relative w-full min-h-8 justify-center flex">
        <h1 class="text-2xl">Application</h1>
        <div
            class="absolute flex gap-1.5 items-center border rounded-md border-border h-8 top-0 right-0 px-2.5 py-1.5"
        >
            <div class="w-2.5 h-2.5 rounded-full bg-selected"></div>
            <p class="text-sm font-semibold">Saved</p>
        </div>
    </div>

    <div class="flex flex-col gap-8">
        {#each Object.entries(formSchema) as [section, questions]}
            <div class="bg-primary shadow-lg rounded-4xl w-full p-7">
                <h2 class="text-lg font-semibold capitalize mb-5">
                    {section} Information
                </h2>
                <div class="flex flex-col gap-6">
                    {#each questions.filter((q) => !q.condition || formData[q.condition.id] === q.condition.value) as question}
                        {#if question.type === "single-line-text"}
                            <SingleLineText
                                label={question.question}
                                description={question.description}
                                maxLength={question.maxLength}
                                required={question.required}
                                value={formData[question.id]?.toString() ?? ""}
                                onInput={(v: string) =>
                                    (formData[question.id] = v)}
                            />
                        {:else if question.type === "long-response"}
                            <LongResponse
                                label={question.question}
                                description={question.description}
                                maxLength={question.maxLength}
                                required={question.required}
                                value={formData[question.id]?.toString() ?? ""}
                                onInput={(v: string) =>
                                    (formData[question.id] = v)}
                            />
                        {:else if question.type === "dropdown"}
                            <Dropdown
                                label={question.question}
                                description={question.description}
                                options={question.options}
                                required={question.required}
                                value={formData[question.id]?.toString() ?? ""}
                                onInput={(v: string) =>
                                    (formData[question.id] = v)}
                            />
                        {:else if question.type === "multi-checkbox"}
                            <MultiCheckbox
                                label={question.question}
                                description={question.description}
                                options={question.options}
                                required={question.required}
                                value={formData[question.id]?.toString() ?? ""}
                                onInput={(v: string) =>
                                    (formData[question.id] = v)}
                            />
                        {:else if question.type === "checkbox"}
                            <Checkbox
                                label={question.question}
                                description={question.description}
                                required={question.required}
                                checked={!!formData[question.id]}
                                onInput={(v: boolean) =>
                                    (formData[question.id] = v)}
                            />
                        {:else if question.type === "signature"}
                            <Signature
                                label={question.question}
                                description={question.description}
                                required={question.required}
                                value={formData[question.id]?.toString() ?? ""}
                                onInput={(v: string) =>
                                    (formData[question.id] = v)}
                            />
                        {/if}
                    {/each}
                </div>
            </div>
        {/each}
    </div>

    <div class="flex justify-end">
        <button
            class="bg-selected text-primary font-semibold px-5 py-3.5 rounded-4xl"
        >
            Submit
        </button>
    </div>
</div>
