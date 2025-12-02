<script lang="ts">
    export let label: string;
    export let description: string | null = null;
    export let required: boolean = false;
    export let disabled: boolean = false;
    export let value: string = "";
    export let onInput: (value: string) => void = () => {};
    export let options: string[] = [];

    // Parse comma-separated string to array
    function getSelectedOptions(): string[] {
        if (!value) return [];
        return value.split(",").filter((v) => v.trim() !== "");
    }

    function toggleOption(option: string) {
        if (disabled) return;
        const selected = getSelectedOptions();
        const index = selected.indexOf(option);
        if (index > -1) {
            selected.splice(index, 1);
        } else {
            selected.push(option);
        }
        onInput(selected.join(","));
    }

    function isSelected(option: string): boolean {
        return getSelectedOptions().includes(option);
    }
</script>

<fieldset class="flex flex-col gap-2">
    <legend class="text-lg text-gray-800">
        {label}
        {#if required}
            <span class="text-error">*</span>
        {/if}
    </legend>
    {#if description}
        <p class="text-sm text-muted-foreground">{description}</p>
    {/if}
    <div class="flex flex-col gap-2 mt-1">
        {#each options as option}
            <label
                class="flex items-center gap-3 cursor-pointer group {disabled
                    ? 'opacity-50 cursor-not-allowed'
                    : ''}"
            >
                <div class="relative">
                    <input
                        type="checkbox"
                        class="peer sr-only"
                        checked={isSelected(option)}
                        {disabled}
                        onchange={() => toggleOption(option)}
                    />
                    <div
                        class="h-5 w-5 rounded-md border-2 border-gray-300 bg-slate-100 transition-colors
                        peer-checked:bg-gray-700 peer-checked:border-gray-700
                        peer-focus:ring-2 peer-focus:ring-blue-300 peer-focus:ring-offset-1
                        peer-disabled:cursor-not-allowed
                        group-hover:border-gray-400"
                    ></div>
                    <svg
                        class="absolute top-0.5 left-0.5 h-4 w-4 text-white opacity-0 peer-checked:opacity-100 transition-opacity pointer-events-none"
                        fill="none"
                        viewBox="0 0 24 24"
                        stroke="currentColor"
                        stroke-width="3"
                    >
                        <path
                            stroke-linecap="round"
                            stroke-linejoin="round"
                            d="M5 13l4 4L19 7"
                        />
                    </svg>
                </div>
                <span class="text-gray-700">{option}</span>
            </label>
        {/each}
    </div>
</fieldset>
