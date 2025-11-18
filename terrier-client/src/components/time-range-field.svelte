<script lang="ts">
    import { TimeRangeField } from "bits-ui";
    import { Time } from "@internationalized/date";
    export let value: { start: Time; end: Time };
</script>

<TimeRangeField.Root
    class="group flex w-full max-w-[340px] flex-col gap-2"
    bind:value
>
    <TimeRangeField.Label class="text-label text-sm font-medium">
        Hours
    </TimeRangeField.Label>

    <div
        class="rounded-lg bg-primary h-10 text-input flex w-full select-none items-center px-4 py-2 text-sm"
    >
        {#each ["start", "end"] as const as type (type)}
            <TimeRangeField.Input {type}>
                {#snippet children({ segments })}
                    {#each segments as { part, value }, i (part + i)}
                        <div class="inline-block select-none">
                            {#if part === "literal"}
                                <TimeRangeField.Segment {part} class="p-1">
                                    {value}
                                </TimeRangeField.Segment>
                            {:else}
                                <TimeRangeField.Segment
                                    {part}
                                    class="rounded-sm p-1"
                                >
                                    {value}
                                </TimeRangeField.Segment>
                            {/if}
                        </div>
                    {/each}
                {/snippet}
            </TimeRangeField.Input>

            {#if type === "start"}
                <div aria-hidden="true" class="pl-1 pr-2">to</div>
            {/if}
        {/each}
    </div>
</TimeRangeField.Root>
