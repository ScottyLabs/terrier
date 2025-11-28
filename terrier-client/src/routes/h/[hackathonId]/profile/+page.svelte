<script lang="ts">
    import { getAuthContext } from "@/lib/auth.svelte";
    import { Edit01Icon } from "@untitled-theme/icons-svelte";

    const auth = getAuthContext();
</script>

<div class="pt-[30px] flex flex-col gap-5">
    <h1 class="text-3xl font-[550]">Profile</h1>

    <div class="flex flex-col gap-7">
        <!-- Name and School Card -->
        <div class="bg-primary rounded-2xl w-full p-7 relative">
            <div class="absolute top-7 right-7">
                <button
                    class="flex gap-2 items-center text-selected cursor-pointer border border-border rounded-3xl px-5 py-1.5"
                >
                    <Edit01Icon class="size-5" />
                    <span class="font-medium">Edit</span>
                </button>
            </div>

            <!-- Name Section -->
            <div class="mb-10">
                <h2 class="text-lg font-medium mb-4">Name</h2>
                <div class="flex gap-25">
                    <div>
                        <p class="text-label text-sm mb-1">Display Name</p>
                        <p class="text-selected font-medium">Placeholder</p>
                    </div>
                    <div>
                        <p class="text-label text-sm mb-1">Team Name</p>
                        <p class="text-selected font-medium">No team</p>
                    </div>
                </div>
            </div>

            <!-- School Section -->
            <div>
                <h2 class="text-lg font-medium mb-4">School</h2>
                <div class="flex gap-25">
                    <div>
                        <p class="text-label text-sm mb-1">Major</p>
                        <p class="text-selected font-medium">Placeholder</p>
                    </div>
                    <div>
                        <p class="text-label text-sm mb-1">Graduation Year</p>
                        <p class="text-selected font-medium">2027</p>
                    </div>
                </div>
            </div>
        </div>

        <!-- Extra Info Card -->
        <div class="bg-primary rounded-2xl w-full p-7 relative">
            <div class="absolute top-7 right-7">
                <p class="text-label text-sm">Contact organizers to edit</p>
            </div>
            <h2 class="text-lg font-semibold mb-6">Extra Info</h2>
            <div class="flex gap-16">
                <div>
                    <p class="text-label text-sm mb-2">Dietary Restrictions</p>
                    <p class="text-selected font-medium">None</p>
                </div>
                <div>
                    <p class="text-label text-sm mb-2">Shirt Size</p>
                    <p class="text-selected font-medium">M</p>
                </div>
            </div>
        </div>
    </div>
</div>
