use dioxus::prelude::*;
use dioxus_free_icons::{Icon, IconShape};

#[derive(Clone, PartialEq, Default)]
pub enum ButtonVariant {
    #[default]
    Default,
    Secondary,
    Tertiary,
    Outline,
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
        ButtonVariant::Outline => "bg-transparent border border-stroke-neutral-1",
        ButtonVariant::Inverse => "bg-foreground-neutral-primary",
        ButtonVariant::Danger => "bg-status-danger-foreground",
        ButtonVariant::Success => "bg-status-success-foreground",
    };

    let text_class = match variant {
        ButtonVariant::Secondary => "text-foreground-neutral-secondary",
        ButtonVariant::Tertiary => "text-foreground-neutral-primary",
        ButtonVariant::Outline => "text-foreground-neutral-primary",
        ButtonVariant::Inverse => "text-background-neutral-primary",
        _ => "text-white",
    };

    let size_class = match variant {
        ButtonVariant::Outline => "px-4 py-2 text-xs leading-4 font-medium",
        _ => "px-5 py-3.5 text-sm leading-5 font-semibold",
    };

    rsx! {
        button {
            r#type: "{button_type}",
            class: "{size_class} {bg_class} {text_class} rounded-[100px] cursor-pointer",
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
        ButtonVariant::Outline => "bg-transparent border border-stroke-neutral-1",
        ButtonVariant::Inverse => "bg-foreground-neutral-primary",
        ButtonVariant::Danger => "bg-status-danger-foreground",
        ButtonVariant::Success => "bg-status-success-foreground",
    };

    let text_class = match variant {
        ButtonVariant::Secondary => "text-foreground-neutral-secondary",
        ButtonVariant::Tertiary => "text-foreground-neutral-primary",
        ButtonVariant::Outline => "text-foreground-neutral-primary",
        ButtonVariant::Inverse => "text-background-neutral-primary",
        _ => "text-white",
    };

    let size_class = match variant {
        ButtonVariant::Outline => "px-4 py-2 text-xs leading-4 font-medium gap-1.5",
        _ => "px-5 py-3.5 text-sm leading-5 font-semibold gap-2",
    };

    rsx! {
        button {
            r#type: "{button_type}",
            class: "{size_class} {bg_class} {text_class} rounded-[100px] cursor-pointer flex items-center",
            onclick: move |evt| {
                if let Some(handler) = onclick {
                    handler.call(evt);
                }
            },
            disabled: disabled,
            Icon {
                width: match variant {
                    ButtonVariant::Outline => 16,
                    _ => 20,
                },
                height: match variant {
                    ButtonVariant::Outline => 16,
                    _ => 20,
                },
                icon,
                class: match variant {
                    ButtonVariant::Secondary => "text-foreground-neutral-secondary",
                    ButtonVariant::Outline => "text-foreground-neutral-primary",
                    _ => "text-white",
                },
            }
            {children}
        }
    }
}
