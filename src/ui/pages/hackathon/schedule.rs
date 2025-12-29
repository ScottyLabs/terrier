use chrono::{Duration, NaiveDate, NaiveDateTime, Timelike};
use dioxus::prelude::*;
use dioxus_free_icons::{
    Icon,
    icons::ld_icons::{LdClock, LdMapPin, LdPlus},
};

use crate::{
    auth::{
        HackathonRole, HackathonRoleType, SCHEDULE_ROLES, hooks::use_require_access_or_redirect,
    },
    domain::{
        applications::handlers::get_user_schedule,
        hackathons::types::{HackathonInfo, ScheduleEvent},
    },
    ui::features::schedule::EventModal,
};

/// Height of one hour in pixels
const HOUR_HEIGHT: f64 = 60.0;

/// Schedule display hours (0 = midnight, 23 = 11pm)
const START_HOUR: u32 = 0;
const END_HOUR: u32 = 24;

#[component]
pub fn HackathonSchedule(slug: String) -> Element {
    if let Some(no_access) = use_require_access_or_redirect(SCHEDULE_ROLES) {
        return no_access;
    }

    // Clone slug for different closures
    let slug_for_resource = slug.clone();
    let slug_for_modal = slug.clone();

    let hackathon = use_context::<Signal<HackathonInfo>>();

    // Get user's role from context
    let user_role = use_context::<Option<HackathonRole>>();
    let is_admin_or_organizer = user_role
        .as_ref()
        .and_then(|r| r.role_type())
        .map(|rt| rt == HackathonRoleType::Admin || rt == HackathonRoleType::Organizer)
        .unwrap_or(false);

    // Get current user ID for event highlighting
    let current_user_id = user_role.as_ref().map(|r| r.user_id);

    // Modal state - None for create, Some(event) for edit
    let mut editing_event = use_signal(|| None::<ScheduleEvent>);
    let mut show_modal = use_signal(|| false);

    // Fetch schedule events
    let mut schedule_resource = use_resource(move || {
        let slug = slug_for_resource.clone();
        async move {
            let result: Result<Vec<ScheduleEvent>, _> = get_user_schedule(slug).await;
            result.ok()
        }
    });

    // Calculate hackathon days
    let hackathon_days = {
        let h = hackathon.read();
        let start = h.start_date.date();
        let end = h.end_date.date();
        get_days_between(start, end)
    };

    // Get current time for "Current" event highlighting
    let now = chrono::Local::now().naive_local();

    // Categorize events
    let events = schedule_resource.read();
    let (current_events, upcoming_events, past_events) =
        categorize_events(events.as_ref().and_then(|e| e.as_ref()), now);

    rsx! {
        div { class: "flex flex-col lg:flex-row gap-6 h-full",
            // Main schedule area
            div { class: "flex-1 flex flex-col",
                // Header with title and add button
                div { class: "flex items-center justify-between pt-11 pb-7",
                    h1 { class: "text-[30px] font-semibold leading-[38px] text-foreground-neutral-primary",
                        "Schedule"
                    }
                    if is_admin_or_organizer {
                        button {
                            class: "flex items-center gap-2 bg-foreground-neutral-primary text-white font-semibold text-sm leading-5 rounded-full px-4 py-2.5",
                            onclick: move |_| {
                                editing_event.set(None);
                                show_modal.set(true);
                            },
                            Icon {
                                width: 16,
                                height: 16,
                                icon: LdPlus,
                                class: "text-white",
                            }
                            "Add new event"
                        }
                    }
                }

                // Calendar grid
                div { class: "bg-background-neutral-primary rounded-[20px] p-4 flex-1 overflow-auto",
                    div { class: "flex min-w-max",
                        // Time column
                        div { class: "w-16 flex-shrink-0",
                            // Header spacer
                            div { class: "h-10 border-b border-stroke-neutral-1" }
                            // Hour labels
                            for hour in START_HOUR..END_HOUR {
                                div {
                                    class: "h-[60px] text-xs text-foreground-neutral-tertiary pr-2 text-right",
                                    style: "line-height: 60px;",
                                    "{format_hour(hour)}"
                                }
                            }
                        }

                        // Day columns
                        for day in hackathon_days.iter() {
                            DayColumn {
                                day: *day,
                                events: events.as_ref().and_then(|e| e.as_ref()).cloned().unwrap_or_default(),
                                current_user_id,
                                is_admin: is_admin_or_organizer,
                                on_edit: move |event: ScheduleEvent| {
                                    editing_event.set(Some(event));
                                    show_modal.set(true);
                                },
                            }
                        }
                    }
                }
            }

            // Sidebar with event details
            div { class: "w-full lg:w-80 flex-shrink-0",
                div { class: "pt-11",
                    // Current events
                    EventSection {
                        title: "Current",
                        events: current_events,
                        highlight: true,
                    }

                    // Upcoming events
                    EventSection {
                        title: "Upcoming",
                        events: upcoming_events,
                        highlight: false,
                    }

                    // Past events
                    EventSection {
                        title: "Past",
                        events: past_events,
                        highlight: false,
                    }
                }
            }
        }

        // Add/Edit Event Modal
        if show_modal() {
            EventModal {
                slug: slug_for_modal.clone(),
                event: editing_event(),
                hackathon_start_date: hackathon.read().start_date.date(),
                on_close: move |_| {
                    show_modal.set(false);
                    editing_event.set(None);
                },
                on_save: move |_| {
                    show_modal.set(false);
                    editing_event.set(None);
                    schedule_resource.restart();
                },
            }
        }
    }
}

