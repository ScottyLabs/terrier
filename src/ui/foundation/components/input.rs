use dioxus::prelude::*;

#[derive(Clone, Copy, PartialEq)]
pub enum InputHeight {
    Default,
    Tall,
}

#[derive(Clone, Copy, PartialEq)]
pub enum InputVariant {
    Primary,
    Secondary,
}

#[component]
pub fn NumberInput(
    label: String,
    value: Signal<i32>,
    #[props(default = InputVariant::Primary)] variant: InputVariant,
    name: Option<String>,
) -> Element {
    let bg_class = match variant {
        InputVariant::Primary => "bg-background-neutral-secondary",
        InputVariant::Secondary => "bg-background-neutral-primary",
    };

    let base_classes = format!(
        "px-4 h-12 {} text-foreground-neutral-primary text-sm font-normal placeholder:text-foreground-neutral-tertiary rounded-[0.625rem]",
        bg_class
    );

    rsx! {
        div { class: "flex flex-col gap-2",
            label { class: "text-base font-medium text-foreground-neutral-primary", "{label}" }
            input {
                class: "{base_classes}",
                r#type: "number",
                name,
                value: "{value}",
                oninput: move |evt| {
                    if let Ok(num) = evt.value().parse::<i32>() {
                        value.set(num);
                    }
                },
            }
        }
    }
}

#[component]
pub fn Input(
    label: String,
    placeholder: Option<String>,
    value: Signal<String>,
    #[props(default = InputHeight::Default)] height: InputHeight,
    #[props(default = InputVariant::Primary)] variant: InputVariant,
    #[props(default = "text".to_string())] input_type: String,
    name: Option<String>,
    id: Option<String>,
    #[props(default = false)] required: bool,
    #[props(default = None)] oninput: Option<EventHandler<FormEvent>>,
    #[props(default = None)] onblur: Option<EventHandler<FocusEvent>>,
    #[props(default = None)] help_text: Option<String>,
) -> Element {
    let (height_class, is_tall) = match height {
        InputHeight::Default => ("h-12", false),
        InputHeight::Tall => ("h-20", true),
    };

    let bg_class = match variant {
        InputVariant::Primary => "bg-background-neutral-secondary",
        InputVariant::Secondary => "bg-background-neutral-primary",
    };

    // Add extra padding for date inputs to accommodate calendar icon
    let base_classes = format!(
        "px-4 {} text-foreground-neutral-primary text-sm font-normal placeholder:text-foreground-neutral-tertiary rounded-[0.625rem]",
        bg_class
    );

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
            if let Some(help) = &help_text {
                p { class: "text-sm text-foreground-neutral-secondary", "{help}" }
            }
            if is_tall {
                textarea {
                    id: id.as_ref().cloned(),
                    class: "{height_class} {base_classes} pt-3 resize-none",
                    name: name.clone(),
                    placeholder: placeholder.unwrap_or("Enter".to_string()),
                    required,
                    value: "{value}",
                    oninput: move |evt| {
                        value.set(evt.value());
                        if let Some(handler) = &oninput {
                            handler.call(evt);
                        }
                    },
                    onblur: move |evt| {
                        if let Some(handler) = &onblur {
                            handler.call(evt);
                        }
                    },
                }
            } else {
                input {
                    id: id.as_ref().cloned(),
                    class: "{height_class} {base_classes}",
                    r#type: "{input_type}",
                    name,
                    placeholder: placeholder.unwrap_or("Enter".to_string()),
                    required,
                    value: "{value}",
                    oninput: move |evt| {
                        value.set(evt.value());
                        if let Some(handler) = &oninput {
                            handler.call(evt);
                        }
                    },
                    onblur: move |evt| {
                        if let Some(handler) = &onblur {
                            handler.call(evt);
                        }
                    },
                }
            }
        }
    }
}
