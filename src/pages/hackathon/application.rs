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

    let mut submit_application = use_action(backend::submit_hackathon_application);

    rsx! {
        div { class: "space-y-8",
            h1 { class: "text-2xl font-bold", "Hackathon Application" }
            p { class: "text-sm text-gray-600", "Hackathon ID: {id}" }
            {
                match &*form_res.read_unchecked() {
                    Some(Ok(json)) => {
                        rsx! {
                            form {
                                onsubmit: move |ev: FormEvent| {
                                    ev.prevent_default();
                                    let data = values.read().clone();
                                    let slug = id.clone();
                                    tracing::info!("Form submitted: {:?}", data);

                                    spawn(async move {
                                        submit_application.call(slug, data).await;
                                    });
                                },
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
                                                                                class: "w-full border rounded px-3 py-2",
                                                                                value: "{values.read().get(&field_id).cloned().unwrap_or_default()}",
                                                                                oninput: move |evt| {
                                                                                    values.write().insert(field_id_clone.clone(), evt.value());
                                                                                }
                                                                            }
                                                                            if let Some(desc) = &description {
                                                                                p { class: "text-xs text-gray-500 mt-1", "{desc}" }
                                                                            }
                                                                        }
                                                                    }
                                                                },
                                                                "long-response" => {
                                                                    let field_id_clone = field_id.clone();
                                                                    rsx! {
                                                                        div { key: "{field_id}", class: "mb-4",
                                                                            label { r#for: "{field_id}", class: "block mb-1", "{question}" }
                                                                            textarea {
                                                                                id: "{field_id}",
                                                                                name: "{field_id}",
                                                                                class: "w-full border rounded px-3 py-2",
                                                                                rows: "4",
                                                                                value: "{values.read().get(&field_id).cloned().unwrap_or_default()}",
                                                                                oninput: move |evt| {
                                                                                    values.write().insert(field_id_clone.clone(), evt.value());
                                                                                }
                                                                            }
                                                                            if let Some(desc) = &description {
                                                                                p { class: "text-xs text-gray-500 mt-1", "{desc}" }
                                                                            }
                                                                        }
                                                                    }
                                                                },
                                                                "dropdown" => {
                                                                    let field_id_clone = field_id.clone();
                                                                    rsx! {
                                                                        div { key: "{field_id}", class: "mb-4",
                                                                            label { r#for: "{field_id}", class: "block mb-1", "{question}" }
                                                                            select {
                                                                                id: "{field_id}",
                                                                                name: "{field_id}",
                                                                                class: "w-full border rounded px-3 py-2",
                                                                                value: "{values.read().get(&field_id).cloned().unwrap_or_default()}",
                                                                                oninput: move |evt| {
                                                                                    values.write().insert(field_id_clone.clone(), evt.value());
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
                                                                        }
                                                                    }
                                                                },
                                                                "multi-checkbox" => {
                                                                    rsx! {
                                                                        fieldset { key: "{field_id}", class: "mb-4",
                                                                            legend { class: "block mb-2 font-medium", "{question}" }
                                                                            for opt in options.unwrap_or_default().iter() {
                                                                                {
                                                                                    opt.as_str().map(|text| {
                                                                                        let opt_id = format!("{}__{}", field_id, text);
                                                                                        let opt_id_clone = opt_id.clone();
                                                                                        let text = text.to_string();
                                                                                        let text_clone = text.clone();
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
                                                                        }
                                                                    }
                                                                },
                                                                "checkbox" => {
                                                                    let field_id_clone = field_id.clone();
                                                                    rsx! {
                                                                        div { key: "{field_id}", class: "mb-4 flex items-center",
                                                                            input {
                                                                                r#type: "checkbox",
                                                                                id: "{field_id}",
                                                                                class: "mr-2",
                                                                                checked: values.read().get(&field_id).map(|v| v == "true").unwrap_or(false),
                                                                                oninput: move |_| {
                                                                                    let mut w = values.write();
                                                                                    let cur = w.get(&field_id_clone).map(|v| v == "true").unwrap_or(false);
                                                                                    w.insert(field_id_clone.clone(), (!cur).to_string());
                                                                                }
                                                                            }
                                                                            label { r#for: "{field_id}", "{question}" }
                                                                            if let Some(desc) = &description {
                                                                                p { class: "text-xs text-gray-500 ml-2", "{desc}" }
                                                                            }
                                                                        }
                                                                    }
                                                                },
                                                                "signature" => {
                                                                    let field_id_clone = field_id.clone();
                                                                    rsx! {
                                                                        div { key: "{field_id}", class: "mb-4",
                                                                            label { r#for: "{field_id}", class: "block mb-1", "{question}" }
                                                                            input {
                                                                                r#type: "text",
                                                                                placeholder: "Type full name as signature",
                                                                                id: "{field_id}",
                                                                                class: "w-full border rounded px-3 py-2",
                                                                                value: "{values.read().get(&field_id).cloned().unwrap_or_default()}",
                                                                                oninput: move |evt| {
                                                                                    values.write().insert(field_id_clone.clone(), evt.value());
                                                                                }
                                                                            }
                                                                            if let Some(desc) = &description {
                                                                                p { class: "text-xs text-gray-500 mt-1", "{desc}" }
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
