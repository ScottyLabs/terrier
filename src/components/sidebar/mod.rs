mod sidebar_item;

use dioxus::prelude::*;
use dioxus_free_icons::icons::ld_icons::{
    LdBookUser, LdBox, LdCalendar, LdClipboardPen, LdFileText, LdHome, LdMenu, LdMessageSquare,
    LdQrCode, LdSettings, LdUser, LdUsers, LdX,
};

use crate::{
    Route,
    auth::{
        APPLICANTS_ROLES, APPLY_ROLES, CHECKIN_ROLES, DASHBOARD_ROLES, HackathonRole,
        HackathonRoleType, PEOPLE_ROLES, SCHEDULE_ROLES, SETTINGS_ROLES, SUBMISSION_ROLES,
        TEAM_ROLES, has_access,
    },
    components::{Header, HeaderSize},
    hackathons::handlers::applications::get_application,
};
use dioxus_free_icons::Icon;
use sidebar_item::SidebarItem;

/// Shared navigation items component to avoid duplication between mobile and desktop
#[component]
fn NavItems(
    slug: String,
    has_dashboard: bool,
    has_applicants: bool,
    has_people: bool,
    has_team: bool,
    has_schedule: bool,
    has_submission: bool,
    has_checkin: bool,
    has_apply: bool,
    has_settings: bool,
    include_settings_in_nav: bool,
    on_item_click: Option<EventHandler<()>>,
) -> Element {
    let handle_click = move |_| {
        if let Some(handler) = &on_item_click {
            handler.call(());
        }
    };

    rsx! {
        if has_dashboard {
            div { onclick: handle_click,
                SidebarItem {
                    label: "Dashboard".to_string(),
                    icon: LdHome,
                    to: Route::HackathonDashboard {
                        slug: slug.clone(),
                    },
                }
            }
        }
        if has_applicants {
            div { onclick: handle_click,
                SidebarItem {
                    label: "Applicants".to_string(),
                    icon: LdFileText,
                    to: Route::HackathonApplicants {
                        slug: slug.clone(),
                    },
                }
            }
        }
        if has_people {
            div { onclick: handle_click,
                SidebarItem {
                    label: "People".to_string(),
                    icon: LdBookUser,
                    to: Route::HackathonPeople {
                        slug: slug.clone(),
                    },
                }
            }
        }
        if has_team {
            div { onclick: handle_click,
                SidebarItem {
                    label: "Team".to_string(),
                    icon: LdUsers,
                    to: Route::HackathonTeam {
                        slug: slug.clone(),
                    },
                }
            }
        }
        if has_schedule {
            div { onclick: handle_click,
                SidebarItem {
                    label: "Schedule".to_string(),
                    icon: LdCalendar,
                    to: Route::HackathonSchedule {
                        slug: slug.clone(),
                    },
                }
            }
        }
        div { onclick: handle_click,
            SidebarItem {
                label: "Messages".to_string(),
                icon: LdMessageSquare,
                to: Route::HackathonMessages {
                    slug: slug.clone(),
                },
            }
        }
        if has_submission {
            div { onclick: handle_click,
                SidebarItem {
                    label: "Project Submission".to_string(),
                    icon: LdBox,
                    to: Route::HackathonSubmission {
                        slug: slug.clone(),
                    },
                }
            }
        }
        if has_checkin {
            div { onclick: handle_click,
                SidebarItem {
                    label: "Event Check-In".to_string(),
                    icon: LdQrCode,
                    to: Route::HackathonCheckin {
                        slug: slug.clone(),
                    },
                }
            }
        }
        div { onclick: handle_click,
            SidebarItem {
                label: "Profile".to_string(),
                icon: LdUser,
                to: Route::HackathonProfile {
                    slug: slug.clone(),
                },
            }
        }
        if has_apply {
            div { onclick: handle_click,
                SidebarItem {
                    label: "Apply".to_string(),
                    icon: LdClipboardPen,
                    to: Route::HackathonApply {
                        slug: slug.clone(),
                    },
                }
            }
        }
        if has_settings && include_settings_in_nav {
            div { onclick: handle_click,
                SidebarItem {
                    label: "Settings".to_string(),
                    icon: LdSettings,
                    to: Route::HackathonSettings {
                        slug: slug.clone(),
                    },
                }
            }
        }
    }
}

