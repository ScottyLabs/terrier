use dioxus::prelude::*;
use dioxus_free_icons::icons::ld_icons::{
    LdBookUser, LdBox, LdCalendar, LdClipboardPen, LdFileText, LdHome, LdMessageSquare, LdQrCode,
    LdSettings, LdUser, LdUsers,
};
use dioxus_free_icons::{Icon, IconShape};

use crate::{
    Route,
    auth::{
        APPLICANTS_ROLES, APPLY_ROLES, CHECKIN_ROLES, DASHBOARD_ROLES, HackathonRole,
        HackathonRoleType, PEOPLE_ROLES, SCHEDULE_ROLES, SETTINGS_ROLES, SUBMISSION_ROLES,
        TEAM_ROLES, has_access,
    },
    domain::applications::handlers::get_application,
    ui::foundation::components::{Header, HeaderSize},
};
// SidebarItem is defined below in this file

#[component]
pub fn Sidebar(
    slug: String,
    hackathon_signal: Signal<crate::domain::hackathons::types::HackathonInfo>,
    role: Option<HackathonRole>,
    application_refresh_trigger: Signal<u32>,
) -> Element {
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

    // Check if user has submitted application (status != "draft")
    let has_submitted_application = application_resource
        .read()
        .as_ref()
        .and_then(|app| app.as_ref())
        .map(|app| app.status != "draft")
        .unwrap_or(false);

    rsx! {
        aside { class: "bg-background-neutral-primary h-[calc(100vh-3rem)] flex flex-col gap-8 items-start p-4 rounded-[20px] shadow-[0px_2px_16px_0px_rgba(0,0,0,0.1)] w-60",
            // Header section with logo and hackathon name
            div { class: "flex flex-col gap-3 items-start w-full",
                div { class: "flex gap-1.5 items-center w-full",
                    Header { size: HeaderSize::Small }
                }
                p { class: "font-medium text-xl leading-7 text-foreground-neutral-primary w-full",
                    "{hackathon_signal.read().name}"
                }
            }

            // Navigation items
            nav { class: "flex flex-col gap-1 items-start w-full",
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
#[component]
pub fn SidebarItem<I: IconShape + Clone + PartialEq + 'static>(
    label: String,
    icon: I,
    to: Route,
) -> Element {
    let current_route = use_route::<Route>();

    // Check if this item's route matches the current route (ignoring slug values)
    #[allow(clippy::match_like_matches_macro)]
    let is_active = match (&current_route, &to) {
        (Route::HackathonDashboard { .. }, Route::HackathonDashboard { .. }) => true,
        (Route::HackathonApplicants { .. }, Route::HackathonApplicants { .. }) => true,
        (Route::HackathonPeople { .. }, Route::HackathonPeople { .. }) => true,
        (Route::HackathonTeam { .. }, Route::HackathonTeam { .. }) => true,
        (Route::HackathonSchedule { .. }, Route::HackathonSchedule { .. }) => true,
        (Route::HackathonMessages { .. }, Route::HackathonMessages { .. }) => true,
        (Route::HackathonSubmission { .. }, Route::HackathonSubmission { .. }) => true,
        (Route::HackathonCheckin { .. }, Route::HackathonCheckin { .. }) => true,
        (Route::HackathonProfile { .. }, Route::HackathonProfile { .. }) => true,
        (Route::HackathonApply { .. }, Route::HackathonApply { .. }) => true,
        (Route::HackathonSettings { .. }, Route::HackathonSettings { .. }) => true,
        _ => false,
    };

    let (container_class, text_class, icon_class) = if is_active {
        (
            "bg-foreground-neutral-primary flex gap-2.5 items-center px-3 py-2 rounded-[24px] w-full cursor-pointer",
            "font-semibold text-sm leading-5 text-white whitespace-nowrap",
            "text-white",
        )
    } else {
        (
            "bg-background-neutral-primary flex gap-2.5 items-center px-3 py-2 rounded-[24px] w-full cursor-pointer",
            "font-semibold text-sm leading-5 text-foreground-neutral-primary whitespace-nowrap",
            "text-foreground-neutral-primary",
        )
    };

    rsx! {
        Link { class: "block w-full", to,
            div { class: "{container_class}",
                Icon {
                    width: 20,
                    height: 20,
                    icon,
                    class: "{icon_class}",
                }
                p { class: "{text_class}", "{label}" }
            }
        }
    }
}
