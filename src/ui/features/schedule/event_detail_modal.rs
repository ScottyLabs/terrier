use chrono::NaiveDateTime;
use dioxus::prelude::*;
use dioxus_free_icons::{
    Icon,
    icons::ld_icons::{LdClock, LdMapPin, LdX},
};

use crate::domain::hackathons::types::ScheduleEvent;

/// Read-only modal for viewing event details
#[component]
pub fn EventDetailModal(
    event: ScheduleEvent,
    is_admin: bool,
    on_close: EventHandler<()>,
    on_edit: EventHandler<()>,
) -> Element {
    // Colors for organizer avatars
    let colors = [
        "bg-orange-400",
        "bg-purple-500",
        "bg-pink-400",
        "bg-blue-400",
        "bg-green-400",
    ];

    // Format the date and time
    let formatted_date = format_event_datetime(&event.start_time, &event.end_time);

    // Get event type display name
    let event_type_display = match event.event_type.as_str() {
        "hacking" => "Hacking",
        "speaker" => "Speaker",
        "sponsor" => "Sponsor",
        "food" => "Food",
        _ => "Event",
    };

    rsx! {
        // Backdrop
        div {
            class: "fixed inset-0 bg-black/50 z-50 flex items-center justify-center p-4",
            onclick: move |_| on_close.call(()),

            // Modal
            div {
                class: "bg-white rounded-2xl shadow-xl max-w-lg w-full max-h-[90vh] overflow-y-auto",
                onclick: move |e| e.stop_propagation(),

                // Header with close button
                div { class: "flex justify-end p-4 pb-0",
                    button {
                        class: "p-2 hover:bg-gray-100 rounded-full",
                        onclick: move |_| on_close.call(()),
                        Icon { width: 20, height: 20, icon: LdX }
                    }
                }

                // Content
                div { class: "px-6 pb-6",
                    // Title and category
                    div { class: "flex items-start justify-between gap-4 mb-4",
                        h2 { class: "text-2xl font-semibold text-foreground-neutral-primary",
                            "{event.name}"
                        }
                        span { class: "px-3 py-1 bg-foreground-neutral-secondary/10 text-foreground-neutral-primary text-sm rounded-full",
                            "{event_type_display}"
                        }
                    }

                    // Location
                    if let Some(loc) = &event.location {
                        div { class: "flex items-center gap-2 text-foreground-neutral-secondary mb-2",
                            Icon { width: 16, height: 16, icon: LdMapPin }
                            span { class: "text-sm", "{loc}" }
                        }
                    }

                    // Date/Time
                    div { class: "flex items-center gap-2 text-foreground-neutral-secondary mb-6",
                        Icon { width: 16, height: 16, icon: LdClock }
                        span { class: "text-sm", "{formatted_date}" }
                    }

                    // Description
                    if let Some(desc) = &event.description {
                        div { class: "mb-6",
                            h3 { class: "text-sm text-foreground-neutral-tertiary mb-2", "Description" }
                            p { class: "text-foreground-neutral-primary", "{desc}" }
                        }
                    }

                    // Organizers
                    if !event.organizer_ids.is_empty() {
                        div { class: "mb-6",
                            h3 { class: "text-sm text-foreground-neutral-tertiary mb-3", "Organizers" }
                            div { class: "space-y-3",
                                for (idx, _org_id) in event.organizer_ids.iter().enumerate() {
                                    {
                                        let color = colors[idx % colors.len()];
                                        rsx! {
                                            div { class: "flex items-center gap-3",
                                                div { class: "w-8 h-8 rounded-full {color}" }
                                                span { class: "font-medium text-foreground-neutral-primary",
                                                    "Organizer"
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }

                    // Edit button (admin only)
                    if is_admin {
                        div { class: "flex justify-end",
                            button {
                                class: "px-6 py-2.5 bg-foreground-neutral-primary text-white font-semibold text-sm rounded-full",
                                onclick: move |_| on_edit.call(()),
                                "Edit"
                            }
                        }
                    }
                }
            }
        }
    }
}

/// Format event datetime in a readable way
fn format_event_datetime(start: &NaiveDateTime, end: &NaiveDateTime) -> String {
    let start_date = start.format("%A, %B %d").to_string();
    let start_time = start.format("%-I:%M%P").to_string();
    let end_time = end.format("%-I:%M%P").to_string();

    if start.date() == end.date() {
        // Same day
        format!("{} · {} – {}", start_date, start_time, end_time)
    } else {
        // Multi-day event
        let end_date = end.format("%A, %B %d").to_string();
        format!("{} {} – {} {}", start_date, start_time, end_date, end_time)
    }
}
