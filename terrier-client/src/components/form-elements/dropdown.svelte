<script lang="ts">
    export let label: string;
    export let description: string | null = null;
    export let maxLength: number | null = null;
    export let required: boolean = false;
    export let value: string = "";
    export let onInput: (value: string) => void = () => {};
    export let options: string[] = [];
</script>

<label class="flex flex-col gap-2">
    <span class="text-lg text-gray-800">
        {label}
        {#if required}
            <span class="text-error">*</span>
        {/if}
    </span>
    {#if description}
        <p class="text-sm text-muted-foreground">{description}</p>
    {/if}
    <div class="relative">
        <select
            class="w-full appearance-none rounded-xl bg-slate-100 px-4 py-3 pr-10 text-gray-500 outline-none focus:ring-2 focus:ring-blue-300"
            bind:value
            on:change={(e) => onInput((e.target as HTMLSelectElement).value)}
        >
            <option value="" disabled selected hidden>Select</option>
            {#each options as option}
                <option value={option}>{option}</option>
            {/each}
        </select>
        <svg
            class="pointer-events-none absolute right-4 top-1/2 h-5 w-5 -translate-y-1/2 text-gray-500"
            fill="none"
            stroke="currentColor"
            stroke-width="2"
            viewBox="0 0 24 24"
        >
            <path
                stroke-linecap="round"
                stroke-linejoin="round"
                d="M19 9l-7 7-7-7"
            />
        </svg>
    </div>
</label>
