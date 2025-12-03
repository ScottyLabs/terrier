use dioxus::prelude::*;
use dioxus_free_icons::{Icon, icons::ld_icons::LdX};

/// Base modal component with backdrop, close button, and consistent styling
#[component]
pub fn ModalBase(
    children: Element,
    on_close: EventHandler<()>,
    #[props(default = "600px".to_string())] width: String,
    #[props(default = "80vh".to_string())] max_height: String,
) -> Element {
    rsx! {
        // Backdrop
        div {
            class: "fixed inset-0 flex items-center justify-center z-50",
            style: "background-color: rgba(0, 0, 0, 0.5);",
            onclick: move |_| on_close.call(()),

            // Modal container
            div {
                class: "bg-background-brandNeutral-secondary rounded-[20px] shadow-lg max-h-[{max_height}] overflow-y-auto relative",
                style: "width: {width};",
                onclick: move |e| e.stop_propagation(),

                // Close button (absolutely positioned)
                button {
                    class: "absolute top-7 right-7 text-foreground-neutral-primary hover:text-foreground-neutral-tertiary transition-colors z-10",
                    onclick: move |_| on_close.call(()),
                    Icon {
                        width: 24,
                        height: 24,
                        icon: LdX,
                    }
                }

                // Content
                {children}
            }
        }
    }
}
