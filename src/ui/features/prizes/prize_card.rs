use dioxus::prelude::*;

use crate::domain::prizes::handlers::PrizeInfo;

#[component]
pub fn PrizeCard(prize: PrizeInfo, on_click: Option<EventHandler<()>>) -> Element {
    rsx! {
        div {
            class: "bg-background-neutral-primary rounded-2xl p-6 cursor-pointer hover:shadow-lg transition-shadow",
            onclick: move |_| {
                if let Some(handler) = &on_click {
                    handler.call(());
                }
            },

            h3 { class: "text-lg font-semibold text-foreground-neutral-primary mb-2",
                "{prize.name}"
            }

            if let Some(desc) = &prize.description {
                p { class: "text-sm text-foreground-neutral-secondary line-clamp-4 mb-3",
                    "{desc}"
                }
            }

            div { class: "flex items-center justify-between mt-auto",
                if let Some(cat) = &prize.category {
                    span { class: "text-xs text-foreground-neutral-tertiary", "{cat}" }
                }
                span { class: "text-sm font-medium text-foreground-brand-primary", "{prize.value}" }
            }
        }
    }
}
