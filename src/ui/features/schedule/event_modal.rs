use chrono::{NaiveDate, NaiveDateTime, NaiveTime};
use dioxus::prelude::*;
use dioxus_free_icons::{
    Icon,
    icons::ld_icons::{LdClock, LdMapPin, LdSearch, LdX},
};

use crate::{
    domain::{
        applications::handlers::{
            CreateEventRequest, UpdateEventRequest, create_event, delete_event, update_event,
        },
        hackathons::types::ScheduleEvent,
        people::handlers::{HackathonPerson, get_hackathon_people},
    },
    ui::foundation::modals::base::ModalBase,
};

/// Simple organizer info for the modal
#[derive(Debug, Clone, PartialEq)]
pub struct OrganizerInfo {
    pub user_id: i32,
    pub name: String,
    pub color: String,
}

/// Event modal for creating or editing events
#[component]
pub fn EventModal(
    slug: String,
    event: Option<ScheduleEvent>,
    hackathon_start_date: NaiveDate,
    on_close: EventHandler<()>,
    on_save: EventHandler<()>,
) -> Element {
    let is_edit_mode = event.is_some();

    // Clone slug for different closures
    let slug_for_people = slug.clone();
    let slug_for_save = slug.clone();
    let slug_for_delete = slug.clone();

    // Colors for organizer avatars
    let colors = [
        "bg-orange-400",
        "bg-purple-500",
        "bg-pink-400",
        "bg-blue-400",
        "bg-green-400",
    ];

    // Initialize form state from event if editing, otherwise use defaults
    let initial_name = event.as_ref().map(|e| e.name.clone()).unwrap_or_default();
    let initial_description = event
        .as_ref()
        .and_then(|e| e.description.clone())
        .unwrap_or_default();
    let initial_location = event
        .as_ref()
        .and_then(|e| e.location.clone())
        .unwrap_or_default();
    // Default date to hackathon start date for new events
    let initial_date = event
        .as_ref()
        .map(|e| e.start_time.date().format("%Y-%m-%d").to_string())
        .unwrap_or_else(|| hackathon_start_date.format("%Y-%m-%d").to_string());
    let initial_start_time = event
        .as_ref()
        .map(|e| e.start_time.format("%H:%M").to_string())
        .unwrap_or_default();
    // Default end date to start date if not explicitly different
    let initial_end_date = event
        .as_ref()
        .map(|e| e.end_time.date().format("%Y-%m-%d").to_string())
        .unwrap_or_else(|| hackathon_start_date.format("%Y-%m-%d").to_string());
    let initial_end_time = event
        .as_ref()
        .map(|e| e.end_time.format("%H:%M").to_string())
        .unwrap_or_default();
    let initial_visible_to = event.as_ref().and_then(|e| e.visible_to_role.clone());
    let initial_event_type = event
        .as_ref()
        .map(|e| e.event_type.clone())
        .unwrap_or_else(|| "default".to_string());
    let initial_organizer_ids = event
        .as_ref()
        .map(|e| e.organizer_ids.clone())
        .unwrap_or_default();
    let event_id = event.as_ref().map(|e| e.id);

    // Form state
    let mut name = use_signal(|| initial_name);
    let mut location = use_signal(|| initial_location);
    let mut description = use_signal(|| initial_description);
    let mut start_date = use_signal(|| initial_date);
    let mut start_time = use_signal(|| initial_start_time);
    let mut end_date = use_signal(|| initial_end_date);
    let mut end_time = use_signal(|| initial_end_time);
    let mut visible_to_role = use_signal(|| initial_visible_to);
    let mut event_type = use_signal(|| initial_event_type);
    let mut selected_organizers = use_signal(Vec::<OrganizerInfo>::new);

    // Organizer search
    let mut organizer_search = use_signal(String::new);
    let mut show_organizer_dropdown = use_signal(|| false);

    // Error, loading, and confirmation state
    let mut error = use_signal(|| None::<String>);
    let mut is_saving = use_signal(|| false);
    let mut show_delete_confirm = use_signal(|| false);
    let mut is_deleting = use_signal(|| false);

    // Fetch all people for organizer search and to populate initial organizers
    let people_resource = use_resource(move || {
        let slug = slug_for_people.clone();
        async move {
            let result: Result<Vec<HackathonPerson>, _> = get_hackathon_people(slug).await;
            result.ok()
        }
    });

    // Initialize selected organizers from event when people are loaded
    let _ = use_memo(move || {
        if let Some(people) = people_resource.read().as_ref().and_then(|p| p.as_ref()) {
            let current_orgs = selected_organizers();
            if current_orgs.is_empty() && !initial_organizer_ids.is_empty() {
                let orgs: Vec<OrganizerInfo> = initial_organizer_ids
                    .iter()
                    .filter_map(|id| {
                        people.iter().find(|p| p.user_id == *id).map(|p| {
                            let color_idx = (p.user_id as usize) % colors.len();
                            OrganizerInfo {
                                user_id: p.user_id,
                                name: p.name.clone().unwrap_or_else(|| p.email.clone()),
                                color: colors[color_idx].to_string(),
                            }
                        })
                    })
                    .collect();
                selected_organizers.set(orgs);
            }
        }
    });

    // Filter organizers based on search
    let filtered_organizers = {
        let search = organizer_search().to_lowercase();
        let selected_ids: Vec<i32> = selected_organizers().iter().map(|o| o.user_id).collect();

        people_resource
            .read()
            .as_ref()
            .and_then(|p| p.as_ref())
            .map(|people| {
                people
                    .iter()
                    .filter(|p| {
                        // Only show organizers/admins
                        (p.role == "organizer" || p.role == "admin")
                            && !selected_ids.contains(&p.user_id)
                            && (search.is_empty()
                                || p.name
                                    .as_ref()
                                    .map(|n| n.to_lowercase().contains(&search))
                                    .unwrap_or(false)
                                || p.email.to_lowercase().contains(&search))
                    })
                    .cloned()
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default()
    };

    let handle_save = {
        move |_| {
            let slug = slug_for_save.clone();
            let name_val = name();
            let location_val = location();
            let description_val = description();
            let start_date_val = start_date();
            let start_time_val = start_time();
            let end_date_val = end_date();
            let end_time_val = end_time();
            let visible_to_role_val = visible_to_role();
            let event_type_val = event_type();
            let organizer_ids_val: Vec<i32> =
                selected_organizers().iter().map(|o| o.user_id).collect();

            spawn(async move {
                is_saving.set(true);
                error.set(None);

                // Validate required fields
                if name_val.trim().is_empty() {
                    error.set(Some("Event name is required".to_string()));
                    is_saving.set(false);
                    return;
                }

                if start_date_val.is_empty() {
                    error.set(Some("Date is required".to_string()));
                    is_saving.set(false);
                    return;
                }

                if start_time_val.is_empty() || end_time_val.is_empty() {
                    error.set(Some("Start and end times are required".to_string()));
                    is_saving.set(false);
                    return;
                }

                // Parse date and times
                let parsed_start_date = match NaiveDate::parse_from_str(&start_date_val, "%Y-%m-%d")
                {
                    Ok(d) => d,
                    Err(_) => {
                        error.set(Some("Invalid start date format".to_string()));
                        is_saving.set(false);
                        return;
                    }
                };

                let parsed_end_date = match NaiveDate::parse_from_str(&end_date_val, "%Y-%m-%d") {
                    Ok(d) => d,
                    Err(_) => {
                        error.set(Some("Invalid end date format".to_string()));
                        is_saving.set(false);
                        return;
                    }
                };

                let parsed_start_time = match NaiveTime::parse_from_str(&start_time_val, "%H:%M") {
                    Ok(t) => t,
                    Err(_) => {
                        error.set(Some("Invalid start time format".to_string()));
                        is_saving.set(false);
                        return;
                    }
                };

                let parsed_end_time = match NaiveTime::parse_from_str(&end_time_val, "%H:%M") {
                    Ok(t) => t,
                    Err(_) => {
                        error.set(Some("Invalid end time format".to_string()));
                        is_saving.set(false);
                        return;
                    }
                };

                let start_datetime = NaiveDateTime::new(parsed_start_date, parsed_start_time);
                let end_datetime = NaiveDateTime::new(parsed_end_date, parsed_end_time);

                if let Some(id) = event_id {
                    // Update existing event
                    let request = UpdateEventRequest {
                        id,
                        name: name_val,
                        description: if description_val.is_empty() {
                            None
                        } else {
                            Some(description_val)
                        },
                        location: if location_val.is_empty() {
                            None
                        } else {
                            Some(location_val)
                        },
                        start_time: start_datetime,
                        end_time: end_datetime,
                        visible_to_role: visible_to_role_val,
                        event_type: event_type_val,
                        organizer_ids: organizer_ids_val,
                    };

                    match update_event(slug, id, request).await {
                        Ok(_) => {
                            on_save.call(());
                        }
                        Err(e) => {
                            error.set(Some(e.to_string()));
                        }
                    }
                } else {
                    // Create new event
                    let event_slug = name_val
                        .to_lowercase()
                        .chars()
                        .map(|c| if c.is_alphanumeric() { c } else { '-' })
                        .collect::<String>();

                    let request = CreateEventRequest {
                        name: name_val,
                        slug: event_slug,
                        description: if description_val.is_empty() {
                            None
                        } else {
                            Some(description_val)
                        },
                        location: if location_val.is_empty() {
                            None
                        } else {
                            Some(location_val)
                        },
                        start_time: start_datetime,
                        end_time: end_datetime,
                        visible_to_role: visible_to_role_val,
                        event_type: event_type_val,
                        organizer_ids: organizer_ids_val,
                    };

                    match create_event(slug, request).await {
                        Ok(_) => {
                            on_save.call(());
                        }
                        Err(e) => {
                            error.set(Some(e.to_string()));
                        }
                    }
                }

                is_saving.set(false);
            });
        }
    };

    let handle_delete = {
        move |_| {
            let slug = slug_for_delete.clone();
            if let Some(id) = event_id {
                spawn(async move {
                    is_deleting.set(true);
                    match delete_event(slug, id).await {
                        Ok(_) => {
                            on_save.call(());
                        }
                        Err(e) => {
                            error.set(Some(e.to_string()));
                            show_delete_confirm.set(false);
                        }
                    }
                    is_deleting.set(false);
                });
            }
        }
    };

    rsx! {
        ModalBase {
            on_close: move |_| on_close.call(()),
            div { class: "p-8",
                // Header with name input
                div { class: "mb-6",
                    input {
                        r#type: "text",
                        class: "text-2xl font-semibold text-foreground-neutral-primary bg-transparent border-none outline-none w-full placeholder:text-foreground-neutral-tertiary",
                        placeholder: "Name of event",
                        value: "{name}",
                        oninput: move |e| name.set(e.value()),
                    }

                    // Category/visibility selector
                    div { class: "mt-2 flex gap-2",
                        // Event type
                        select {
                            class: "text-sm border border-stroke-neutral-1 rounded-lg px-3 py-1.5 bg-white",
                            value: "{event_type}",
                            onchange: move |e| event_type.set(e.value()),
                            option { value: "default", "Default" }
                            option { value: "hacking", "Hacking" }
                            option { value: "speaker", "Speaker" }
                            option { value: "sponsor", "Sponsor" }
                            option { value: "food", "Food" }
                        }
                        // Visibility
                        select {
                            class: "text-sm border border-stroke-neutral-1 rounded-lg px-3 py-1.5 bg-white",
                            onchange: move |e| {
                                let val = e.value();
                                visible_to_role.set(if val.is_empty() { None } else { Some(val) });
                            },
                            option { value: "", selected: visible_to_role().is_none(), "Everyone" }
                            option { value: "participant", selected: visible_to_role().as_deref() == Some("participant"), "Participants" }
                            option { value: "sponsor", selected: visible_to_role().as_deref() == Some("sponsor"), "Sponsors" }
                            option { value: "judge", selected: visible_to_role().as_deref() == Some("judge"), "Judges" }
                            option { value: "organizer", selected: visible_to_role().as_deref() == Some("organizer"), "Organizers" }
                        }
                    }
                }

                // Location
                div { class: "flex items-center gap-2 text-foreground-neutral-secondary mb-2",
                    Icon {
                        width: 16,
                        height: 16,
                        icon: LdMapPin,
                    }
                    input {
                        r#type: "text",
                        class: "flex-1 text-sm bg-transparent border-none outline-none placeholder:text-foreground-neutral-tertiary",
                        placeholder: "Add location (optional)",
                        value: "{location}",
                        oninput: move |e| location.set(e.value()),
                    }
                }

                // Date and Time
                div { class: "flex flex-wrap items-center gap-2 text-foreground-neutral-secondary mb-4",
                    Icon {
                        width: 16,
                        height: 16,
                        icon: LdClock,
                    }
                    // Start date/time
                    input {
                        r#type: "date",
                        class: "text-sm border border-stroke-neutral-1 rounded px-2 py-1",
                        value: "{start_date}",
                        oninput: move |e| start_date.set(e.value()),
                    }
                    input {
                        r#type: "time",
                        class: "text-sm border border-stroke-neutral-1 rounded px-2 py-1",
                        value: "{start_time}",
                        oninput: move |e| start_time.set(e.value()),
                    }
                    span { class: "text-sm", "to" }
                    // End date/time
                    input {
                        r#type: "date",
                        class: "text-sm border border-stroke-neutral-1 rounded px-2 py-1",
                        value: "{end_date}",
                        oninput: move |e| end_date.set(e.value()),
                    }
                    input {
                        r#type: "time",
                        class: "text-sm border border-stroke-neutral-1 rounded px-2 py-1",
                        value: "{end_time}",
                        oninput: move |e| end_time.set(e.value()),
                    }
                }

                // Description
                div { class: "mb-6",
                    textarea {
                        class: "w-full h-24 p-3 text-sm border border-stroke-neutral-1 rounded-lg resize-none placeholder:text-foreground-neutral-tertiary",
                        placeholder: "Add event description...",
                        value: "{description}",
                        oninput: move |e| description.set(e.value()),
                    }
                }

                // Organizers section
                div { class: "mb-6",
                    h3 { class: "text-sm font-medium text-foreground-neutral-primary mb-3",
                        "Event Organizers"
                    }

                    // Search input
                    div { class: "relative mb-3",
                        div { class: "flex items-center gap-2 px-3 py-2 border border-stroke-neutral-1 rounded-lg",
                            Icon {
                                width: 16,
                                height: 16,
                                icon: LdSearch,
                            }
                            input {
                                r#type: "text",
                                class: "flex-1 text-sm bg-transparent border-none outline-none placeholder:text-foreground-neutral-tertiary",
                                placeholder: "Search organizers...",
                                value: "{organizer_search}",
                                oninput: move |e| {
                                    organizer_search.set(e.value());
                                    show_organizer_dropdown.set(true);
                                },
                                onfocus: move |_| show_organizer_dropdown.set(true),
                            }
                        }

                        // Dropdown
                        if show_organizer_dropdown() && !filtered_organizers.is_empty() {
                            div { class: "absolute left-0 right-0 top-full mt-1 bg-white border border-stroke-neutral-1 rounded-lg shadow-lg max-h-48 overflow-y-auto z-10",
                                for person in filtered_organizers.iter() {
                                    button {
                                        key: "{person.user_id}",
                                        class: "w-full px-4 py-2 text-left hover:bg-background-neutral-secondary-enabled flex items-center gap-2",
                                        onclick: {
                                            let p = person.clone();
                                            let color_idx = (p.user_id as usize) % colors.len();
                                            move |_| {
                                                let mut orgs = selected_organizers();
                                                orgs.push(OrganizerInfo {
                                                    user_id: p.user_id,
                                                    name: p.name.clone().unwrap_or_else(|| p.email.clone()),
                                                    color: colors[color_idx].to_string(),
                                                });
                                                selected_organizers.set(orgs);
                                                organizer_search.set(String::new());
                                                show_organizer_dropdown.set(false);
                                            }
                                        },
                                        div {
                                            class: "w-6 h-6 rounded-full bg-gray-300",
                                        }
                                        span { class: "text-sm",
                                            "{person.name.clone().unwrap_or_else(|| person.email.clone())}"
                                        }
                                    }
                                }
                            }
                        }
                    }

                    // Selected organizers
                    div { class: "space-y-2",
                        for org in selected_organizers().iter().cloned() {
                            {
                                let org_id = org.user_id;
                                rsx! {
                                    div {
                                        key: "{org.user_id}",
                                        class: "flex items-center justify-between p-3 bg-background-neutral-secondary-enabled rounded-lg",
                                        div { class: "flex items-center gap-3",
                                            // Avatar circle with color
                                            div {
                                                class: "w-8 h-8 rounded-full {org.color}",
                                            }
                                            span { class: "text-sm font-medium text-foreground-neutral-primary",
                                                "{org.name}"
                                            }
                                        }
                                        button {
                                            class: "text-sm text-foreground-neutral-secondary border border-stroke-neutral-1 rounded-full px-3 py-1 hover:bg-background-neutral-secondary-enabled",
                                            onclick: move |_| {
                                                let orgs: Vec<_> = selected_organizers()
                                                    .into_iter()
                                                    .filter(|o| o.user_id != org_id)
                                                    .collect();
                                                selected_organizers.set(orgs);
                                            },
                                            "Remove"
                                        }
                                    }
                                }
                            }
                        }
                    }
                }

                // Error display
                if let Some(err) = error() {
                    div { class: "mb-4 p-3 bg-red-50 border border-red-200 rounded-lg",
                        p { class: "text-red-600 text-sm", "{err}" }
                    }
                }

                // Delete confirmation dialog
                if show_delete_confirm() {
                    div { class: "mb-4 p-4 bg-red-50 border border-red-200 rounded-lg",
                        p { class: "text-red-700 font-medium mb-3", "Are you sure you want to delete this event?" }
                        div { class: "flex gap-2",
                            button {
                                class: "px-4 py-2 text-sm border border-stroke-neutral-1 rounded-full hover:bg-gray-100",
                                onclick: move |_| show_delete_confirm.set(false),
                                "Cancel"
                            }
                            button {
                                class: "px-4 py-2 text-sm bg-red-600 text-white rounded-full hover:bg-red-700",
                                disabled: is_deleting(),
                                onclick: handle_delete,
                                if is_deleting() { "Deleting..." } else { "Yes, Delete" }
                            }
                        }
                    }
                }

                // Action buttons
                div { class: "flex justify-center gap-3",
                    // Only show delete button in edit mode
                    if is_edit_mode && !show_delete_confirm() {
                        button {
                            class: "px-6 py-2 bg-red-500 text-white font-medium rounded-full hover:bg-red-600",
                            onclick: move |_| show_delete_confirm.set(true),
                            "Delete"
                        }
                    }
                    button {
                        class: "px-6 py-2 bg-foreground-neutral-primary text-white font-medium rounded-full hover:opacity-90",
                        disabled: is_saving(),
                        onclick: handle_save,
                        if is_saving() { "Saving..." } else { "Save" }
                    }
                }
            }
        }
    }
}