#[component]
pub fn Sidebar(
    slug: String,
    hackathon_signal: Signal<crate::hackathons::HackathonInfo>,
    role: Option<HackathonRole>,
    application_refresh_trigger: Signal<u32>,
) -> Element {
    let is_mobile = use_context::<Signal<bool>>();

    let has = |allowed: &[HackathonRoleType]| {
        role.as_ref()
            .map(|r| has_access(r, allowed))
            .unwrap_or(false)
    };

    // Fetch application to check if submitted
    let slug_for_app = slug.clone();
    let application_resource = use_resource(move || {
        let slug = slug_for_app.clone();
        let _ = application_refresh_trigger.read();
        async move { get_application(slug).await.ok() }
    });

    let mut menu_open = use_signal(|| false);

    // Check if user has submitted application (status != "draft")
    let has_submitted_application = application_resource
        .read()
        .as_ref()
        .and_then(|app| app.as_ref())
        .map(|app| app.status != "draft")
        .unwrap_or(false);

    // Pre-compute role-based visibility flags
    let has_dashboard = has(DASHBOARD_ROLES);
    let has_applicants = has(APPLICANTS_ROLES);
    let has_people = has(PEOPLE_ROLES);
    let has_team = has(TEAM_ROLES) && has_submitted_application;
    let has_schedule = has(SCHEDULE_ROLES);
    let has_submission = has(SUBMISSION_ROLES);
    let has_checkin = has(CHECKIN_ROLES);
    let has_apply = has(APPLY_ROLES);
    let has_settings = has(SETTINGS_ROLES);

    rsx! {
        if *is_mobile.read() {
            // Mobile: Header bar + full-screen overlay when open
            div { class: "bg-background-neutral-primary flex justify-between items-center w-full px-4 py-3",
                p { class: "font-medium text-xl leading-7 text-foreground-neutral-primary",
                    "{hackathon_signal.read().name}"
                }
                button {
                    onclick: move |_| menu_open.set(true),
                    class: "p-2 cursor-pointer",
                    Icon {
                        width: 24,
                        height: 24,
                        icon: LdMenu,
                        class: "text-foreground-neutral-primary",
                    }
                }
            }

            // Full-screen overlay menu
            if *menu_open.read() {
                div { class: "fixed inset-0 z-50 bg-background-neutral-primary flex flex-col",
                    div { class: "flex justify-end p-4",
                        button {
                            onclick: move |_| menu_open.set(false),
                            class: "p-2 cursor-pointer",
                            Icon {
                                width: 24,
                                height: 24,
                                icon: LdX,
                                class: "text-foreground-neutral-primary",
                            }
                        }
                    }
                    nav { class: "flex flex-col gap-2 px-6 py-4",
                        NavItems {
                            slug: slug.clone(),
                            has_dashboard,
                            has_applicants,
                            has_people,
                            has_team,
                            has_schedule,
                            has_submission,
                            has_checkin,
                            has_apply,
                            has_settings,
                            include_settings_in_nav: true,
                            on_item_click: move |_| menu_open.set(false),
                        }
                    }
                }
            }
        } else {
            // Desktop: Original sidebar
            aside { class: "bg-background-neutral-primary flex flex-col gap-8 items-start h-[calc(100vh-3rem)] w-60 p-4 rounded-[20px] shadow-[0px_2px_16px_0px_rgba(0,0,0,0.1)]",
                div { class: "flex justify-between items-center w-full p-[16px]",
                    div { class: "flex flex-col gap-3 items-start w-full",
                        div { class: "flex gap-1.5 items-center w-full",
                            Header { size: HeaderSize::Small }
                        }
                        p { class: "font-medium text-xl leading-7 text-foreground-neutral-primary w-full",
                            "{hackathon_signal.read().name}"
                        }
                    }
                }

                nav { class: "flex flex-col gap-1 items-start w-full",
                    NavItems {
                        slug: slug.clone(),
                        has_dashboard,
                        has_applicants,
                        has_people,
                        has_team,
                        has_schedule,
                        has_submission,
                        has_checkin,
                        has_apply,
                        has_settings,
                        include_settings_in_nav: false,
                        on_item_click: None,
                    }
                }

                if has_settings {
                    div { class: "mt-auto w-full",
                        SidebarItem {
                            label: "Settings".to_string(),
                            icon: LdSettings,
                            to: Route::HackathonSettings {
                                slug: slug.clone(),
                            },
                        }
                    }
                }
            }
        }
    }
}
