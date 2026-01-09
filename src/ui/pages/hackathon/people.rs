use dioxus::prelude::*;
use dioxus_free_icons::{
    Icon,
    icons::ld_icons::{LdChevronDown, LdLink, LdSearch},
};

use crate::{
    auth::{HackathonRole, HackathonRoleType, PEOPLE_ROLES, hooks::use_require_access_or_redirect},
    domain::people::handlers::{HackathonPerson, get_hackathon_people, remove_hackathon_person},
    ui::{
        features::people::{PeopleModal, PersonCard},
        foundation::components::{
            ButtonSize, ButtonWithIcon, Dropdown, DropdownOption, TabSwitcher,
        },
    },
};

#[derive(Clone, Copy, PartialEq)]
enum PeopleTab {
    Individuals,
    Teams,
}

#[component]
pub fn HackathonPeople(slug: String) -> Element {
    if let Some(no_access) = use_require_access_or_redirect(PEOPLE_ROLES) {
        return no_access;
    }

    let slug_for_remove = slug.clone();

    let mut filter_open = use_signal(|| false);
    let mut selected_filters = use_signal(Vec::new);
    let active_tab = use_signal(|| PeopleTab::Individuals);
    let mut search_query = use_signal(String::new);
    let mut selected_person = use_signal(|| None::<HackathonPerson>);
    let mut show_modal = use_signal(|| false);

    // Get user's role from context
    let user_role = use_context::<Option<HackathonRole>>();
    let is_admin = user_role
        .as_ref()
        .and_then(|r| r.role_type())
        .map(|rt| rt == HackathonRoleType::Admin)
        .unwrap_or(false);

    // Fetch hackathon people
    let mut people_resource = use_resource(move || {
        let slug = slug.clone();
        async move {
            let result: Result<Vec<HackathonPerson>, _> = get_hackathon_people(slug).await;
            result.ok()
        }
    });

    // Filter options for different roles
    let filter_options = vec![
        DropdownOption {
            label: "Participant".to_string(),
            value: "participant".to_string(),
            selected: selected_filters().contains(&"participant".to_string()),
        },
        DropdownOption {
            label: "Judge".to_string(),
            value: "judge".to_string(),
            selected: selected_filters().contains(&"judge".to_string()),
        },
        DropdownOption {
            label: "Sponsor".to_string(),
            value: "sponsor".to_string(),
            selected: selected_filters().contains(&"sponsor".to_string()),
        },
        DropdownOption {
            label: "Organizer".to_string(),
            value: "organizer".to_string(),
            selected: selected_filters().contains(&"organizer".to_string()),
        },
        DropdownOption {
            label: "Admin".to_string(),
            value: "admin".to_string(),
            selected: selected_filters().contains(&"admin".to_string()),
        },
    ];

    let tabs = vec![
        (PeopleTab::Individuals, "Individuals".to_string()),
        (PeopleTab::Teams, "Teams".to_string()),
    ];

    let search_placeholder = match active_tab() {
        PeopleTab::Individuals => "Search individuals",
        PeopleTab::Teams => "Search teams",
    };

    let show_filter = matches!(active_tab(), PeopleTab::Individuals);

    // Filter people based on search and role filters
    let filtered_people = people_resource.read().as_ref().and_then(|people| {
        people.as_ref().map(|people_list| {
            people_list
                .iter()
                .filter(|person| {
                    // Search filter
                    let query = search_query().to_lowercase();
                    let matches_search = query.is_empty()
                        || person
                            .name
                            .as_ref()
                            .map(|name| name.to_lowercase().contains(&query))
                            .unwrap_or(false)
                        || person.email.to_lowercase().contains(&query);

                    // Role filter
                    let filters = selected_filters();
                    let matches_role_filter =
                        filters.is_empty() || filters.contains(&person.role.to_lowercase());

                    matches_search && matches_role_filter
                })
                .cloned()
                .collect::<Vec<_>>()
        })
    });

    rsx! {
        div { class: "flex flex-col h-full",
            h1 { class: "text-[30px] font-semibold leading-[38px] text-foreground-neutral-primary pt-11 pb-7",
                "People"
            }

            div { class: "mb-6",
                TabSwitcher { active_tab, tabs }
            }

            div { class: "flex flex-col gap-7 flex-1 min-h-0",
                div { class: "flex items-center justify-between max-w-full flex-col gap-7 md:gap-0 md:flex-row",
                    div { class: "flex items-center gap-2",
                        // Search bar
                        div { class: "flex-1 h-10 border border-stroke-neutral-1 rounded-full flex items-center px-3 py-1",
                            Icon {
                                width: 20,
                                height: 20,
                                icon: LdSearch,
                                class: "text-foreground-neutral-tertiary",
                            }
                            input {
                                class: "flex-1 px-2.5 text-sm leading-5 text-foreground-neutral-tertiary outline-none bg-transparent",
                                placeholder: "{search_placeholder}",
                                r#type: "text",
                                value: "{search_query}",
                                oninput: move |e| search_query.set(e.value()),
                            }
                        }

                        // Filter button and dropdown (only on Individuals tab)
                        if show_filter {
                            div { class: "relative",
                                button {
                                    class: "flex-3 bg-foreground-neutral-primary text-white font-semibold text-sm leading-5 rounded-full px-4 py-[9px] flex gap-2 items-center cursor-pointer",
                                    onclick: move |_| filter_open.set(!filter_open()),
                                    "Filter"
                                    Icon {
                                        width: 20,
                                        height: 20,
                                        icon: LdChevronDown,
                                        class: "text-white inline-block",
                                    }
                                }

                                if filter_open() {
                                    div { class: "absolute top-[calc(100%+5px)] right-0 z-10",
                                        Dropdown {
                                            options: filter_options.clone(),
                                            on_change: move |new_values| {
                                                selected_filters.set(new_values);
                                            },
                                        }
                                    }
                                }
                            }
                        }
                    }

                    ButtonWithIcon::<LdLink> { icon: LdLink, size: ButtonSize::Compact, "Create Account Link" }
                }

                // People list
                div { class: "bg-background-neutral-primary rounded-[20px] p-7 flex flex-col overflow-y-auto flex-1",
                    match filtered_people {
                        Some(people) => rsx! {
                            if people.is_empty() {
                                div { class: "flex items-center justify-center h-full",
                                    p { class: "text-foreground-neutral-secondary", "No people found" }
                                }
                            } else {
                                for person in people {
                                    PersonCard {
                                        key: "{person.user_id}",
                                        name: person.name.clone().unwrap_or_else(|| "Unknown".to_string()),
                                        role: format_role(&person.role),
                                        on_view: {
                                            let person = person.clone();
                                            move |_| {
                                                selected_person.set(Some(person.clone()));
                                                show_modal.set(true);
                                            }
                                        },
                                    }
                                }
                            }
                        },
                        None => rsx! {
                            div { class: "flex items-center justify-center h-full",
                                p { class: "text-foreground-neutral-secondary", "Loading people..." }
                            }
                        },
                    }
                }
            }
        }

        // People modal
        if show_modal() {
            if let Some(person) = selected_person() {
                PeopleModal {
                    user_name: person.name.clone().unwrap_or_else(|| "Unknown".to_string()),
                    user_email: person.email.clone(),
                    role: format_role(&person.role),
                    display_name: person.name.clone(),
                    portfolio: None, // TODO: Add to HackathonPerson struct
                    major: None, // TODO: Add to HackathonPerson struct
                    graduation_year: None, // TODO: Add to HackathonPerson struct
                    dietary_restrictions: None, // TODO: Add to HackathonPerson struct
                    shirt_size: None, // TODO: Add to HackathonPerson struct
                    is_admin,
                    on_close: move |_| {
                        show_modal.set(false);
                        selected_person.set(None);
                    },
                    on_remove: {
                        let slug = slug_for_remove.clone();
                        let user_id = person.user_id;
                        move |_| {
                            let slug = slug.clone();
                            spawn(async move {
                                let _ = remove_hackathon_person(slug, user_id).await;
                                people_resource.restart();
                                show_modal.set(false);
                                selected_person.set(None);
                            });
                        }
                    },
                    on_send_message: move |_| {},
                }
            }
        }
    }
}

fn format_role(role: &str) -> String {
    let mut chars = role.chars();
    match chars.next() {
        None => String::new(),
        Some(first) => first.to_uppercase().collect::<String>() + chars.as_str(),
    }
}
