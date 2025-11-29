<script lang="ts">
    import { CalendarDate } from "@internationalized/date";
    import {
        CalendarIcon,
        ChevronLeftIcon,
        ChevronRightIcon,
    } from "@untitled-theme/icons-svelte";
    import { DatePicker } from "bits-ui";

    export let value: CalendarDate;
    export let label: string | null = null;
</script>

<DatePicker.Root weekdayFormat="short" fixedWeeks={true} bind:value>
    {#if label}
        <DatePicker.Label class="text-label text-sm font-medium"
            >{label}</DatePicker.Label
        >
    {/if}

    <div
        class="rounded-lg bg-primary h-10 text-input flex w-full select-none items-center px-4 py-2 text-sm"
    >
        <DatePicker.Input>
            {#snippet children({ segments })}
                {#each segments as { part, value }, i (part + i)}
                    <div class="inline-block select-none">
                        {#if part === "literal"}
                            <DatePicker.Segment {part} class="p-1">
                                {value}
                            </DatePicker.Segment>
                        {:else}
                            <DatePicker.Segment {part} class="rounded-5px p-1">
                                {value}
                            </DatePicker.Segment>
                        {/if}
                    </div>
                {/each}
            {/snippet}
        </DatePicker.Input>

        <DatePicker.Trigger
            class="text-input mx-2 inline-flex size-8 items-center justify-center"
        >
            <CalendarIcon class="size-5" />
        </DatePicker.Trigger>
    </div>

    <DatePicker.Content sideOffset={6} class="z-50">
        <DatePicker.Calendar
            class="bg-primary border border-border rounded-lg shadow-lg p-7"
        >
            {#snippet children({ months, weekdays })}
                <DatePicker.Header class="flex items-center justify-between">
                    <DatePicker.PrevButton
                        class="rounded-lg hover:bg-gray-200 inline-flex size-10 items-center justify-center transition-all active:scale-[0.98]"
                    >
                        <ChevronLeftIcon class="size-6" />
                    </DatePicker.PrevButton>

                    <DatePicker.Heading class="text-[15px] font-medium" />

                    <DatePicker.NextButton
                        class="rounded-lg hover:bg-gray-200 inline-flex size-10 items-center justify-center transition-all active:scale-[0.98]"
                    >
                        <ChevronRightIcon class="size-6" />
                    </DatePicker.NextButton>
                </DatePicker.Header>
                <div
                    class="flex flex-col space-y-4 pt-4 sm:flex-row sm:space-x-4 sm:space-y-0"
                >
                    {#each months as month (month.value)}
                        <DatePicker.Grid
                            class="w-full border-collapse select-none space-y-1"
                        >
                            <DatePicker.GridHead>
                                <DatePicker.GridRow
                                    class="mb-1 flex w-full justify-between"
                                >
                                    {#each weekdays as day (day)}
                                        <DatePicker.HeadCell
                                            class="text-muted-foreground font-normal! w-10 rounded-md text-xs"
                                        >
                                            <div>
                                                {day.slice(0, 2)}
                                            </div>
                                        </DatePicker.HeadCell>
                                    {/each}
                                </DatePicker.GridRow>
                            </DatePicker.GridHead>
                            <DatePicker.GridBody>
                                {#each month.weeks as weekDates (weekDates)}
                                    <DatePicker.GridRow class="flex w-full">
                                        {#each weekDates as date (date)}
                                            <DatePicker.Cell
                                                {date}
                                                month={month.value}
                                                class="p-0! relative m-0 size-10 overflow-visible text-center text-sm focus-within:relative focus-within:z-20"
                                            >
                                                <DatePicker.Day
                                                    class="hover:bg-gray-200 data-highlighted:bg-gray-100 data-selected:bg-gray-100 data-disabled:pointer-events-none data-outside-month:pointer-events-none data-selected:font-medium data-selection-end:font-medium data-selection-start:font-medium data-selection-start:focus-visible:ring-2 data-selection-start:focus-visible:ring-offset-2! data-unavailable:line-through group relative inline-flex size-10 items-center justify-center overflow-visible whitespace-nowrap border border-transparent bg-transparent p-0 text-sm font-normal transition-all"
                                                >
                                                    <div
                                                        class="group-data-today:block absolute top-[5px] hidden size-1 rounded-full transition-all"
                                                    ></div>
                                                    {date.day}
                                                </DatePicker.Day>
                                            </DatePicker.Cell>
                                        {/each}
                                    </DatePicker.GridRow>
                                {/each}
                            </DatePicker.GridBody>
                        </DatePicker.Grid>
                    {/each}
                </div>
            {/snippet}
        </DatePicker.Calendar>
    </DatePicker.Content>
</DatePicker.Root>
