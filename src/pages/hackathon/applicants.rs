use dioxus::prelude::*;
use dioxus_free_icons::{
    Icon,
    icons::ld_icons::{LdChevronDown, LdSearch},
};

use crate::{
    auth::{APPLICANTS_ROLES, hooks::use_require_access_or_redirect},
    components::{
        ApplicationModal, Button, ButtonSize, ButtonVariant, Dropdown, DropdownOption, ModalBase,
        PersonCard, TabSwitcher,
    },
    hackathons::{
        HackathonInfo,
        handlers::applications::{
            ApplicationWithUser, accept_applications, get_all_applications, reject_applications,
        },
    },
    schemas::FormSchema,
};

#[derive(Clone, Copy, PartialEq)]
enum ApplicantTab {
    Individuals,
    Teams,
}

#[component]
pub fn HackathonApplicants(slug: String) -> Element {
    if let Some(no_access) = use_require_access_or_redirect(APPLICANTS_ROLES) {
        return no_access;
    }

    let hackathon = use_context::<Signal<HackathonInfo>>();

    // Parse form schema from hackathon config
    let form_schema = use_memo(move || {
        hackathon
            .read()
            .form_config
            .as_ref()
            .and_then(|config| serde_json::from_value::<FormSchema>(config.clone()).ok())
    });

    let mut filter_open = use_signal(|| false);
    let mut selected_filters = use_signal(|| vec![]);
    let active_tab = use_signal(|| ApplicantTab::Individuals);
    let mut search_query = use_signal(|| String::new());
    let mut selected_application = use_signal(|| None::<ApplicationWithUser>);
    let mut show_approve_all_modal = use_signal(|| false);

    // Fetch applications
    let mut applications_resource = use_resource({
        let slug = slug.clone();
        move || {
            let slug = slug.clone();
            async move {
                let result: Result<Vec<ApplicationWithUser>, _> = get_all_applications(slug).await;
                result.ok()
            }
        }
    });

    // Filter options
    let filter_options = vec![
        DropdownOption {
            label: "CMU Students".to_string(),
            value: "cmu_students".to_string(),
            selected: selected_filters().contains(&"cmu_students".to_string()),
        },
        DropdownOption {
            label: "Draft".to_string(),
            value: "status:draft".to_string(),
            selected: selected_filters().contains(&"status:draft".to_string()),
        },
        DropdownOption {
            label: "Pending".to_string(),
            value: "status:pending".to_string(),
            selected: selected_filters().contains(&"status:pending".to_string()),
        },
        DropdownOption {
            label: "Accepted".to_string(),
            value: "status:accepted".to_string(),
            selected: selected_filters().contains(&"status:accepted".to_string()),
        },
        DropdownOption {
            label: "Rejected".to_string(),
            value: "status:rejected".to_string(),
            selected: selected_filters().contains(&"status:rejected".to_string()),
        },
        DropdownOption {
            label: "Confirmed".to_string(),
            value: "status:confirmed".to_string(),
            selected: selected_filters().contains(&"status:confirmed".to_string()),
        },
        DropdownOption {
            label: "Declined".to_string(),
            value: "status:declined".to_string(),
            selected: selected_filters().contains(&"status:declined".to_string()),
        },
    ];

    let tabs = vec![
        (ApplicantTab::Individuals, "Individuals".to_string()),
        (ApplicantTab::Teams, "Teams".to_string()),
    ];

    let search_placeholder = match active_tab() {
        ApplicantTab::Individuals => "Search individuals",
        ApplicantTab::Teams => "Search teams",
    };

    let show_filter = matches!(active_tab(), ApplicantTab::Individuals);

    // Filter applications based on search and filters
    let filtered_applications = applications_resource.read().as_ref().and_then(|apps| {
        apps.as_ref().map(|app_list| {
            app_list
                .iter()
                .filter(|app| {
                    // Search filter
                    let query = search_query().to_lowercase();
                    let matches_search = query.is_empty()
                        || app
                            .user_name
                            .as_ref()
                            .map(|name| name.to_lowercase().contains(&query))
                            .unwrap_or(false)
                        || app.user_email.to_lowercase().contains(&query);

                    // CMU students filter
                    let is_cmu_filter_active =
                        selected_filters().contains(&"cmu_students".to_string());
                    let matches_cmu_filter =
                        !is_cmu_filter_active || app.user_email.ends_with("@andrew.cmu.edu");

                    // Status filters
                    let status_filters: Vec<String> = selected_filters()
                        .iter()
                        .filter(|f| f.starts_with("status:"))
                        .map(|f| f.strip_prefix("status:").unwrap_or("").to_string())
                        .collect();

                    let matches_status_filter = if status_filters.is_empty() {
                        true
                    } else {
                        status_filters.contains(&app.status)
                    };

                    matches_search && matches_cmu_filter && matches_status_filter
                })
                .cloned()
                .collect::<Vec<_>>()
        })
    });

    // Calculate pending applications in filtered list
    let pending_count = filtered_applications
        .as_ref()
        .map(|apps| apps.iter().filter(|app| app.status == "pending").count())
        .unwrap_or(0);

    let pending_application_ids: Vec<i32> = filtered_applications
        .as_ref()
        .map(|apps| {
            apps.iter()
                .filter(|app| app.status == "pending")
                .map(|app| app.id)
                .collect()
        })
        .unwrap_or_default();

    rsx! {
        div { class: "flex flex-col h-full",
            h1 { class: "text-[30px] font-semibold leading-[38px] text-foreground-neutral-primary pt-11 pb-7",
                "Applicants"
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
                                value: "{search_query}",
                                oninput: move |e| search_query.set(e.value()),
                            }
                        }

                        // Filter button and dropdown
                        if show_filter {
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
                    }

                    Button {
                        size: ButtonSize::Compact,
                        disabled: pending_count == 0,
                        onclick: move |_| show_approve_all_modal.set(true),
                        "Approve All"
                    }
                }

                // Application list
                div { class: "bg-background-neutral-primary rounded-[20px] p-7 flex flex-col overflow-y-auto flex-1",
                    match filtered_applications {
                        Some(apps) => rsx! {
                            if apps.is_empty() {
                                div { class: "flex items-center justify-center h-full",
                                    p { class: "text-foreground-neutral-secondary", "No applicants found" }
                                }
                            } else {
                                for app in apps {
                                    {
                                        let app_id = app.id;
                                        let app_clone = app.clone();
                                        rsx! {
                                            PersonCard {
                                                key: "{app.id}",
                                                name: app.user_name.clone().unwrap_or_else(|| "Unknown".to_string()),
                                                role: app.status.clone(),
                                                on_view: move |_| {
                                                    selected_application.set(Some(app_clone.clone()));
                                                },
                                                on_deny: {
                                                    let slug = slug.clone();
                                                    move |_| {
                                                        let slug = slug.clone();
                                                        spawn(async move {
                                                            let _ = reject_applications(slug, vec![app_id]).await;
                                                            applications_resource.restart();
                                                        });
                                                    }
                                                },
                                                on_approve: {
                                                    let slug = slug.clone();
                                                    move |_| {
                                                        let slug = slug.clone();
                                                        spawn(async move {
                                                            let _ = accept_applications(slug, vec![app_id]).await;
                                                            applications_resource.restart();
                                                        });
                                                    }
                                                },
                                            }
                                        }
                                    }
                                }
                            }
                        },
                        None => rsx! {
                            div { class: "flex items-center justify-center h-full",
                                p { class: "text-foreground-neutral-secondary", "Loading applications..." }
                            }
                        },
                    }
                }
            }
        }

        // Application modal
        if let Some(app) = selected_application() {
            if let Some(schema) = form_schema() {
                {
                    let app_id = app.id;
                    rsx! {
                        ApplicationModal {
                            user_name: app.user_name.unwrap_or_else(|| "Unknown".to_string()),
                            user_email: app.user_email,
                            form_data: app.form_data,
                            form_schema: schema,
                            on_close: move |_| selected_application.set(None),
                            on_deny: {
                                let slug = slug.clone();
                                move |_| {
                                    selected_application.set(None);
                                    let slug = slug.clone();
                                    spawn(async move {
                                        let _ = reject_applications(slug, vec![app_id]).await;
                                        applications_resource.restart();
                                    });
                                }
                            },
                            on_approve: {
                                let slug = slug.clone();
                                move |_| {
                                    selected_application.set(None);
                                    let slug = slug.clone();
                                    spawn(async move {
                                        let _ = accept_applications(slug, vec![app_id]).await;
                                        applications_resource.restart();
                                    });
                                }
                            },
                        }
                    }
                }
            }
        }

        // Approve All confirmation modal
        if show_approve_all_modal() {
            {
                let person_text = if pending_count == 1 { "person" } else { "people" };
                rsx! {
                    ModalBase {
                        on_close: move |_| show_approve_all_modal.set(false),
                        width: "500px",
                        max_height: "auto",

                        div { class: "p-7",
                            h2 { class: "text-2xl font-semibold text-foreground-neutral-primary mb-4",
                                "Approve All Applications"
                            }
                            p { class: "text-base text-foreground-neutral-secondary mb-6",
                                "Are you sure you want to admit {pending_count} {person_text}?"
                            }
                            div { class: "flex gap-3 justify-end",
                                Button {
                                    variant: ButtonVariant::Tertiary,
                                    onclick: move |_| show_approve_all_modal.set(false),
                                    "Cancel"
                                }
                                Button {
                                    variant: ButtonVariant::Default,
                                    onclick: {
                                        let slug = slug.clone();
                                        let ids = pending_application_ids.clone();
                                        move |_| {
                                            show_approve_all_modal.set(false);
                                            let slug = slug.clone();
                                            let ids = ids.clone();
                                            spawn(async move {
                                                let _ = accept_applications(slug, ids).await;
                                                applications_resource.restart();
                                            });
                                        }
                                    },
                                    "Approve {pending_count}"
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}
