mod sidebar_item;

use dioxus::prelude::*;
use dioxus_free_icons::icons::ld_icons::{
    LdBookUser, LdBox, LdCalendar, LdClipboardPen, LdFileText, LdHome, LdMessageSquare, LdQrCode,
    LdSettings, LdUser, LdUsers,
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
use sidebar_item::SidebarItem;

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

    rsx! {
        aside {
            class: format!(
                "bg-background-neutral-primary flex flex-col gap-8 items-start {}",
                if *is_mobile.read() {
                    "h-fit w-screen"
                } else {
                    "h-[calc(100vh-3rem)] w-60 p-4 rounded-[20px] shadow-[0px_2px_16px_0px_rgba(0,0,0,0.1)]"
                },
            ),
            div { class: "flex justify-between items-center w-full p-[16px]",
                // Header section with logo and hackathon name
                div {
                    class: format!(
                        "flex flex-col gap-3 items-start {}",
                        if *is_mobile.read() { "w-auto" } else { "w-full" },
                    ),
                    if !*is_mobile.read() {
                        div { class: "flex gap-1.5 items-center w-full",
                            Header { size: HeaderSize::Small }
                        }
                    }
                    p { class: "font-medium text-xl leading-7 text-foreground-neutral-primary w-full",
                        "{hackathon_signal.read().name}"
                    }
                }

                // Hamburger menu for mobile
                if *is_mobile.read() {
                    button {
                        onclick: move |_| {
                            menu_open.toggle();
                            web_sys::console::log_1(&"Menu toggled".into());
                        },
                        class: "bg-background-neutral-secondary-enabled",
                        "🍔"
                    }
                }
            }

            // Navigation items
            nav {
                class: format!(
                    "flex flex-col gap-1 items-start w-full {}",
                    if *is_mobile.read() && !*menu_open.read() { "hidden" } else { "" },
                ),
                if has(DASHBOARD_ROLES) {
                    SidebarItem {
                        label: "Dashboard".to_string(),
                        icon: LdHome,
                        to: Route::HackathonDashboard {
                            slug: slug.clone(),
                        },
                    }
                }
                if has(APPLICANTS_ROLES) {
                    SidebarItem {
                        label: "Applicants".to_string(),
                        icon: LdFileText,
                        to: Route::HackathonApplicants {
                            slug: slug.clone(),
                        },
                    }
                }
                if has(PEOPLE_ROLES) {
                    SidebarItem {
                        label: "People".to_string(),
                        icon: LdBookUser,
                        to: Route::HackathonPeople {
                            slug: slug.clone(),
                        },
                    }
                }
                if has(TEAM_ROLES) && has_submitted_application {
                    SidebarItem {
                        label: "Team".to_string(),
                        icon: LdUsers,
                        to: Route::HackathonTeam {
                            slug: slug.clone(),
                        },
                    }
                }
                if has(SCHEDULE_ROLES) {
                    SidebarItem {
                        label: "Schedule".to_string(),
                        icon: LdCalendar,
                        to: Route::HackathonSchedule {
                            slug: slug.clone(),
                        },
                    }
                }
                SidebarItem {
                    label: "Messages".to_string(),
                    icon: LdMessageSquare,
                    to: Route::HackathonMessages {
                        slug: slug.clone(),
                    },
                }
                if has(SUBMISSION_ROLES) {
                    SidebarItem {
                        label: "Project Submission".to_string(),
                        icon: LdBox,
                        to: Route::HackathonSubmission {
                            slug: slug.clone(),
                        },
                    }
                }
                if has(CHECKIN_ROLES) {
                    SidebarItem {
                        label: "Event Check-In".to_string(),
                        icon: LdQrCode,
                        to: Route::HackathonCheckin {
                            slug: slug.clone(),
                        },
                    }
                }
                SidebarItem {
                    label: "Profile".to_string(),
                    icon: LdUser,
                    to: Route::HackathonProfile {
                        slug: slug.clone(),
                    },
                }
                if has(APPLY_ROLES) {
                    SidebarItem {
                        label: "Apply".to_string(),
                        icon: LdClipboardPen,
                        to: Route::HackathonApply {
                            slug: slug.clone(),
                        },
                    }
                }
            }

            // Settings button at bottom
            if has(SETTINGS_ROLES) {
                div { class: format!("mt-auto w-full {}", if *is_mobile.read() { "hidden" } else { "" }),
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
