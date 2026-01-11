//! Scan check-in page - handles QR code deep links for organizer check-in
//!
//! URL: /h/{slug}/scan/{user_id}
//! This page is opened when an organizer scans a participant's QR code.

use dioxus::prelude::*;
use dioxus_free_icons::{
    Icon,
    icons::ld_icons::{LdCheck, LdChevronLeft, LdLoader, LdUser, LdX},
};

use crate::{
    auth::{PEOPLE_ROLES, hooks::use_require_access_or_redirect},
    domain::applications::handlers::{
        ParticipantInfo, get_participant_info, get_user_schedule, organizer_checkin,
    },
    domain::hackathons::types::ScheduleEvent,
};

/// Scan check-in page - shows participant info and allows organizer to check them in
#[component]
pub fn HackathonScan(slug: String, user_id: i32) -> Element {
    // Require organizer/admin access
    if let Some(no_access) = use_require_access_or_redirect(PEOPLE_ROLES) {
        return no_access;
    }

    let nav = use_navigator();
    let slug_for_participant = slug.clone();
    let slug_for_events = slug.clone();
    let slug_for_checkin = slug.clone();

    // State
    let mut selected_event_id = use_signal(|| None::<i32>);
    let mut is_checking_in = use_signal(|| false);
    let mut check_in_result = use_signal(|| None::<Result<(), String>>);

    // Fetch participant info
    let participant_resource = use_resource(move || {
        let slug = slug_for_participant.clone();
        async move { get_participant_info(slug, user_id).await.ok() }
    });

    // Fetch events for selection
    let events_resource = use_resource(move || {
        let slug = slug_for_events.clone();
        async move { get_user_schedule(slug).await.ok().unwrap_or_default() }
    });

    let participant = participant_resource.read().clone().flatten();
    let events: Vec<ScheduleEvent> = events_resource.read().clone().unwrap_or_default();

    // Filter to show only QR scan events (not self-checkin)
    let qr_events: Vec<ScheduleEvent> = events
        .into_iter()
        .filter(|e| e.checkin_type != "self_checkin")
        .collect();

    // Handle check-in
    let mut do_checkin = move |event_id: i32| {
        let slug = slug_for_checkin.clone();
        is_checking_in.set(true);
        spawn(async move {
            match organizer_checkin(slug, event_id, user_id).await {
                Ok(()) => {
                    check_in_result.set(Some(Ok(())));
                }
                Err(e) => {
                    let error_str = e.to_string();
                    if error_str.contains("ALREADY_CHECKED_IN") {
                        check_in_result.set(Some(Err("Already checked in".to_string())));
                    } else {
                        check_in_result.set(Some(Err(format!("Error: {}", error_str))));
                    }
                }
            }
            is_checking_in.set(false);
        });
    };

    rsx! {
        div { class: "min-h-screen bg-background-neutral-secondary p-4 md:p-8",
            div { class: "max-w-lg mx-auto",
                // Header with back button
                div { class: "flex items-center gap-3 mb-6",
                    button {
                        class: "p-2 rounded-xl hover:bg-background-neutral-primary transition-colors",
                        onclick: move |_| nav.go_back(),
                        Icon { width: 20, height: 20, icon: LdChevronLeft }
                    }
                    h1 { class: "text-xl font-semibold text-foreground-neutral-primary",
                        "Scan Check-In"
                    }
                }

                // Main card
                div { class: "bg-background-neutral-primary rounded-2xl p-6 shadow-sm",
                    // Participant info
                    if let Some(ref info) = participant {
                        div { class: "flex items-center gap-4 pb-6 border-b border-stroke-neutral-1",
                            div { class: "w-16 h-16 rounded-full bg-background-neutral-secondary flex items-center justify-center",
                                Icon {
                                    width: 32,
                                    height: 32,
                                    icon: LdUser,
                                    class: "text-foreground-neutral-tertiary",
                                }
                            }
                            div { class: "flex-1",
                                p { class: "text-lg font-semibold text-foreground-neutral-primary",
                                    "{info.name}"
                                }
                                p { class: "text-sm text-foreground-neutral-secondary",
                                    "{info.email}"
                                }
                                p { class: "text-xs text-foreground-neutral-tertiary mt-1",
                                    "ID: {info.user_id}"
                                }
                            }
                        }
                    } else {
                        div { class: "flex items-center justify-center py-8",
                            Icon {
                                width: 24,
                                height: 24,
                                icon: LdLoader,
                                class: "text-foreground-neutral-tertiary animate-spin",
                            }
                        }
                    }

                    // Success/Error result
                    if let Some(result) = check_in_result() {
                        match result {
                            Ok(()) => rsx! {
                                div { class: "mt-6 p-4 bg-green-50 border border-green-200 rounded-xl flex items-center gap-3",
                                    div { class: "w-10 h-10 rounded-full bg-green-100 flex items-center justify-center",
                                        Icon {
                                            width: 20,
                                            height: 20,
                                            icon: LdCheck,
                                            class: "text-green-600",
                                        }
                                    }
                                    div {
                                        p { class: "font-medium text-green-800", "Check-in successful!" }
                                        p { class: "text-sm text-green-600", "Participant has been checked in." }
                                    }
                                }
                                // Done button
                                button {
                                    class: "w-full mt-4 py-3 px-4 rounded-xl bg-foreground-neutral-primary text-white font-medium",
                                    onclick: move |_| nav.go_back(),
                                    "Done"
                                }
                            },
                            Err(ref msg) => rsx! {
                                div { class: "mt-6 p-4 bg-red-50 border border-red-200 rounded-xl flex items-center gap-3",
                                    div { class: "w-10 h-10 rounded-full bg-red-100 flex items-center justify-center",
                                        Icon {
                                            width: 20,
                                            height: 20,
                                            icon: LdX,
                                            class: "text-red-600",
                                        }
                                    }
                                    div {
                                        p { class: "font-medium text-red-800", "Check-in failed" }
                                        p { class: "text-sm text-red-600", "{msg}" }
                                    }
                                }
                                // Try again button
                                button {
                                    class: "w-full mt-4 py-3 px-4 rounded-xl border border-stroke-neutral-1 font-medium",
                                    onclick: move |_| check_in_result.set(None),
                                    "Try Again"
                                }
                            },
                        }
                    } else {
                        // Event selection
                        div { class: "mt-6",
                            p { class: "text-sm font-medium text-foreground-neutral-primary mb-3",
                                "Select event to check in:"
                            }

                            if qr_events.is_empty() {
                                p { class: "text-sm text-foreground-neutral-tertiary py-4 text-center",
                                    "No events requiring QR scan check-in"
                                }
                            } else {
                                div { class: "flex flex-col gap-2",
                                    for event in qr_events.iter() {
                                        {
                                            let event_id = event.id;
                                            let is_selected = selected_event_id() == Some(event_id);
                                            let is_already_checked_in = event.is_checked_in;
                                            rsx! {
                                                button {
                                                    class: if is_selected { "w-full p-4 rounded-xl border-2 border-blue-500 bg-blue-50 text-left" } else if is_already_checked_in { "w-full p-4 rounded-xl border border-green-200 bg-green-50 text-left opacity-60" } else { "w-full p-4 rounded-xl border border-stroke-neutral-1 bg-background-neutral-secondary text-left hover:bg-background-neutral-secondary-hover" },
                                                    disabled: is_already_checked_in,
                                                    onclick: move |_| selected_event_id.set(Some(event_id)),
                                                    div { class: "flex items-center justify-between",
                                                        div {
                                                            p { class: "font-medium text-foreground-neutral-primary", "{event.name}" }
                                                            if is_already_checked_in {
                                                                p { class: "text-xs text-green-600 mt-1", "Already checked in" }
                                                            }
                                                        }
                                                        if let Some(pts) = event.points {
                                                            span { class: "text-xs bg-background-neutral-primary px-2 py-1 rounded-full",
                                                                "{pts} pts"
                                                            }
                                                        }
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }

                        // Check-in button
                        if selected_event_id().is_some() && participant.is_some() {
                            button {
                                class: "w-full mt-6 py-3 px-4 rounded-xl bg-foreground-neutral-primary text-white font-medium flex items-center justify-center gap-2 disabled:opacity-50",
                                disabled: is_checking_in(),
                                onclick: move |_| {
                                    if let Some(event_id) = selected_event_id() {
                                        do_checkin(event_id);
                                    }
                                },
                                if is_checking_in() {
                                    Icon {
                                        width: 20,
                                        height: 20,
                                        icon: LdLoader,
                                        class: "animate-spin",
                                    }
                                    "Checking in..."
                                } else {
                                    Icon {
                                        width: 20,
                                        height: 20,
                                        icon: LdCheck,
                                    }
                                    "Check In"
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}
