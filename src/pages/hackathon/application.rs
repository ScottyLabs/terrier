use crate::backend;
use dioxus::prelude::*;
use std::collections::HashMap;
use dioxus_logger::tracing;

#[component]
pub fn Application(id: String) -> Element {
    // Fetch form definition
    let id_clone = id.clone();
    let form_res = use_resource(move || {
        let id = id_clone.clone();
        async move { backend::get_hackathon_form(id).await }
    });

    // Track field values dynamically
    let mut values = use_signal(|| HashMap::<String, String>::new());

    // Track validation errors
    let mut errors = use_signal(|| HashMap::<String, String>::new());

    // Track if form was submitted (to show errors)
    let mut submitted = use_signal(|| false);

    let mut submit_application = use_action(backend::submit_hackathon_application);

    rsx! {
        div { class: "space-y-8",
            h1 { class: "text-2xl font-bold", "Hackathon Application" }
            p { class: "text-sm text-gray-600", "Hackathon ID: {id}" }
            {
                match &*form_res.read_unchecked() {
                    Some(Ok(json)) => {
                        let json_clone = json.clone();
                        rsx! {
                            form {
                                onsubmit: move |ev: FormEvent| {
                                    ev.prevent_default();
                                    submitted.set(true);

                                    // Validate form
                                    let mut validation_errors = HashMap::new();
                                    let empty_vec = vec![];
                                    let sections = json_clone.as_array().unwrap_or(&empty_vec);

                                    for section in sections {
                                        if let Some(fields) = section.get("fields").and_then(|v| v.as_array()) {
                                            for field in fields {
                                                let field_id = field.get("id").and_then(|v| v.as_str()).unwrap_or("");
                                                let required = field.get("required").and_then(|v| v.as_bool()).unwrap_or(false);
                                                let question = field.get("question").and_then(|v| v.as_str()).unwrap_or(field_id);
                                                let field_type = field.get("type").and_then(|v| v.as_str()).unwrap_or("single-line-text");

                                                // Check if field should be visible (condition check)
                                                let show = if let Some(cond) = field.get("condition") {
                                                    let target = cond.get("id").and_then(|v| v.as_str());
                                                    let cond_value = cond.get("value").and_then(|v| v.as_str());
                                                    if let (Some(t), Some(val)) = (target, cond_value) {
                                                        values.read().get(t).map(|v| v == val).unwrap_or(false)
                                                    } else {
                                                        true
                                                    }
                                                } else {
                                                    true
                                                };

                                                // Only validate visible required fields
                                                if show && required {
                                                    let value = values.read().get(field_id).cloned().unwrap_or_default();

                                                    if field_type == "multi-checkbox" {
                                                        // Check if at least one checkbox is selected
                                                        let has_selection = values.read().keys().any(|k| k.starts_with(&format!("{}__", field_id)));
                                                        if !has_selection {
                                                            validation_errors.insert(field_id.to_string(), format!("{} is required", question));
                                                        }
                                                    } else if field_type == "checkbox" {
                                                        if value != "true" {
                                                            validation_errors.insert(field_id.to_string(), format!("{} must be checked", question));
                                                        }
                                                    } else {
                                                        if value.trim().is_empty() {
                                                            validation_errors.insert(field_id.to_string(), format!("{} is required", question));
                                                        }
                                                    }
                                                }
                                            }
                                        }
                                    }

                                    if validation_errors.is_empty() {
                                        let data = values.read().clone();
                                        let slug = id.clone();
                                        tracing::info!("Form submitted: {:?}", data);
                                        errors.set(HashMap::new());

                                        spawn(async move {
                                            submit_application.call(slug, data).await;
                                        });
                                    } else {
                                        errors.set(validation_errors);
                                        tracing::warn!("Form validation failed");
                                    }
                                },

                                // Show general error if validation failed
                                if submitted() && !errors.read().is_empty() {
                                    div { class: "bg-red-50 border border-red-200 text-red-700 px-4 py-3 rounded mb-4",
                                        p { class: "font-semibold", "Please correct the following errors:" }
                                        ul { class: "list-disc list-inside mt-2",
                                            for (_, error_msg) in errors.read().iter() {
                                                li { "{error_msg}" }
                                            }
                                        }
                                    }
                                }

                                for section in json.as_array().unwrap_or(&vec![]) {
                                    {
                                        let section_name = section.get("name")
                                            .and_then(|v| v.as_str())
                                            .unwrap_or("Section")
                                            .to_string();

                                        let fields = section.get("fields")
                                            .and_then(|v| v.as_array())
                                            .cloned()
                                            .unwrap_or_default();

                                        rsx! {
                                            section { key: "{section_name}", class: "border rounded p-4 space-y-4",
                                                h2 { class: "text-xl font-semibold", "{section_name}" }
                                                for field in fields.iter() {
                                                    {
                                                        // Check condition
                                                        let show = if let Some(cond) = field.get("condition") {
                                                            let target = cond.get("id").and_then(|v| v.as_str());
                                                            let cond_value = cond.get("value").and_then(|v| v.as_str());
                                                            if let (Some(t), Some(val)) = (target, cond_value) {
                                                                values.read().get(t).map(|v| v == val).unwrap_or(false)
                                                            } else {
                                                                true
                                                            }
                                                        } else {
                                                            true
                                                        };

                                                        if !show {
                                                            None
                                                        } else {
                                                            let field_id = field.get("id").and_then(|v| v.as_str()).unwrap_or("").to_string();
                                                            let question = field.get("question").and_then(|v| v.as_str()).unwrap_or(&field_id).to_string();
                                                            let field_type = field.get("type").and_then(|v| v.as_str()).unwrap_or("single-line-text");
                                                            let description = field.get("description").and_then(|v| v.as_str()).map(|s| s.to_string());
                                                            let required = field.get("required").and_then(|v| v.as_bool()).unwrap_or(false);
                                                            let options = field.get("options").and_then(|v| v.as_array()).cloned();
                                                            let has_error = errors.read().contains_key(&field_id);

                                                            Some(match field_type {
                                                                "single-line-text" => {
                                                                    let field_id_clone = field_id.clone();
                                                                    rsx! {
                                                                        div { key: "{field_id}", class: "mb-4",
                                                                            label { r#for: "{field_id}", class: "block mb-1", "{question}" }
                                                                            if required { span { class: "text-xs text-red-500", " *" } }
                                                                            input {
                                                                                r#type: "text",
                                                                                id: "{field_id}",
                                                                                name: "{field_id}",
                                                                                class: if has_error { "w-full border-2 border-red-500 rounded px-3 py-2" } else { "w-full border rounded px-3 py-2" },
                                                                                value: "{values.read().get(&field_id).cloned().unwrap_or_default()}",
                                                                                oninput: move |evt| {
                                                                                    values.write().insert(field_id_clone.clone(), evt.value());
                                                                                    // Clear error when user types
                                                                                    if errors.read().contains_key(&field_id_clone) {
                                                                                        errors.write().remove(&field_id_clone);
                                                                                    }
                                                                                }
                                                                            }
                                                                            if let Some(desc) = &description {
                                                                                p { class: "text-xs text-gray-500 mt-1", "{desc}" }
                                                                            }
                                                                            if has_error {
                                                                                p { class: "text-xs text-red-500 mt-1", "{errors.read().get(&field_id).unwrap()}" }
                                                                            }
                                                                        }
                                                                    }
                                                                },
                                                                "long-response" => {
                                                                    let field_id_clone = field_id.clone();
                                                                    rsx! {
                                                                        div { key: "{field_id}", class: "mb-4",
                                                                            label { r#for: "{field_id}", class: "block mb-1", "{question}" }
                                                                            if required { span { class: "text-xs text-red-500", " *" } }
                                                                            textarea {
                                                                                id: "{field_id}",
                                                                                name: "{field_id}",
                                                                                class: if has_error { "w-full border-2 border-red-500 rounded px-3 py-2" } else { "w-full border rounded px-3 py-2" },
                                                                                rows: "4",
                                                                                value: "{values.read().get(&field_id).cloned().unwrap_or_default()}",
                                                                                oninput: move |evt| {
                                                                                    values.write().insert(field_id_clone.clone(), evt.value());
                                                                                    if errors.read().contains_key(&field_id_clone) {
                                                                                        errors.write().remove(&field_id_clone);
                                                                                    }
                                                                                }
                                                                            }
                                                                            if let Some(desc) = &description {
                                                                                p { class: "text-xs text-gray-500 mt-1", "{desc}" }
                                                                            }
                                                                            if has_error {
                                                                                p { class: "text-xs text-red-500 mt-1", "{errors.read().get(&field_id).unwrap()}" }
                                                                            }
                                                                        }
                                                                    }
                                                                },
                                                                "dropdown" => {
                                                                    let field_id_clone = field_id.clone();
                                                                    rsx! {
                                                                        div { key: "{field_id}", class: "mb-4",
                                                                            label { r#for: "{field_id}", class: "block mb-1", "{question}" }
                                                                            if required { span { class: "text-xs text-red-500", " *" } }
                                                                            select {
                                                                                id: "{field_id}",
                                                                                name: "{field_id}",
                                                                                class: if has_error { "w-full border-2 border-red-500 rounded px-3 py-2" } else { "w-full border rounded px-3 py-2" },
                                                                                value: "{values.read().get(&field_id).cloned().unwrap_or_default()}",
                                                                                oninput: move |evt| {
                                                                                    values.write().insert(field_id_clone.clone(), evt.value());
                                                                                    if errors.read().contains_key(&field_id_clone) {
                                                                                        errors.write().remove(&field_id_clone);
                                                                                    }
                                                                                },
                                                                                option { value: "", "-- Select --" }
                                                                                for opt in options.unwrap_or_default().iter() {
                                                                                    {
                                                                                        opt.as_str().map(|text| {
                                                                                            let text = text.to_string();
                                                                                            rsx! {
                                                                                                option { key: "{text}", value: "{text}", "{text}" }
                                                                                            }
                                                                                        })
                                                                                    }
                                                                                }
                                                                            }
                                                                            if let Some(desc) = &description {
                                                                                p { class: "text-xs text-gray-500 mt-1", "{desc}" }
                                                                            }
                                                                            if has_error {
                                                                                p { class: "text-xs text-red-500 mt-1", "{errors.read().get(&field_id).unwrap()}" }
                                                                            }
                                                                        }
                                                                    }
                                                                },
                                                                "multi-checkbox" => {
                                                                    rsx! {
                                                                        fieldset { key: "{field_id}", class: if has_error { "mb-4 border-2 border-red-500 rounded p-2" } else { "mb-4" },
                                                                            legend { class: "block mb-2 font-medium", "{question}" }
                                                                            if required { span { class: "text-xs text-red-500", " *" } }
                                                                            for opt in options.unwrap_or_default().iter() {
                                                                                {
                                                                                    opt.as_str().map(|text| {
                                                                                        let opt_id = format!("{}__{}", field_id, text);
                                                                                        let opt_id_clone = opt_id.clone();
                                                                                        let text = text.to_string();
                                                                                        let text_clone = text.clone();
                                                                                        let field_id_for_clear = field_id.clone();
                                                                                        rsx! {
                                                                                            div { key: "{opt_id}", class: "flex items-center mb-2",
                                                                                                input {
                                                                                                    r#type: "checkbox",
                                                                                                    id: "{opt_id}",
                                                                                                    class: "mr-2",
                                                                                                    checked: values.read().contains_key(&opt_id),
                                                                                                    oninput: move |_| {
                                                                                                        let mut w = values.write();
                                                                                                        if w.contains_key(&opt_id_clone) {
                                                                                                            w.remove(&opt_id_clone);
                                                                                                        } else {
                                                                                                            w.insert(opt_id_clone.clone(), text_clone.clone());
                                                                                                        }
                                                                                                        drop(w);
                                                                                                        if errors.read().contains_key(&field_id_for_clear) {
                                                                                                            errors.write().remove(&field_id_for_clear);
                                                                                                        }
                                                                                                    }
                                                                                                }
                                                                                                label { r#for: "{opt_id}", "{text}" }
                                                                                            }
                                                                                        }
                                                                                    })
                                                                                }
                                                                            }
                                                                            if let Some(desc) = &description {
                                                                                p { class: "text-xs text-gray-500 mt-1", "{desc}" }
                                                                            }
                                                                            if has_error {
                                                                                p { class: "text-xs text-red-500 mt-1", "{errors.read().get(&field_id).unwrap()}" }
                                                                            }
                                                                        }
                                                                    }
                                                                },
                                                                "checkbox" => {
                                                                    let field_id_clone = field_id.clone();
                                                                    rsx! {
                                                                        div { key: "{field_id}", class: "mb-4",
                                                                            div { class: if has_error { "flex items-center border-2 border-red-500 rounded p-2" } else { "flex items-center" },
                                                                                input {
                                                                                    r#type: "checkbox",
                                                                                    id: "{field_id}",
                                                                                    class: "mr-2",
                                                                                    checked: values.read().get(&field_id).map(|v| v == "true").unwrap_or(false),
                                                                                    oninput: move |_| {
                                                                                        let mut w = values.write();
                                                                                        let cur = w.get(&field_id_clone).map(|v| v == "true").unwrap_or(false);
                                                                                        w.insert(field_id_clone.clone(), (!cur).to_string());
                                                                                        drop(w);
                                                                                        if errors.read().contains_key(&field_id_clone) {
                                                                                            errors.write().remove(&field_id_clone);
                                                                                        }
                                                                                    }
                                                                                }
                                                                                label { r#for: "{field_id}", "{question}" }
                                                                                if required { span { class: "text-xs text-red-500 ml-1", " *" } }
                                                                            }
                                                                            if let Some(desc) = &description {
                                                                                p { class: "text-xs text-gray-500 ml-6 mt-1", "{desc}" }
                                                                            }
                                                                            if has_error {
                                                                                p { class: "text-xs text-red-500 ml-6 mt-1", "{errors.read().get(&field_id).unwrap()}" }
                                                                            }
                                                                        }
                                                                    }
                                                                },
                                                                "signature" => {
                                                                    let field_id_clone = field_id.clone();
                                                                    rsx! {
                                                                        div { key: "{field_id}", class: "mb-4",
                                                                            label { r#for: "{field_id}", class: "block mb-1", "{question}" }
                                                                            if required { span { class: "text-xs text-red-500", " *" } }
                                                                            input {
                                                                                r#type: "text",
                                                                                placeholder: "Type full name as signature",
                                                                                id: "{field_id}",
                                                                                class: if has_error { "w-full border-2 border-red-500 rounded px-3 py-2" } else { "w-full border rounded px-3 py-2" },
                                                                                value: "{values.read().get(&field_id).cloned().unwrap_or_default()}",
                                                                                oninput: move |evt| {
                                                                                    values.write().insert(field_id_clone.clone(), evt.value());
                                                                                    if errors.read().contains_key(&field_id_clone) {
                                                                                        errors.write().remove(&field_id_clone);
                                                                                    }
                                                                                }
                                                                            }
                                                                            if let Some(desc) = &description {
                                                                                p { class: "text-xs text-gray-500 mt-1", "{desc}" }
                                                                            }
                                                                            if has_error {
                                                                                p { class: "text-xs text-red-500 mt-1", "{errors.read().get(&field_id).unwrap()}" }
                                                                            }
                                                                        }
                                                                    }
                                                                },
                                                                _ => rsx! {
                                                                    div { key: "{field_id}", "Unsupported field type: {field_type}" }
                                                                },
                                                            })
                                                        }
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                                button {
                                    r#type: "submit",
                                    class: "mt-6 px-6 py-3 bg-blue-600 text-white rounded hover:bg-blue-700",
                                    "Submit Application"
                                }
                            }
                        }
                    },
                    Some(Err(e)) => rsx! {
                        div { class: "text-red-600", "Error loading form: {e}" }
                    },
                    None => rsx! {
                        div { class: "text-gray-600", "Loading form..." }
                    },
                }
            }
        }
    }
}
