use dioxus::prelude::*;
use dioxus_free_icons::{Icon, IconShape};

#[derive(Clone, PartialEq, Default)]
pub enum ButtonVariant {
    #[default]
    Default,
    Primary,
    Secondary,
    Tertiary,
    Outline,
    Danger,
    Success,
}

#[derive(Clone, PartialEq, Default)]
pub enum ButtonSize {
    #[default]
    Normal,
    Compact,
}

#[component]
pub fn Button(
    children: Element,
    #[props(default)] variant: ButtonVariant,
    #[props(default)] size: ButtonSize,
    #[props(default = "button".to_string())] button_type: String,
    onclick: Option<EventHandler<MouseEvent>>,
    #[props(default = false)] disabled: bool,
    #[props(default = "".to_string())] class: String,
) -> Element {
    let bg_class = match variant {
        ButtonVariant::Default => "bg-foreground-neutral-primary",
        ButtonVariant::Primary => "bg-background-brandNeutral-secondary",
        ButtonVariant::Secondary => "bg-background-neutral-primary",
        ButtonVariant::Tertiary => "bg-background-neutral-secondary-enabled",
        ButtonVariant::Outline => "bg-transparent border border-stroke-neutral-1",
        ButtonVariant::Danger => "bg-status-danger-foreground",
        ButtonVariant::Success => "bg-status-success-foreground",
    };

    let text_class = match variant {
        ButtonVariant::Primary => "text-foreground-brandNeutral-primary",
        ButtonVariant::Secondary => "text-foreground-neutral-secondary",
        ButtonVariant::Tertiary => "text-foreground-neutral-primary",
        ButtonVariant::Outline => "text-foreground-neutral-primary",
        _ => "text-white",
    };

    let size_class = match size {
        ButtonSize::Compact => "px-4 py-[9px] text-sm leading-5 font-semibold",
        ButtonSize::Normal => "px-5 py-3.5 text-sm leading-5 font-semibold",
    };

    rsx! {
        button {
            r#type: "{button_type}",
            class: "{size_class} {bg_class} {text_class} {class} rounded-[100px] cursor-pointer",
            onclick: move |evt| {
                if let Some(handler) = onclick {
                    handler.call(evt);
                }
            },
            disabled,
            {children}
        }
    }
}

#[component]
pub fn ButtonWithIcon<I: IconShape + Clone + PartialEq + 'static>(
    children: Element,
    icon: I,
    #[props(default)] variant: ButtonVariant,
    #[props(default)] size: ButtonSize,
    #[props(default = "button".to_string())] button_type: String,
    onclick: Option<EventHandler<MouseEvent>>,
    #[props(default = false)] disabled: bool,
    #[props(default = "".to_string())] class: String,
) -> Element {
    let bg_class = match variant {
        ButtonVariant::Default => "bg-foreground-neutral-primary",
        ButtonVariant::Primary => "bg-background-brandNeutral-secondary",
        ButtonVariant::Secondary => "bg-background-neutral-primary",
        ButtonVariant::Tertiary => "bg-background-neutral-secondary-enabled",
        ButtonVariant::Outline => "bg-transparent border border-stroke-neutral-1",
        ButtonVariant::Danger => "bg-status-danger-foreground",
        ButtonVariant::Success => "bg-status-success-foreground",
    };

    let text_class = match variant {
        ButtonVariant::Primary => "text-foreground-brandNeutral-primary",
        ButtonVariant::Secondary => "text-foreground-neutral-secondary",
        ButtonVariant::Tertiary => "text-foreground-neutral-primary",
        ButtonVariant::Outline => "text-foreground-neutral-primary",
        _ => "text-white",
    };

    let size_class = match size {
        ButtonSize::Compact => "px-4 py-[9px] text-sm leading-5 font-semibold gap-1.5",
        ButtonSize::Normal => "px-5 py-3.5 text-sm leading-5 font-semibold gap-2",
    };

    rsx! {
        button {
            r#type: "{button_type}",
            class: "{size_class} {bg_class} {text_class} {class} rounded-[100px] cursor-pointer flex items-center",
            onclick: move |evt| {
                if let Some(handler) = onclick {
                    handler.call(evt);
                }
            },
            disabled,
            Icon {
                width: match size {
                    ButtonSize::Compact => 16,
                    ButtonSize::Normal => 20,
                },
                height: match size {
                    ButtonSize::Compact => 16,
                    ButtonSize::Normal => 20,
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
