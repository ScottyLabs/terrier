<script lang="ts">
    export let label: string;
    export let description: string | null = null;
    export let required: boolean = false;
    export let checked: boolean = false;
    export let onInput: (value: boolean) => void = () => {};
</script>

<label class="flex items-center gap-3">
    <input
        type="checkbox"
        class="appearance-none border-2 border-black rounded-sm h-5 w-5 bg-white text-blue-600 focus:ring-2 focus:ring-blue-300 checked:bg-gray-600 checked:border-transparent checked:bg-checkmark"
        bind:checked
        on:change={(e) => onInput((e.target as HTMLInputElement).checked)}
    />
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
