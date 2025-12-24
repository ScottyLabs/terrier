use dioxus::prelude::*;

use crate::domain::teams::{JoinTeamRequest, handlers::request_join_team};
use crate::ui::foundation::components::{Button, ButtonVariant, Input, InputHeight, InputVariant};
use crate::ui::foundation::modals::ModalBase;
use crate::ui::foundation::hooks::use_async_action;

#[component]
pub fn JoinRequestModal(
    on_close: EventHandler<()>,
    slug: String,
    team_id: i32,
    team_name: String,
) -> Element {
    let message = use_signal(|| String::new());
    let action = use_async_action();

    let on_submit = move |evt: FormEvent| {
        evt.prevent_default();

        let slug = slug.clone();
        let mut action = action.clone();
        spawn(async move {
            action.set_loading(true);
            action.clear_error();

            let req = JoinTeamRequest {
                team_id,
                message: if message().is_empty() {
                    None
                } else {
                    Some(message())
                },
            };

            match request_join_team(slug, req).await {
                Ok(_) => {
                    on_close.call(());
                }
                Err(e) => {
                    action.set_error(Some(e.to_string()));
                    action.set_loading(false);
                }
            }
        });
    };

    rsx! {
        ModalBase { on_close, width: "500px", max_height: "80vh",

            div { class: "p-7",
                // Header
                div { class: "mb-6",
                    h2 { class: "text-2xl font-semibold leading-8 text-foreground-neutral-primary",
                        "Request to Join"
                    }
                    p { class: "text-sm text-foreground-neutral-secondary mt-1",
                        "Send a request to join \"{team_name}\""
                    }
                }

                // Error message
                if let Some(error) = action.error() {
                    div { class: "mb-4 p-3 bg-status-danger-background text-status-danger-foreground rounded-lg text-sm",
                        "{error}"
                    }
                }

                // Form
                form { onsubmit: on_submit,
                    div { class: "flex flex-col gap-5 mb-6",
                        div { class: "flex flex-col gap-2",
                            Input {
                                label: "Message (Optional)".to_string(),
                                placeholder: Some("Tell them why you'd like to join...".to_string()),
                                value: message,
                                height: InputHeight::Tall,
                                variant: InputVariant::Secondary,
                                name: Some("message".to_string()),
                            }
                        }
                    }

                    div { class: "flex items-center justify-end gap-3",
                        Button {
                            button_type: "button".to_string(),
                            variant: ButtonVariant::Secondary,
                            onclick: move |_| on_close.call(()),
                            "Cancel"
                        }
                        Button {
                            button_type: "submit".to_string(),
                            variant: ButtonVariant::Default,
                            disabled: action.is_loading(),
                            if action.is_loading() {
                                "Sending..."
                            } else {
                                "Send Request"
                            }
                        }
                    }
                }
            }
        }
    }
}
