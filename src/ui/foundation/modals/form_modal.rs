use crate::ui::foundation::components::{Button, ButtonVariant};
use crate::ui::foundation::modals::base::ModalBase;
use dioxus::prelude::*;

/// Generic form modal wrapper with standard layout and submit/cancel buttons
#[component]
pub fn FormModal(
    title: String,
    description: Option<String>,
    on_submit: EventHandler<FormEvent>,
    on_close: EventHandler<()>,
    children: Element,
    #[props(default = "Submit".to_string())] submit_text: String,
    #[props(default = false)] is_loading: bool,
    #[props(default = None)] error: Option<String>,
) -> Element {
    rsx! {
        ModalBase {
            on_close: move |_| on_close.call(()),

            div {
                class: "p-8",

                // Header
                div {
                    class: "mb-6",

                    h2 {
                        class: "text-xl font-semibold text-foreground-neutral-primary mb-2",
                        "{title}"
                    }

                    if let Some(desc) = description {
                        p {
                            class: "text-foreground-neutral-secondary text-sm",
                            "{desc}"
                        }
                    }
                }

                // Form
                form {
                    onsubmit: move |evt| {
                        evt.prevent_default();
                        on_submit.call(evt);
                    },

                    // Form content
                    {children}

                    // Error display
                    if let Some(err) = error {
                        div {
                            class: "mt-4 p-3 bg-status-danger-background border border-status-danger-foreground rounded-lg",
                            p {
                                class: "text-status-danger-foreground text-sm",
                                "{err}"
                            }
                        }
                    }

                    // Actions
                    div {
                        class: "flex gap-3 justify-end mt-6",

                        Button {
                            variant: ButtonVariant::Secondary,
                            button_type: "button",
                            onclick: move |_| on_close.call(()),
                            disabled: is_loading,
                            "Cancel"
                        }

                        Button {
                            variant: ButtonVariant::Default,
                            button_type: "submit",
                            disabled: is_loading,
                            "{submit_text}"
                        }
                    }
                }
            }
        }
    }
}
