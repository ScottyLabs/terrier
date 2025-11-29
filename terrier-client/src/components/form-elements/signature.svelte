<script lang="ts">
    import DatePicker from "@/components/form-elements/date-picker.svelte";
    import { parseDate } from "@internationalized/date";

    export let label: string;
    export let description: string | null = null;
    export let placeholder: string = "Enter";
    export let maxLength: number | null = null;
    export let required: boolean = false;
    export let value: string = "";
    export let onInput: (value: string) => void = () => {};

    let date = parseDate(new Date().toISOString().slice(0, 10));
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
    <input
        type="text"
        class="w-full rounded-xl bg-slate-100 px-4 py-2 text-gray-500 placeholder-gray-400 outline-none focus:ring-2 focus:ring-blue-300"
        bind:value
        {placeholder}
        maxlength={maxLength ?? undefined}
        on:input={(e) => onInput((e.target as HTMLInputElement).value)}
    />
    <div class="ml-auto">
        <DatePicker value={date} on:dateSelected={(e) => onInput(e.detail)} />
    </div>
</label>
