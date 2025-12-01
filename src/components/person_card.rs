use dioxus::prelude::*;

use super::{Button, ButtonVariant};

#[component]
pub fn PersonCard(
    name: String,
    role: String,
    on_view: Option<EventHandler<()>>,
    on_deny: Option<EventHandler<()>>,
    on_approve: Option<EventHandler<()>>,
) -> Element {
    rsx! {
        div { class: "flex items-center justify-between py-3 border-b border-stroke-neutral-1",
            p { class: "text-base font-medium leading-6 text-foreground-neutral-primary",
                "{name}"
            }
            p { class: "text-xs font-medium leading-4 text-foreground-neutral-primary px-4",
                "{role}"
            }
            div { class: "flex items-center gap-3",
                if let Some(handler) = on_view {
                    Button { onclick: move |_| handler.call(()), "View" }
                }
                if let Some(handler) = on_deny {
                    Button {
                        variant: ButtonVariant::Danger,
                        onclick: move |_| handler.call(()),
                        "Deny"
                    }
                }
                if let Some(handler) = on_approve {
                    Button {
                        variant: ButtonVariant::Success,
                        onclick: move |_| handler.call(()),
                        "Approve"
                    }
                }
            }
        }
    }
}
