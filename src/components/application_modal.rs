use dioxus::prelude::*;
use dioxus_free_icons::{Icon, icons::ld_icons::LdExternalLink};
use serde_json::Value as JsonValue;
use std::collections::HashMap;

use super::{Button, ButtonSize, ButtonVariant, ModalBase};
use crate::schemas::FormSchema;

#[component]
pub fn ApplicationModal(
    user_name: String,
    user_email: String,
    form_data: JsonValue,
    form_schema: FormSchema,
    on_close: EventHandler<()>,
    on_deny: EventHandler<()>,
    on_approve: EventHandler<()>,
) -> Element {
    // Group fields by section, maintaining schema order
    let grouped_fields = use_memo(move || {
        if let JsonValue::Object(data) = &form_data {
            // Create ordered list of sections based on field order in schema
            let mut section_order: Vec<String> = Vec::new();
            let mut sections: HashMap<String, Vec<(String, JsonValue, u32)>> = HashMap::new();

            for field in form_schema.fields.iter() {
                let section_name = field.section.clone().unwrap_or_else(|| "Other".to_string());

                // Track section order by first appearance
                if !section_order.contains(&section_name) {
                    section_order.push(section_name.clone());
                }

                // If this field has submitted data, add it to the section
                if let Some(value) = data.get(&field.name) {
                    sections.entry(section_name).or_insert_with(Vec::new).push((
                        field.name.clone(),
                        value.clone(),
                        field.order,
                    ));
                }
            }

            // Build with sections in order
            let mut result: Vec<(String, Vec<(String, JsonValue)>)> = Vec::new();
            for section_name in section_order {
                if let Some(mut fields) = sections.remove(&section_name) {
                    // Sort fields within section by their order
                    fields.sort_by_key(|(_, _, order)| *order);
                    let fields_without_order: Vec<(String, JsonValue)> = fields
                        .into_iter()
                        .map(|(name, value, _)| (name, value))
                        .collect();
                    result.push((section_name, fields_without_order));
                }
            }

            result
        } else {
            vec![]
        }
    });

    rsx! {
        ModalBase { on_close, width: "600px", max_height: "none",
            div { class: "flex flex-col max-h-[80vh]",
                // Header
                div { class: "px-8 pt-8 pb-6 shrink-0",
                    h2 { class: "text-2xl font-semibold leading-8 text-foreground-neutral-primary mb-1",
                        "{user_name}"
                    }
                    p { class: "text-sm text-foreground-neutral-secondary", "{user_email}" }
                }

                // Form data sections
                div { class: "px-8 flex-1 overflow-y-auto min-h-0",
                    div { class: "space-y-4 pb-4",
                        for (section_name , fields) in grouped_fields().iter() {
                            div {
                                key: "{section_name}",
                                class: "bg-white rounded-xl p-5",
                                h3 { class: "text-lg font-semibold text-foreground-neutral-primary mb-4",
                                    "{section_name}"
                                }
                                div { class: "grid grid-cols-2 gap-x-6 gap-y-4",
                                    for (field_name , field_value) in fields.iter() {
                                        {render_field_row(field_name, field_value)}
                                    }
                                }
                            }
                        }
                    }
                }

                // Action buttons
                div { class: "px-8 pb-8 pt-4 shrink-0 border-t border-stroke-neutral-1",
                    div { class: "flex items-center justify-end gap-3",
                        Button {
                            size: ButtonSize::Compact,
                            variant: ButtonVariant::Danger,
                            onclick: move |_| on_deny.call(()),
                            "Deny"
                        }
                        Button {
                            size: ButtonSize::Compact,
                            variant: ButtonVariant::Success,
                            onclick: move |_| on_approve.call(()),
                            "Approve"
                        }
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

    // Determine if field should span full width based on content length
    let is_long = formatted_value.len() > 50 || is_url;
    let col_span_class = if is_long { "col-span-2" } else { "" };

    rsx! {
        div { key: "{name}", class: "flex flex-col gap-1 {col_span_class}",
            p { class: "text-xs font-medium text-foreground-neutral-secondary uppercase tracking-wide",
                "{formatted_name}"
            }
            if is_url {
                a {
                    href: "{formatted_value}",
                    target: "_blank",
                    class: "text-sm text-blue-600 hover:text-blue-800 wrap-break-word",
                    "{formatted_value} "
                    span { class: "inline-block relative top-px",
                        Icon { width: 14, height: 14, icon: LdExternalLink }
                    }
                }
            } else {
                p { class: "text-sm text-foreground-neutral-primary", "{formatted_value}" }
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
        JsonValue::Array(arr) => arr
            .iter()
            .filter_map(|v| {
                if let JsonValue::String(s) = v {
                    Some(s.clone())
                } else {
                    None
                }
            })
            .collect::<Vec<_>>()
            .join(", "),
        JsonValue::Null => "Not provided".to_string(),
        JsonValue::Object(_) => "Complex data".to_string(),
    }
}