#[component]
fn DayColumn(
    day: NaiveDate,
    events: Vec<ScheduleEvent>,
    current_user_id: Option<i32>,
    is_admin: bool,
    on_edit: EventHandler<ScheduleEvent>,
) -> Element {
    let day_name = day.format("%a").to_string();
    let day_num = day.format("%d").to_string();

    // Filter events for this day - include events that span multiple days
    let day_events: Vec<_> = events
        .iter()
        .filter(|e| {
            let start_date = e.start_time.date();
            let end_date = e.end_time.date();
            // Event is visible on this day if day is between start and end (inclusive)
            start_date <= day && day <= end_date
        })
        .cloned()
        .collect();

    rsx! {
        div { class: "flex-1 min-w-[120px] border-l border-stroke-neutral-1",
            // Day header
            div { class: "h-10 border-b border-stroke-neutral-1 text-center py-2",
                span { class: "text-sm font-medium text-foreground-neutral-primary",
                    "{day_name} {day_num}"
                }
            }

            // Time slots with events
            div { class: "relative",
                // Hour grid lines
                for hour in START_HOUR..END_HOUR {
                    div { class: "h-[60px] border-b border-stroke-neutral-1" }
                }

                // Events positioned absolutely
                for event in day_events {
                    EventBlock {
                        event,
                        day,
                        current_user_id,
                        is_admin,
                        on_edit,
                    }
                }
            }
        }
    }
}

#[component]
fn EventBlock(
    event: ScheduleEvent,
    day: NaiveDate,
    current_user_id: Option<i32>,
    is_admin: bool,
    on_edit: EventHandler<ScheduleEvent>,
) -> Element {
    let event_start_date = event.start_time.date();
    let event_end_date = event.end_time.date();

    // Calculate display hours based on which day we're rendering
    let display_start_hour = if day == event_start_date {
        // First day: use actual start time
        event.start_time.hour() as f64 + event.start_time.minute() as f64 / 60.0
    } else {
        // Middle/last day: start at beginning of day
        START_HOUR as f64
    };

    let display_end_hour = if day == event_end_date {
        // Last day: use actual end time
        event.end_time.hour() as f64 + event.end_time.minute() as f64 / 60.0
    } else {
        // First/middle day: end at end of day
        END_HOUR as f64
    };

    let top = (display_start_hour - START_HOUR as f64) * HOUR_HEIGHT;
    let height = (display_end_hour - display_start_hour) * HOUR_HEIGHT;

    // Check if current user is an organizer of this event
    let is_my_event = current_user_id
        .map(|uid| event.organizer_ids.contains(&uid))
        .unwrap_or(false);

    // Color coding based on event_type
    // If user is an organizer, use bright vibrant colors (full opacity)
    // Otherwise, use softer pastel colors
    let (bg_color, text_class) = if is_my_event {
        // Bright, vibrant colors for user's own events
        match event.event_type.as_str() {
            "hacking" => ("bg-blue-600 border-l-4 border-blue-800", "text-white"),
            "speaker" => ("bg-purple-600 border-l-4 border-purple-800", "text-white"),
            "sponsor" => ("bg-amber-500 border-l-4 border-amber-700", "text-white"),
            "food" => ("bg-orange-500 border-l-4 border-orange-700", "text-white"),
            _ => ("bg-green-600 border-l-4 border-green-800", "text-white"), // default
        }
    } else {
        // Soft pastel colors for other events
        match event.event_type.as_str() {
            "hacking" => (
                "bg-blue-50 border-l-4 border-blue-400",
                "text-foreground-neutral-primary",
            ),
            "speaker" => (
                "bg-purple-50 border-l-4 border-purple-400",
                "text-foreground-neutral-primary",
            ),
            "sponsor" => (
                "bg-amber-50 border-l-4 border-amber-400",
                "text-foreground-neutral-primary",
            ),
            "food" => (
                "bg-orange-50 border-l-4 border-orange-400",
                "text-foreground-neutral-primary",
            ),
            _ => (
                "bg-green-50 border-l-4 border-green-400",
                "text-foreground-neutral-primary",
            ), // default
        }
    };

    // Add cursor pointer and hover effect for admins
    let cursor_class = if is_admin {
        "cursor-pointer hover:opacity-80"
    } else {
        ""
    };

    let time_str = format!(
        "{} - {}",
        event.start_time.format("%l:%M%P"),
        event.end_time.format("%l:%M%P")
    );

    let event_for_click = event.clone();

    rsx! {
        div {
            class: "absolute left-1 right-1 rounded-md p-2 overflow-hidden {bg_color} {cursor_class}",
            style: "top: {top}px; height: {height}px;",
            onclick: move |_| {
                if is_admin {
                    on_edit.call(event_for_click.clone());
                }
            },
            p { class: "text-xs font-medium {text_class} truncate", "{event.name}" }
            p { class: "text-xs {text_class} opacity-75 truncate", "{time_str}" }
        }
    }
}

