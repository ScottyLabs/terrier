use dioxus::prelude::*;

use super::ModalBase;
use crate::forms::{TeamForm, TeamFormFields};
use crate::hackathons::handlers::teams::{CreateTeamRequest, create_team};

#[component]
pub fn CreateTeamModal(on_close: EventHandler<()>, slug: String) -> Element {
    let form_fields = TeamFormFields::new();
    let mut is_submitting = use_signal(|| false);
    let mut error_message = use_signal(|| None::<String>);

    let on_submit = move |evt: FormEvent| {
        evt.prevent_default();

        let name = form_fields.name.value.read().clone();
        let description = form_fields.description.value.read().clone();

        spawn({
            let slug = slug.clone();
            async move {
                is_submitting.set(true);
                error_message.set(None);

                let req = CreateTeamRequest {
                    name,
                    description: if description.is_empty() {
                        None
                    } else {
                        Some(description)
                    },
                };

                match create_team(slug, req).await {
                    Ok(_) => {
                        on_close.call(());
                    }
                    Err(e) => {
                        error_message.set(Some(e.to_string()));
                        is_submitting.set(false);
                    }
                }
            }
        });
    };

    rsx! {
        ModalBase {
            on_close,
            width: "600px",
            max_height: "80vh",

            div { class: "p-7",
                // Header
                div { class: "mb-8",
                    h2 { class: "text-2xl font-semibold leading-8 text-foreground-neutral-primary",
                        "Create New Team"
                    }
                    p { class: "text-sm text-foreground-neutral-secondary mt-1",
                        "Start a team and invite members to join you"
                    }
                }

                // Error message
                if let Some(error) = error_message() {
                    div { class: "mb-4 p-3 bg-status-danger-background text-status-danger-foreground rounded-lg text-sm",
                        "{error}"
                    }
                }

                // Form
                TeamForm {
                    fields: form_fields,
                    on_submit,
                    submit_label: if is_submitting() {
                        "Creating...".to_string()
                    } else {
                        "Create Team".to_string()
                    },
                }
            }
        }
    }
}
