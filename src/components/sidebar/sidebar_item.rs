use dioxus::prelude::*;
use dioxus_free_icons::{Icon, IconShape};

use crate::Route;

#[component]
pub fn SidebarItem<I: IconShape + Clone + PartialEq + 'static>(
    label: String,
    icon: I,
    to: Route,
) -> Element {
    let current_route = use_route::<Route>();

    // Check if this item's route matches the current route (ignoring slug values)
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
        Link {
            class: "block w-full",
            to,
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
