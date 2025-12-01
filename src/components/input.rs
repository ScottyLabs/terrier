use dioxus::prelude::*;

#[derive(Clone, Copy, PartialEq)]
pub enum InputHeight {
    Default,
    Tall,
}

#[derive(Clone, Copy, PartialEq)]
pub enum InputVariant {
    Default,
    Secondary,
}

#[component]
pub fn NumberInput(
    label: String,
    value: Signal<i32>,
    #[props(default = InputVariant::Default)] variant: InputVariant,
    name: Option<String>,
) -> Element {
    let bg_class = match variant {
        InputVariant::Default => "bg-background-brandNeutral-secondary",
        InputVariant::Secondary => "bg-background-neutral-primary",
    };

    let base_classes = format!(
        "px-4 h-12 {} text-foreground-brandNeutral-secondary text-sm font-normal placeholder:text-foreground-brandNeutral-secondary rounded-[0.625rem]",
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
    #[props(default = InputVariant::Default)] variant: InputVariant,
    name: Option<String>,
) -> Element {
    let (height_class, is_tall) = match height {
        InputHeight::Default => ("h-12", false),
        InputHeight::Tall => ("h-20", true),
    };

    let bg_class = match variant {
        InputVariant::Default => "bg-background-brandNeutral-secondary",
        InputVariant::Secondary => "bg-background-neutral-primary",
    };

    let base_classes = format!(
        "pl-4 {} text-foreground-brandNeutral-secondary text-sm font-normal placeholder:text-foreground-brandNeutral-secondary rounded-[0.625rem]",
        bg_class
    );

    rsx! {
        div { class: "flex flex-col gap-2",
            label { class: "text-base font-medium text-foreground-neutral-primary", "{label}" }
            if is_tall {
                textarea {
                    class: "{height_class} {base_classes} pt-3 resize-none",
                    name: name.clone(),
                    placeholder: placeholder.unwrap_or("Enter".to_string()),
                    value: "{value}",
                    oninput: move |evt| value.set(evt.value()),
                }
            } else {
                input {
                    class: "{height_class} {base_classes}",
                    r#type: "text",
                    name,
                    placeholder: placeholder.unwrap_or("Enter".to_string()),
                    value: "{value}",
                    oninput: move |evt| value.set(evt.value()),
                }
            }
        }
    }
}