#[component]
fn EventSection(title: String, events: Vec<ScheduleEvent>, highlight: bool) -> Element {
    if events.is_empty() {
        return rsx! {};
    }

    rsx! {
        div { class: "mb-6",
            h2 { class: "text-lg font-semibold text-foreground-neutral-primary mb-4",
                "{title}"
            }
            for event in events {
                EventCard { event, highlight }
            }
        }
    }
}

#[component]
fn EventCard(event: ScheduleEvent, highlight: bool) -> Element {
    let border_class = if highlight {
        "border-l-4 border-green-500"
    } else {
        ""
    };

    let time_str = format!(
        "{} · {} – {}",
        event.start_time.format("%A, %B %d"),
        event.start_time.format("%l:%M"),
        event.end_time.format("%l:%M%P")
    );

    rsx! {
        div { class: "bg-background-neutral-primary rounded-xl p-4 mb-3 {border_class}",
            div { class: "flex items-start justify-between mb-2",
                h3 { class: "font-semibold text-foreground-neutral-primary", "{event.name}" }
                if let Some(role) = &event.visible_to_role {
                    span { class: "text-xs bg-blue-100 text-blue-700 px-2 py-1 rounded",
                        "{role}"
                    }
                }
            }

            // Location
            if let Some(loc) = &event.location {
                div { class: "flex items-center gap-2 text-sm text-foreground-neutral-secondary mb-1",
                    Icon { width: 14, height: 14, icon: LdMapPin }
                    "{loc}"
                }
            }

            // Time
            div { class: "flex items-center gap-2 text-sm text-foreground-neutral-secondary mb-2",
                Icon { width: 14, height: 14, icon: LdClock }
                "{time_str}"
            }

            // Description
            if let Some(desc) = &event.description {
                p { class: "text-sm text-foreground-neutral-tertiary", "{desc}" }
            }
        }
    }
}

fn get_days_between(start: NaiveDate, end: NaiveDate) -> Vec<NaiveDate> {
    let mut days = Vec::new();
    let mut current = start;
    while current <= end {
        days.push(current);
        current += Duration::days(1);
    }
    days
}

fn categorize_events(
    events: Option<&Vec<ScheduleEvent>>,
    now: NaiveDateTime,
) -> (Vec<ScheduleEvent>, Vec<ScheduleEvent>, Vec<ScheduleEvent>) {
    let events = match events {
        Some(e) => e,
        None => return (vec![], vec![], vec![]),
    };

    let mut current = vec![];
    let mut upcoming = vec![];
    let mut past = vec![];

    for event in events {
        if event.start_time <= now && event.end_time >= now {
            current.push(event.clone());
        } else if event.start_time > now {
            upcoming.push(event.clone());
        } else {
            past.push(event.clone());
        }
    }

    (current, upcoming, past)
}

fn format_hour(hour: u32) -> String {
    match hour {
        0 => "12 AM".to_string(),
        1..=11 => format!("{} AM", hour),
        12 => "12 PM".to_string(),
        13..=23 => format!("{} PM", hour - 12),
        _ => format!("{}", hour),
    }
}
