use dioxus::prelude::*;

use crate::ui::features::dashboard::{QRModal, QRTile};

use crate::{
    auth::{DASHBOARD_ROLES, hooks::use_require_access_or_redirect},
    domain::hackathons::types::HackathonInfo,
};

#[component]
pub fn HackathonDashboard(slug: String) -> Element {
    if let Some(no_access) = use_require_access_or_redirect(DASHBOARD_ROLES) {
        return no_access;
    }

    let _hackathon = use_context::<Signal<HackathonInfo>>();

    rsx! {
        h1 { class: "text-[30px] font-semibold leading-[38px] text-foreground-neutral-primary pt-11 pb-7",
            "Dashboard"
        }
        // Tile grid
        div { class: "grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-6", QRTile {} }
    }
}
