use chrono::NaiveDateTime;
use dioxus::prelude::*;

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum SaveStatus {
    Unsaved,
    Saving,
    Saved,
}

#[component]
pub fn SaveStatusIndicator(status: SaveStatus, last_saved: Option<NaiveDateTime>) -> Element {
    let (dot_color, text, text_color) = match status {
        SaveStatus::Unsaved => (
            "bg-status-danger-foreground",
            "Unsaved",
            "text-status-danger-foreground",
        ),
        SaveStatus::Saving => (
            "bg-status-warning-foreground",
            "Saving...",
            "text-status-warning-foreground",
        ),
        SaveStatus::Saved => (
            "bg-status-success-foreground",
            "Saved",
            "text-status-success-foreground",
        ),
    };

    let tooltip_text = if let Some(timestamp) = last_saved {
        format!("Last saved: {}", timestamp.format("%B %d, %Y at %l:%M %p"))
    } else {
        "Not yet saved".to_string()
    };

    rsx! {
        div {
            class: "flex items-center gap-1.5 px-2.5 py-1.5 relative border border-stroke-neutral-1 rounded",
            title: "{tooltip_text}",

            // Dot indicator
            div { class: "relative shrink-0 w-2.5 h-2.5",
                div { class: "absolute inset-0 rounded-full {dot_color}" }
            }

            // Status text
            p { class: "font-semibold text-sm leading-5 {text_color} whitespace-nowrap",
                "{text}"
            }
        }
    }
}
