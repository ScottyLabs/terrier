<script lang="ts">
    import { CalendarDate } from "@internationalized/date";
    import {
        CalendarIcon,
        ChevronLeftIcon,
        ChevronRightIcon,
    } from "@untitled-theme/icons-svelte";
    import { DateRangePicker } from "bits-ui";

    export let value: { start: CalendarDate; end: CalendarDate };
</script>

<DateRangePicker.Root
    weekdayFormat="short"
    fixedWeeks={true}
    class="flex w-full max-w-[340px] flex-col gap-2"
    bind:value
>
    <DateRangePicker.Label class="text-label text-sm font-medium"
        >Days</DateRangePicker.Label
    >

    <div
        class="rounded-lg bg-primary h-10 text-input flex w-full select-none items-center px-4 py-2 text-sm"
    >
        {#each ["start", "end"] as const as type (type)}
            <DateRangePicker.Input {type}>
                {#snippet children({ segments })}
                    {#each segments as { part, value }, i (part + i)}
                        <div class="inline-block select-none">
                            {#if part === "literal"}
                                <DateRangePicker.Segment {part} class="p-1">
                                    {value}
                                </DateRangePicker.Segment>
                            {:else}
                                <DateRangePicker.Segment
                                    {part}
                                    class="rounded-5px p-1"
                                >
                                    {value}
                                </DateRangePicker.Segment>
                            {/if}
                        </div>
                    {/each}
                {/snippet}
            </DateRangePicker.Input>

            {#if type === "start"}
                <div aria-hidden="true" class="px-1">-</div>
            {/if}
        {/each}

        <DateRangePicker.Trigger
            class="text-input ml-auto inline-flex size-8 items-center justify-center"
        >
            <CalendarIcon class="size-5" />
        </DateRangePicker.Trigger>
    </div>

    <DateRangePicker.Content sideOffset={6} class="z-50">
        <DateRangePicker.Calendar
            class="bg-primary border border-border rounded-lg shadow-lg p-7"
        >
            {#snippet children({ months, weekdays })}
                <DateRangePicker.Header
                    class="flex items-center justify-between"
                >
                    <DateRangePicker.PrevButton
                        class="rounded-lg hover:bg-gray-200 inline-flex size-10 items-center justify-center transition-all active:scale-[0.98]"
                    >
                        <ChevronLeftIcon class="size-6" />
                    </DateRangePicker.PrevButton>

                    <DateRangePicker.Heading class="text-[15px] font-medium" />

                    <DateRangePicker.NextButton
                        class="rounded-lg hover:bg-gray-200 inline-flex size-10 items-center justify-center transition-all active:scale-[0.98]"
                    >
                        <ChevronRightIcon class="size-6" />
                    </DateRangePicker.NextButton>
                </DateRangePicker.Header>
                <div
                    class="flex flex-col space-y-4 pt-4 sm:flex-row sm:space-x-4 sm:space-y-0"
                >
                    {#each months as month (month.value)}
                        <DateRangePicker.Grid
                            class="w-full border-collapse select-none space-y-1"
                        >
                            <DateRangePicker.GridHead>
                                <DateRangePicker.GridRow
                                    class="mb-1 flex w-full justify-between"
                                >
                                    {#each weekdays as day (day)}
                                        <DateRangePicker.HeadCell
                                            class="text-muted-foreground font-normal! w-10 rounded-md text-xs"
                                        >
                                            <div>
                                                {day.slice(0, 2)}
                                            </div>
                                        </DateRangePicker.HeadCell>
                                    {/each}
                                </DateRangePicker.GridRow>
                            </DateRangePicker.GridHead>
                            <DateRangePicker.GridBody>
                                {#each month.weeks as weekDates (weekDates)}
                                    <DateRangePicker.GridRow
                                        class="flex w-full"
                                    >
                                        {#each weekDates as date (date)}
                                            <DateRangePicker.Cell
                                                {date}
                                                month={month.value}
                                                class="p-0! relative m-0 size-10 overflow-visible text-center text-sm focus-within:relative focus-within:z-20"
                                            >
                                                <DateRangePicker.Day
                                                    class="hover:bg-gray-200 data-highlighted:bg-gray-100 data-selected:bg-gray-100 data-disabled:pointer-events-none data-outside-month:pointer-events-none data-selected:font-medium data-selection-end:font-medium data-selection-start:font-medium data-selection-start:focus-visible:ring-2 data-selection-start:focus-visible:ring-offset-2! data-unavailable:line-through group relative inline-flex size-10 items-center justify-center overflow-visible whitespace-nowrap border border-transparent bg-transparent p-0 text-sm font-normal transition-all"
                                                >
                                                    <div
                                                        class="group-data-today:block absolute top-[5px] hidden size-1 rounded-full transition-all"
                                                    ></div>
                                                    {date.day}
                                                </DateRangePicker.Day>
                                            </DateRangePicker.Cell>
                                        {/each}
                                    </DateRangePicker.GridRow>
                                {/each}
                            </DateRangePicker.GridBody>
                        </DateRangePicker.Grid>
                    {/each}
                </div>
            {/snippet}
        </DateRangePicker.Calendar>
    </DateRangePicker.Content>
</DateRangePicker.Root>
