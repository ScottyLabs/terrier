<script lang="ts">
    export let label: string;
    export let description: string | null = null;
    export let required: boolean = false;
    export let disabled: boolean = false;
    export let checked: boolean = false;
    export let onInput: (value: boolean) => void = () => {};
</script>

<label class="flex items-start gap-3 cursor-pointer group">
    <div class="relative mt-0.5">
        <input
            type="checkbox"
            class="peer sr-only"
            {checked}
            {disabled}
            on:change={(e) => onInput((e.target as HTMLInputElement).checked)}
        />
        <div class="h-5 w-5 rounded-md border-2 border-gray-300 bg-slate-100 transition-colors
            peer-checked:bg-gray-700 peer-checked:border-gray-700
            peer-focus:ring-2 peer-focus:ring-blue-300 peer-focus:ring-offset-1
            peer-disabled:opacity-50 peer-disabled:cursor-not-allowed
            group-hover:border-gray-400"></div>
        <svg
            class="absolute top-0.5 left-0.5 h-4 w-4 text-white opacity-0 peer-checked:opacity-100 transition-opacity pointer-events-none"
            fill="none"
            viewBox="0 0 24 24"
            stroke="currentColor"
            stroke-width="3"
        >
            <path stroke-linecap="round" stroke-linejoin="round" d="M5 13l4 4L19 7" />
        </svg>
    </div>
    <div class="flex flex-col">
        <span class="text-lg text-gray-800">
            {label}
            {#if required}
                <span class="text-error">*</span>
            {/if}
        </span>
        {#if description}
            <p class="text-sm text-muted-foreground">{description}</p>
        {/if}
    </div>
</label>
