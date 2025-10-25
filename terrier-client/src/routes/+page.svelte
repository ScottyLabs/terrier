<script lang="ts">
    import DateRangePicker from "@/components/date-range-picker.svelte";
    import ScottyLabsFilled from "@/components/icons/ScottyLabs_filled.svelte";
    import TimeRangeField from "@/components/time-range-field.svelte";
    import { client } from "@/lib/api";
    import { getAuthContext } from "@/lib/auth.svelte";
    import { createForm } from "@tanstack/svelte-form";
    import { PlusIcon, XCloseIcon } from "@untitled-theme/icons-svelte";
    import { Dialog } from "bits-ui";
    import { onMount } from "svelte";

    const auth = getAuthContext();

    let hackathons = $state<any[]>([]);
    let isLoading = $state(true);

    onMount(async () => {
        const { data, response } = await client.GET("/hackathons/public");

        if (data && response.ok) {
            hackathons = data;
        }

        isLoading = false;
    });

    const form = createForm(() => ({
        defaultValues: {
            name: "",
        },
    }));
</script>

<div class="min-h-screen bg-secondary text-selected flex flex-col">
    <div class="m-7 mr-auto">
        <a href="/" class="gap-2 flex">
            <ScottyLabsFilled class="my-auto" />
            <span class="text-2xl font-medium">Terrier</span>
        </a>
    </div>

    <main class="mx-7">
        {#if auth.user?.is_admin}
            <Dialog.Root>
                <Dialog.Trigger
                    class="bg-selected text-primary cursor-pointer font-semibold px-5 py-3.5 flex gap-2 rounded-4xl mb-6"
                >
                    <PlusIcon class="text-primary" />
                    <span>New Hackathon</span>
                </Dialog.Trigger>

                <Dialog.Portal>
                    <Dialog.Overlay class="fixed inset-0 z-50 bg-black/80" />

                    <Dialog.Content
                        class="rounded-[1.75rem] bg-accent outline-hidden fixed left-[50%] top-[50%] z-50 w-full max-w-[calc(100%-2rem)] translate-x-[-50%] translate-y-[-50%] p-7 sm:max-w-[490px] md:w-full"
                    >
                        <Dialog.Title class="text-2xl"
                            >Create new hackathon</Dialog.Title
                        >

                        <div class="my-7 flex flex-col gap-5">
                            <div class="flex flex-col gap-2">
                                <label
                                    for="name"
                                    class="text-label text-sm font-medium"
                                    >Name</label
                                >
                                <input
                                    id="name"
                                    type="text"
                                    placeholder="Name"
                                    class="text-input h-10 bg-primary rounded-lg px-4 py-2"
                                />
                            </div>

                            <div class="flex flex-col gap-2">
                                <label
                                    for="slug"
                                    class="text-label text-sm font-medium"
                                    >Slug</label
                                >
                                <input
                                    id="slug"
                                    type="text"
                                    placeholder="hackathon-slug"
                                    class="text-input h-10 bg-primary rounded-lg px-4 py-2"
                                />
                            </div>

                            <div class="flex flex-col gap-2">
                                <label
                                    for="description"
                                    class="text-label text-sm font-medium"
                                    >Description</label
                                >
                                <textarea
                                    id="description"
                                    placeholder="Description"
                                    class="text-input h-20 bg-primary rounded-lg px-4 py-2 resize-none"
                                ></textarea>
                            </div>

                            <DateRangePicker />
                            <TimeRangeField />
                        </div>

                        <div class="flex justify-end gap-3">
                            <Dialog.Close
                                class="cursor-pointer font-semibold px-5 py-3.5 rounded-4xl hover:bg-gray-200"
                            >
                                Cancel
                            </Dialog.Close>

                            <Dialog.Close
                                class="bg-selected text-primary cursor-pointer font-semibold px-5 py-3.5 rounded-4xl"
                            >
                                Create
                            </Dialog.Close>
                        </div>

                        <Dialog.Close
                            class="absolute right-5 top-5 cursor-pointer p-2 rounded-full hover:bg-gray-200"
                        >
                            <XCloseIcon class="text-foreground size-5" />
                            <span class="sr-only">Close</span>
                        </Dialog.Close>
                    </Dialog.Content>
                </Dialog.Portal>
            </Dialog.Root>
        {/if}

        <div class="w-full flex flex-col gap-5">
            {#if isLoading}
                <p class="text-center">Loading hackathons...</p>
            {:else if hackathons.length === 0}
                <p class="text-center">No hackathons found.</p>
            {:else}
                <div
                    class="grid grid-cols-auto gap-4 [grid-template-columns:repeat(auto-fill,minmax(16rem,16rem))]"
                >
                    {#each hackathons as hackathon}
                        <a
                            href="/h/{hackathon.slug}/dashboard"
                            class="size-64 p-6 bg-primary rounded-lg shadow-sm hover:shadow-md duration-250 transition-shadow"
                        >
                            <h2 class="text-2xl font-bold">{hackathon.name}</h2>
                            <p class="text-gray-600">
                                {hackathon.description || "No description"}
                            </p>
                        </a>
                    {/each}
                </div>
            {/if}
        </div>
    </main>
</div>
