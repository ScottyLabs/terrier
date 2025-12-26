use dioxus::prelude::*;

use crate::ui::foundation::components::{Button, ButtonSize, ButtonVariant};

#[component]
pub fn PersonCard(
    name: String,
    role: String,
    on_view: Option<EventHandler<()>>,
    on_deny: Option<EventHandler<()>>,
    on_approve: Option<EventHandler<()>>,
) -> Element {
    // Determine badge style based on status
    let (badge_bg, badge_text, status_text) = match role.to_lowercase().as_str() {
        "accepted" => (
            "bg-background-neutral-secondary-enabled",
            "text-status-success-foreground",
            "Accepted",
        ),
        "pending" => (
            "bg-background-neutral-secondary-enabled",
            "text-status-warning-foreground",
            "Pending",
        ),
        "draft" => (
            "bg-background-neutral-secondary-enabled",
            "text-foreground-neutral-secondary",
            "Draft",
        ),
        "rejected" => (
            "bg-background-neutral-secondary-enabled",
            "text-status-danger-foreground",
            "Rejected",
        ),
        _ => (
            "bg-background-neutral-secondary-enabled",
            "text-foreground-neutral-primary",
            role.as_str(),
        ),
    };

    rsx! {
        div { class: "flex flex-col py-3 border-b border-stroke-neutral-1 gap-3",
            // Name and status row
            div { class: "flex items-center justify-between gap-3",
                // Name
                p { class: "text-base font-medium leading-6 text-foreground-neutral-primary min-w-0 truncate",
                    "{name}"
                }
                // Status badge
                span { class: "px-3 py-1 text-xs font-semibold leading-4 rounded-full shrink-0 {badge_bg} {badge_text}",
                    "{status_text}"
                }
            }
            // Action buttons - wrap on mobile
            div { class: "flex flex-wrap items-center gap-2",
                if let Some(handler) = on_view {
                    Button {
                        size: ButtonSize::Compact,
                        onclick: move |_| handler.call(()),
                        "View"
                    }
                }
                if let Some(handler) = on_deny {
                    Button {
                        size: ButtonSize::Compact,
                        variant: ButtonVariant::Danger,
                        onclick: move |_| handler.call(()),
                        "Deny"
                    }
                }
                if let Some(handler) = on_approve {
                    Button {
                        size: ButtonSize::Compact,
                        variant: ButtonVariant::Success,
                        onclick: move |_| handler.call(()),
                        "Approve"
                    }
                }
            }
        }
    }
}
