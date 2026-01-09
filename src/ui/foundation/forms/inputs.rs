use dioxus::prelude::*;

#[derive(Clone, PartialEq)]
pub struct FormSelectOption {
    pub label: String,
    pub value: String,
}

#[component]
pub fn Select(
    label: String,
    options: Vec<FormSelectOption>,
    value: Signal<String>,
    #[props(default = None)] placeholder: Option<String>,
    #[props(default = None)] name: Option<String>,
    #[props(default = None)] id: Option<String>,
    #[props(default = false)] required: bool,
    #[props(default = None)] onchange: Option<EventHandler<FormEvent>>,
) -> Element {
    let placeholder_text = placeholder.unwrap_or_else(|| "Select an option".to_string());

    rsx! {
        div { class: "flex flex-col gap-2",
            label {
                class: "text-base font-medium text-foreground-neutral-primary",
                r#for: id.clone().unwrap_or_default(),
                "{label}"
                if required {
                    span { class: "text-status-danger-foreground ml-1", "*" }
                }
            }
            select {
                id,
                name,
                class: "pl-4 pr-10 h-12 bg-background-neutral-secondary text-foreground-brandNeutral-secondary text-sm font-normal rounded-[0.625rem] appearance-none",
                required,
                value: "{value}",
                onchange: move |evt| {
                    value.set(evt.value());
                    if let Some(handler) = &onchange {
                        handler.call(evt);
                    }
                },
                option {
                    value: "",
                    disabled: required,
                    selected: value().is_empty(),
                    "{placeholder_text}"
                }
                for option in options {
                    option { value: "{option.value}", "{option.label}" }
                }
            }
        }
    }
}

#[component]
pub fn RadioGroup(
    label: String,
    options: Vec<FormSelectOption>,
    value: Signal<String>,
    #[props(default = None)] name: Option<String>,
    #[props(default = false)] required: bool,
    #[props(default = None)] onchange: Option<EventHandler<String>>,
) -> Element {
    let field_name = name.unwrap_or_else(|| label.to_lowercase().replace(" ", "_"));

    rsx! {
        div { class: "flex flex-col gap-2",
            label {
                class: "text-base font-medium text-foreground-neutral-primary mb-2",
                "{label}"
                if required {
                    span { class: "text-status-danger-foreground ml-1", "*" }
                }
            }
            div { class: "flex flex-col gap-2",
                for option in options {
                    label { class: "flex items-center gap-2 cursor-pointer",
                        input {
                            r#type: "radio",
                            name: "{field_name}",
                            value: "{option.value}",
                            required,
                            checked: value() == option.value,
                            onchange: move |_| {
                                let new_value = option.value.clone();
                                value.set(new_value.clone());
                                if let Some(handler) = &onchange {
                                    handler.call(new_value);
                                }
                            },
                        }
                        span { class: "text-sm text-foreground-neutral-secondary", "{option.label}" }
                    }
                }
            }
        }
    }
}

#[component]
pub fn Checkbox(
    label: String,
    value: Signal<String>,
    #[props(default = None)] name: Option<String>,
    #[props(default = None)] id: Option<String>,
    #[props(default = false)] required: bool,
    #[props(default = None)] onchange: Option<EventHandler<String>>,
) -> Element {
    rsx! {
        div { class: "flex flex-col gap-2",
            label { class: "flex items-center gap-2 cursor-pointer",
                input {
                    id,
                    name,
                    r#type: "checkbox",
                    required,
                    checked: value() == "true",
                    onchange: move |evt| {
                        let new_value = if evt.checked() { "true".to_string() } else { "false".to_string() };
                        value.set(new_value.clone());
                        if let Some(handler) = &onchange {
                            handler.call(new_value);
                        }
                    },
                }
                span {
                    class: "text-base font-medium text-foreground-neutral-primary",
                    "{label}"
                    if required {
                        span { class: "text-status-danger-foreground ml-1", "*" }
                    }
                }
            }
        }
    }
}

#[component]
pub fn CheckboxGroup(
    label: String,
    options: Vec<FormSelectOption>,
    value: Signal<String>,
    #[props(default = None)] name: Option<String>,
    #[props(default = false)] required: bool,
    #[props(default = None)] onchange: Option<EventHandler<String>>,
) -> Element {
    let field_name = name.unwrap_or_else(|| label.to_lowercase().replace(" ", "_"));

    rsx! {
        div { class: "flex flex-col gap-2",
            label {
                class: "text-base font-medium text-foreground-neutral-primary mb-2",
                "{label}"
                if required {
                    span { class: "text-status-danger-foreground ml-1", "*" }
                }
            }
            div { class: "flex flex-col gap-2",
                for option in options {
                    {
                        let option_value = option.value.clone();
                        let field_name = field_name.clone();
                        rsx! {
                            label { class: "flex items-center gap-2 cursor-pointer",
                                input {
                                    r#type: "checkbox",
                                    name: "{field_name}[]",
                                    value: "{option_value}",
                                    checked: {
                                        let current = value();
                                        if current.is_empty() {
                                            false
                                        } else {
                                            current.split(',').any(|v| v == option_value.as_str())
                                        }
                                    },
                                    onchange: move |evt| {
                                        let current = value();
                                        let values: Vec<&str> = if current.is_empty() {
                                            vec![]
                                        } else {
                                            current.split(',').collect()
                                        };
                                        let new_value = if evt.checked() {
                                            let mut new_values = values.clone();
                                            if !new_values.contains(&option_value.as_str()) {
                                                new_values.push(&option_value);
                                            }
                                            new_values.join(",")
                                        } else {
                                            let new_values: Vec<&str> = values
                                                .into_iter()
                                                .filter(|v| *v != option_value.as_str())
                                                .collect();
                                            new_values.join(",")
                                        };
                                        value.set(new_value.clone());
                                        if let Some(handler) = &onchange {
                                            handler.call(new_value);
                                        }
                                    },
                                }
                                span { class: "text-sm text-foreground-brandNeutral-secondary", "{option.label}" }
                            }
                        }
                    }
                }
            }
        }
    }
}
