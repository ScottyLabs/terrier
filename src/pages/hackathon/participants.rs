use dioxus::prelude::*;
use dioxus_free_icons::{
    Icon,
    icons::ld_icons::{LdChevronDown, LdSearch},
};

use crate::components::{Button, ButtonVariant, Dropdown, DropdownOption, TabSwitcher};

#[derive(Clone, Copy, PartialEq)]
enum ParticipantTab {
    Participants,
    Teams,
}

#[component]
pub fn HackathonParticipants(slug: String) -> Element {
    let mut filter_open = use_signal(|| false);
    let mut selected_filters = use_signal(|| vec![]);
    let active_tab = use_signal(|| ParticipantTab::Participants);

    let filter_options = vec![
        DropdownOption {
            label: "CMU Students".to_string(),
            value: "cmu_students".to_string(),
            selected: selected_filters().contains(&"cmu_students".to_string()),
        },
        DropdownOption {
            label: "Organizers".to_string(),
            value: "organizers".to_string(),
            selected: selected_filters().contains(&"organizers".to_string()),
        },
        DropdownOption {
            label: "Sponsors".to_string(),
            value: "sponsors".to_string(),
            selected: selected_filters().contains(&"sponsors".to_string()),
        },
    ];

    let tabs = vec![
        (ParticipantTab::Participants, "Participants".to_string()),
        (ParticipantTab::Teams, "Teams".to_string()),
    ];

    let search_placeholder = match active_tab() {
        ParticipantTab::Participants => "Search participants",
        ParticipantTab::Teams => "Search teams",
    };

    rsx! {
        div { class: "flex flex-col h-full",
            h1 { class: "text-[30px] font-semibold leading-[38px] text-foreground-neutral-primary pt-11 pb-7",
                "Participants"
            }

            div { class: "mb-6",
                TabSwitcher { active_tab, tabs }
            }

            div { class: "flex flex-col gap-7 flex-1 min-h-0",
                div { class: "flex items-center justify-between",
                    div { class: "flex items-center gap-2",
                        // Search bar
                        div { class: "w-[405px] h-10 border border-stroke-neutral-1 rounded-full flex items-center px-3 py-1",
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
                            }
                        }

                        // Filter button and dropdown
                        div { class: "relative",
                            button {
                                class: "bg-foreground-neutral-primary text-white font-semibold text-sm leading-5 rounded-full px-4 py-[9px] flex gap-2 items-center cursor-pointer",
                                onclick: move |_| filter_open.set(!filter_open()),
                                "Filter"
                                Icon {
                                    width: 20,
                                    height: 20,
                                    icon: LdChevronDown,
                                    class: "text-white",
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

                    Button { size: ButtonSize::Compact, "Approve All" }
                }

                // Application list
                div { class: "bg-background-neutral-primary rounded-[20px] p-7 flex flex-col overflow-y-auto flex-1",
                    for i in 0..10 {
                        div {
                            key: "{i}",
                            class: "flex items-center justify-between py-3 border-b border-stroke-neutral-1",
                            p { class: "text-base font-medium leading-6 text-foreground-neutral-primary",
                                "Individual name"
                            }
                            p { class: "text-xs font-medium leading-4 text-foreground-neutral-primary px-4",
                                "Applicant"
                            }
                            div { class: "flex items-center justify-between w-[263px]",
                                Button { "View" }
                                Button { variant: ButtonVariant::Danger, "Deny" }
                                Button { variant: ButtonVariant::Success, "Approve" }
                            }
                        }
                    }
                }
            }
        }
    }
}
