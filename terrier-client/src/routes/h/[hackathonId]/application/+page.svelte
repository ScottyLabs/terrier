<script lang="ts">
    import { client } from "@/lib/api";
    import { getAuthContext, getHackathonContext } from "@/lib/auth.svelte";
    import { onMount } from "svelte";

    import Checkbox from "@/components/form-elements/checkbox.svelte";
    import Dropdown from "@/components/form-elements/dropdown.svelte";
    import LongResponse from "@/components/form-elements/long-response.svelte";
    import MultiCheckbox from "@/components/form-elements/multi-checkbox.svelte";
    import Signature from "@/components/form-elements/signature.svelte";
    import SingleLineText from "@/components/form-elements/single-line-text.svelte";

    import formSchemaImport from "./mock_form.json";
    const formSchema: FormSchema = formSchemaImport as FormSchema;

    const auth = getAuthContext();
    const hackathon = getHackathonContext();

    // Stateful form data
    let formData: Record<string, string | boolean> = $state({});
    let saveStatus: "saved" | "saving" | "error" = $state("saved");
    let applicationStatus: string = $state("draft");
    let isSubmitting: boolean = $state(false);
    let submitError: string | null = $state(null);
    let showUnsubmitWarning: boolean = $state(false);
    let pendingEdit: { id: string; value: string | boolean } | null =
        $state(null);
    let formKey: number = $state(0); // Used to force re-render on cancel

    // Debounce timer for auto-save
    let saveTimeout: ReturnType<typeof setTimeout> | null = null;

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

    // Load existing application on mount
    onMount(async () => {
        if (!hackathon.hackathonId) return;

        const { data, error } = await client.GET(
            "/hackathons/{slug}/application",
            {
                params: { path: { slug: hackathon.hackathonId } },
            },
        );

        if (data) {
            formData = data.form_data as Record<string, string | boolean>;
            applicationStatus = data.status;
        }
        // 404 is fine - means no application yet
    });

    // Auto-save function with debounce
    function scheduleAutoSave() {
        if (applicationStatus === "under_review") return;

        if (saveTimeout) {
            clearTimeout(saveTimeout);
        }

        saveStatus = "saving";
        saveTimeout = setTimeout(async () => {
            await saveApplication();
        }, 1000); // Save after 1 second of no changes
    }

    async function saveApplication() {
        if (!hackathon.hackathonId || applicationStatus === "under_review")
            return;

        const { data, error } = await client.PUT(
            "/hackathons/{slug}/application",
            {
                params: { path: { slug: hackathon.hackathonId } },
                body: { form_data: formData },
            },
        );

        if (error) {
            saveStatus = "error";
        } else {
            saveStatus = "saved";
            // Update status from response (will be "draft" after editing a submitted app)
            if (data) {
                applicationStatus = data.status;
            }
        }
    }

    async function handleSubmit() {
        if (!hackathon.hackathonId) return;

        isSubmitting = true;
        submitError = null;

        // Save first to make sure latest data is persisted
        await saveApplication();

        const { data, error } = await client.POST(
            "/hackathons/{slug}/application/submit",
            {
                params: { path: { slug: hackathon.hackathonId } },
            },
        );

        isSubmitting = false;

        if (error) {
            submitError = "Failed to submit application. Please try again.";
        } else {
            applicationStatus = "submitted";
        }
    }

    // Update handler that triggers auto-save
    function handleInput(id: string, value: string | boolean) {
        // If application is submitted, show warning before allowing edit
        if (applicationStatus === "submitted") {
            pendingEdit = { id, value };
            showUnsubmitWarning = true;
            return;
        }

        // If under review, don't allow editing at all
        if (applicationStatus === "under_review") {
            return;
        }

        formData[id] = value;
        scheduleAutoSave();
    }

    function confirmUnsubmit() {
        if (pendingEdit) {
            formData[pendingEdit.id] = pendingEdit.value;
            pendingEdit = null;
        }
        showUnsubmitWarning = false;
        scheduleAutoSave();
    }

    function cancelUnsubmit() {
        pendingEdit = null;
        showUnsubmitWarning = false;
        formKey++; // Force re-render to reset input values
    }
</script>

<div
    class="min-w-1/4 max-w-128 mt-10 mx-auto flex flex-col gap-10 justify-evenly"
