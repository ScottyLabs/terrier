<script lang="ts">
    import Dropdown from "@/components/form-elements/dropdown.svelte";
    import SingleLineText from "@/components/form-elements/single-line-text.svelte";

    import { getAuthContext } from "@/lib/auth.svelte";

    const auth = getAuthContext();
    type SingleLineTextQuestion = {
        id: string;
        question: string;
        description: string | null;
        maxLength: number | null;
        required: boolean;
    };
    type DropdownQuestion = {
        id: string;
        question: string;
        description: string | null;
        options: string[];
        required: boolean;
    };
    type FormSchema = Record<
        string,
        (SingleLineTextQuestion | DropdownQuestion)[]
    >;
    const formSchema: FormSchema = {
        personal: [
            {
                id: "full_name",
                question: "Full Name",
                description: null,
                maxLength: 100,
                required: true,
            },
            {
                id: "email",
                question: "Email Address",
                description: null,
                maxLength: 100,
                required: true,
            },
        ],
        project: [
            {
                id: "project_idea",
                question: "Project Idea",
                description: "Describe your project idea in detail.",
                maxLength: 500,
                required: true,
            },
            {
                id: "tech_stack",
                question: "Tech Stack",
                description: null,
                options: ["JavaScript", "Python", "Java", "C++", "Other"],
                required: true,
            } as DropdownQuestion,
        ],
    };
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
                    {#each questions as question}
                        {#if "options" in question}
                            <Dropdown
                                label={question.question}
                                description={question.description}
                                maxLength={null}
                                required={question.required}
                                value=""
                                onInput={(value) =>
                                    console.log(
                                        `Dropdown ${question.id} input: ${value}`,
                                    )}
                                options={question.options}
                            />
                        {:else}
                            <SingleLineText
                                label={question.question}
                                description={question.description}
                                maxLength={question.maxLength}
                                required={question.required}
                                value=""
                                onInput={(value) =>
                                    console.log(
                                        `Text ${question.id} input: ${value}`,
                                    )}
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
