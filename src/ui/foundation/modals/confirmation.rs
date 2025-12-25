use crate::ui::foundation::components::{Button, ButtonVariant};
use crate::ui::foundation::modals::base::ModalBase;
use dioxus::prelude::*;

/// Generic confirmation modal for destructive or important actions
#[component]
pub fn ConfirmationModal(
    title: String,
    message: String,
    confirm_text: String,
    #[props(default = ButtonVariant::Danger)] confirm_variant: ButtonVariant,
    on_confirm: EventHandler<()>,
    on_cancel: EventHandler<()>,
    #[props(default = false)] is_loading: bool,
) -> Element {
    rsx! {
        ModalBase {
            on_close: move |_| on_cancel.call(()),
            width: "500px",

            div {
                class: "p-8",

                // Title
                h2 {
                    class: "text-xl font-semibold text-foreground-neutral-primary mb-4",
                    "{title}"
                }

                // Message
                p {
                    class: "text-foreground-neutral-secondary mb-6",
                    "{message}"
                }

                // Actions
                div {
                    class: "flex gap-3 justify-end",

                    Button {
                        variant: ButtonVariant::Secondary,
                        onclick: move |_| on_cancel.call(()),
                        disabled: is_loading,
                        "Cancel"
                    }

                    Button {
                        variant: confirm_variant,
                        onclick: move |_| on_confirm.call(()),
                        disabled: is_loading,
                        "{confirm_text}"
                    }
                }
            }
        }
    }
}