>
    <div class="relative w-full min-h-8 justify-center flex">
        <h1 class="text-2xl">Application</h1>
        <div
            class="absolute flex gap-1.5 items-center border rounded-md border-border h-8 top-0 right-0 px-2.5 py-1.5"
        >
            {#if applicationStatus === "submitted"}
                <div class="w-2.5 h-2.5 rounded-full bg-green-500"></div>
                <p class="text-sm font-semibold">Submitted</p>
            {:else if applicationStatus === "under_review"}
                <div class="w-2.5 h-2.5 rounded-full bg-purple-500"></div>
                <p class="text-sm font-semibold">Under Review</p>
            {:else if saveStatus === "saving"}
                <div class="w-2.5 h-2.5 rounded-full bg-yellow-500"></div>
                <p class="text-sm font-semibold">Saving...</p>
            {:else if saveStatus === "error"}
                <div class="w-2.5 h-2.5 rounded-full bg-red-500"></div>
                <p class="text-sm font-semibold">Error</p>
            {:else}
                <div class="w-2.5 h-2.5 rounded-full bg-selected"></div>
                <p class="text-sm font-semibold">Saved</p>
            {/if}
        </div>
    </div>

    {#key formKey}
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
                                    disabled={applicationStatus ===
                                        "under_review"}
                                    value={formData[question.id]?.toString() ??
                                        ""}
                                    onInput={(v: string) =>
                                        handleInput(question.id, v)}
                                />
                            {:else if question.type === "long-response"}
                                <LongResponse
                                    label={question.question}
                                    description={question.description}
                                    maxLength={question.maxLength}
                                    required={question.required}
                                    disabled={applicationStatus ===
                                        "under_review"}
                                    value={formData[question.id]?.toString() ??
                                        ""}
                                    onInput={(v: string) =>
                                        handleInput(question.id, v)}
                                />
                            {:else if question.type === "dropdown"}
                                <Dropdown
                                    label={question.question}
                                    description={question.description}
                                    options={question.options}
                                    required={question.required}
                                    disabled={applicationStatus ===
                                        "under_review"}
                                    value={formData[question.id]?.toString() ??
                                        ""}
                                    onInput={(v: string) =>
                                        handleInput(question.id, v)}
                                />
                            {:else if question.type === "multi-checkbox"}
                                <MultiCheckbox
                                    label={question.question}
                                    description={question.description}
                                    options={question.options}
                                    required={question.required}
                                    disabled={applicationStatus ===
                                        "under_review"}
                                    value={formData[question.id]?.toString() ??
                                        ""}
                                    onInput={(v: string) =>
                                        handleInput(question.id, v)}
                                />
                            {:else if question.type === "checkbox"}
                                <Checkbox
                                    label={question.question}
                                    description={question.description}
                                    required={question.required}
                                    disabled={applicationStatus ===
                                        "under_review"}
                                    checked={!!formData[question.id]}
                                    onInput={(v: boolean) =>
                                        handleInput(question.id, v)}
                                />
                            {:else if question.type === "signature"}
                                <Signature
                                    label={question.question}
                                    description={question.description}
                                    required={question.required}
                                    disabled={applicationStatus ===
                                        "under_review"}
                                    value={formData[question.id]?.toString() ??
                                        ""}
                                    onInput={(v: string) =>
                                        handleInput(question.id, v)}
                                />
                            {/if}
                        {/each}
                    </div>
                </div>
            {/each}
        </div>
    {/key}

    {#if submitError}
        <p class="text-red-500 text-center">{submitError}</p>
    {/if}

    <div class="flex justify-end">
        {#if applicationStatus === "under_review"}
            <div
                class="bg-purple-600 text-white font-semibold px-5 py-3.5 rounded-4xl"
            >
                Under Review
            </div>
        {:else if applicationStatus === "submitted"}
            <div
                class="bg-green-600 text-white font-semibold px-5 py-3.5 rounded-4xl"
            >
                Application Submitted
            </div>
        {:else}
            <button
                class="bg-selected text-primary font-semibold px-5 py-3.5 rounded-4xl disabled:opacity-50"
                onclick={handleSubmit}
                disabled={isSubmitting}
            >
                {isSubmitting ? "Submitting..." : "Submit"}
            </button>
        {/if}
    </div>
</div>

<!-- Unsubmit Warning Modal -->
{#if showUnsubmitWarning}
    <div
        class="fixed inset-0 bg-black/50 flex items-center justify-center z-50"
        onclick={cancelUnsubmit}
        onkeydown={(e) => e.key === "Escape" && cancelUnsubmit()}
        role="dialog"
        aria-modal="true"
        tabindex="-1"
    >
        <div
            class="bg-white rounded-2xl p-6 max-w-md mx-4 shadow-xl"
            onclick={(e) => e.stopPropagation()}
            onkeydown={(e) => e.stopPropagation()}
            role="document"
        >
            <h2 class="text-xl font-bold mb-3">Edit Submitted Application?</h2>
            <p class="text-gray-600 mb-6">
                Editing your application will un-submit it and change its status
                back to draft. You will need to submit again after you're done
                making changes.
            </p>
            <div class="flex gap-3 justify-end">
                <button
                    class="px-4 py-2 rounded-lg border border-gray-300 text-gray-700 hover:bg-gray-50"
                    onclick={cancelUnsubmit}
                >
                    Cancel
                </button>
                <button
                    class="px-4 py-2 rounded-lg bg-orange-500 text-white hover:bg-orange-600"
                    onclick={confirmUnsubmit}
                >
                    Edit Anyway
                </button>
            </div>
        </div>
    </div>
{/if}
