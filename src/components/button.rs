use dioxus::prelude::*;
use dioxus_free_icons::{Icon, IconShape};

#[derive(Clone, PartialEq, Default)]
pub enum ButtonVariant {
    #[default]
    Default,
    Secondary,
    Tertiary,
    Inverse,
    Danger,
    Success,
}

#[component]
pub fn Button(
    children: Element,
    #[props(default)] variant: ButtonVariant,
    #[props(default = "button".to_string())] button_type: String,
    onclick: Option<EventHandler<MouseEvent>>,
    #[props(default = false)] disabled: bool,
) -> Element {
    let bg_class = match variant {
        ButtonVariant::Default => "bg-foreground-neutral-primary",
        ButtonVariant::Secondary => "bg-background-neutral-primary",
        ButtonVariant::Tertiary => "bg-background-neutral-secondary-enabled",
        ButtonVariant::Inverse => "bg-foreground-neutral-primary",
        ButtonVariant::Danger => "bg-status-danger-foreground",
        ButtonVariant::Success => "bg-status-success-foreground",
    };

    let text_class = match variant {
        ButtonVariant::Secondary => "text-foreground-neutral-secondary",
        ButtonVariant::Tertiary => "text-foreground-neutral-primary",
        ButtonVariant::Inverse => "text-background-neutral-primary",
        _ => "text-white",
    };

    rsx! {
        button {
            r#type: "{button_type}",
            class: "px-5 py-3.5 {bg_class} {text_class} font-semibold text-sm leading-5 rounded-[100px] cursor-pointer",
            onclick: move |evt| {
                if let Some(handler) = onclick {
                    handler.call(evt);
                }
            },
            disabled: disabled,
            {children}
        }
    }
}

#[component]
pub fn ButtonWithIcon<I: IconShape + Clone + PartialEq + 'static>(
    children: Element,
    icon: I,
    #[props(default)] variant: ButtonVariant,
    #[props(default = "button".to_string())] button_type: String,
    onclick: Option<EventHandler<MouseEvent>>,
    #[props(default = false)] disabled: bool,
) -> Element {
    let bg_class = match variant {
        ButtonVariant::Default => "bg-foreground-neutral-primary",
        ButtonVariant::Secondary => "bg-background-neutral-primary",
        ButtonVariant::Tertiary => "bg-background-neutral-secondary-enabled",
        ButtonVariant::Inverse => "bg-foreground-neutral-primary",
        ButtonVariant::Danger => "bg-status-danger-foreground",
        ButtonVariant::Success => "bg-status-success-foreground",
    };

    let text_class = match variant {
        ButtonVariant::Secondary => "text-foreground-neutral-secondary",
        ButtonVariant::Tertiary => "text-foreground-neutral-primary",
        ButtonVariant::Inverse => "text-background-neutral-primary",
        _ => "text-white",
    };

    rsx! {
        button {
            r#type: "{button_type}",
            class: "px-5 py-3.5 {bg_class} {text_class} font-semibold text-sm leading-5 rounded-[100px] cursor-pointer flex gap-2 items-center",
            onclick: move |evt| {
                if let Some(handler) = onclick {
                    handler.call(evt);
                }
            },
            disabled: disabled,
            Icon {
                width: 20,
                height: 20,
                icon,
                class: match variant {
                    ButtonVariant::Secondary => "text-foreground-neutral-secondary",
                    _ => "text-white",
                },
            }
            {children}
        }
    }
}
