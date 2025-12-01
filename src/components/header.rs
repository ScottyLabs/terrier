use crate::Route;
use dioxus::prelude::*;

const LOGO: Asset = asset!("/assets/terrier.svg");

#[derive(Clone, Copy, PartialEq)]
pub enum HeaderSize {
    Large,
    Small,
}

#[component]
pub fn Header(#[props(default = HeaderSize::Small)] size: HeaderSize) -> Element {
    let (logo_class, text_class, gap_class) = match size {
        HeaderSize::Large => ("size-8", "font-semibold text-2xl", "gap-3"),
        HeaderSize::Small => ("w-4", "font-medium text-xs", "gap-2"),
    };

    rsx! {
        Link { to: Route::Home {}, class: "flex items-center {gap_class}",
            img { src: LOGO, alt: "Terrier logo", class: "{logo_class}" }
            h1 { class: "{text_class}", "Terrier" }
        }
    }
}
