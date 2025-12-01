use dioxus::prelude::*;
use dioxus_free_icons::{
    Icon,
    icons::ld_icons::LdExternalLink,
};
use serde_json::Value as JsonValue;

use super::{Button, ButtonVariant, ModalBase};

#[component]
pub fn ApplicationModal(
    user_name: String,
    user_email: String,
    form_data: JsonValue,
    on_close: EventHandler<()>,
    on_deny: EventHandler<()>,
    on_approve: EventHandler<()>,
) -> Element {
    rsx! {
        ModalBase {
            on_close,
            width: "600px",
            max_height: "80vh",

            div { class: "p-8",

                // Header
                div { class: "mb-6",
                    h2 { class: "text-2xl font-semibold leading-8 text-foreground-neutral-primary mb-1",
                        "{user_name}"
                    }
                    p { class: "text-sm text-foreground-neutral-secondary",
                        "{user_email}"
                    }
                }

                // Form data sections
                div { class: "space-y-4 mb-6",
                    if let JsonValue::Object(data) = &form_data {
                        div {
                            key: "{user_email}-form-fields",
                            class: "bg-white rounded-xl p-5 space-y-4",
                            h3 { class: "text-lg font-semibold text-foreground-neutral-primary mb-4",
                                "Application Responses"
                            }
                            for (field_name , field_value) in data.iter() {
                                {render_field_row(field_name, field_value)}
                            }
                        }
                    }
                }

                // Action buttons
                div { class: "flex items-center justify-end gap-3 pt-4 border-t border-stroke-neutral-1",
                    Button {
                        variant: ButtonVariant::Danger,
                        onclick: move |_| on_deny.call(()),
                        "Deny"
                    }
                    Button {
                        variant: ButtonVariant::Success,
                        onclick: move |_| on_approve.call(()),
                        "Approve"
                    }
                }
            }
        }
    }
}

fn render_field_row(name: &str, value: &JsonValue) -> Element {
    let formatted_name = format_field_name(name);
    let formatted_value = format_field_value(value);
    let is_url = formatted_value.starts_with("http://") || formatted_value.starts_with("https://");

    rsx! {
        div {
            key: "{name}",
            class: "flex flex-col gap-1",
            p { class: "text-xs font-medium text-foreground-neutral-secondary uppercase tracking-wide",
                "{formatted_name}"
            }
            if is_url {
                a {
                    href: "{formatted_value}",
                    target: "_blank",
                    class: "text-sm text-blue-600 hover:text-blue-800 flex items-center gap-1",
                    "{formatted_value}"
                    Icon {
                        width: 14,
                        height: 14,
                        icon: LdExternalLink,
                    }
                }
            } else {
                p { class: "text-sm text-foreground-neutral-primary",
                    "{formatted_value}"
                }
            }
        }
    }
}

fn format_field_name(name: &str) -> String {
    name.replace('_', " ")
        .split_whitespace()
        .map(|word| {
            let mut chars = word.chars();
            match chars.next() {
                None => String::new(),
                Some(first) => first.to_uppercase().collect::<String>() + chars.as_str(),
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

fn format_field_value(value: &JsonValue) -> String {
    match value {
        JsonValue::String(s) => s.clone(),
        JsonValue::Number(n) => n.to_string(),
        JsonValue::Bool(b) => if *b { "Yes" } else { "No" }.to_string(),
        JsonValue::Array(arr) => {
            arr.iter()
                .filter_map(|v| {
                    if let JsonValue::String(s) = v {
                        Some(s.clone())
                    } else {
                        None
                    }
                })
                .collect::<Vec<_>>()
                .join(", ")
        }
        JsonValue::Null => "Not provided".to_string(),
        JsonValue::Object(_) => "Complex data".to_string(),
    }
}
