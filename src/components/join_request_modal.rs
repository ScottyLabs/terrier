use dioxus::prelude::*;

use super::{Button, ButtonVariant, Input, InputHeight, InputVariant, ModalBase};
use crate::hackathons::handlers::teams::{request_join_team, JoinTeamRequest};

#[component]
pub fn JoinRequestModal(
    on_close: EventHandler<()>,
    slug: String,
    team_id: i32,
    team_name: String,
) -> Element {
    let message = use_signal(|| String::new());
    let mut is_submitting = use_signal(|| false);
    let mut error_message = use_signal(|| None::<String>);

    let on_submit = move |evt: FormEvent| {
        evt.prevent_default();

        let slug = slug.clone();
        spawn(async move {
            is_submitting.set(true);
            error_message.set(None);

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
                    error_message.set(Some(e.to_string()));
                    is_submitting.set(false);
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
                if let Some(error) = error_message() {
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
                            variant: ButtonVariant::Secondary,
                            onclick: move |_| on_close.call(()),
                            "Cancel"
                        }
                        Button {
                            button_type: "submit".to_string(),
                            variant: ButtonVariant::Default,
                            disabled: is_submitting(),
                            if is_submitting() {
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
