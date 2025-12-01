use dioxus::prelude::*;

use crate::hackathons::HackathonInfo;

#[component]
pub fn HackathonProfile(slug: String) -> Element {
    let _hackathon = use_context::<Signal<HackathonInfo>>();

    rsx! {
        h1 { class: "text-[30px] font-semibold leading-[38px] text-foreground-neutral-primary pt-11 pb-7", "Profile" }
    }
}
