use dioxus::prelude::*;

use crate::{
    auth::{TEAM_ROLES, hooks::use_require_access_or_redirect},
    hackathons::HackathonInfo,
};

#[component]
pub fn HackathonTeam(slug: String) -> Element {
    if let Some(no_access) = use_require_access_or_redirect(TEAM_ROLES) {
        return no_access;
    }

    let _hackathon = use_context::<Signal<HackathonInfo>>();

    rsx! {
        h1 { class: "text-[30px] font-semibold leading-[38px] text-foreground-neutral-primary pt-11 pb-7",
            "Team"
        }
    }
}
