use dioxus::prelude::*;

use super::ModalBase;
use crate::forms::{TeamForm, TeamFormFields};
use crate::hackathons::handlers::teams::{UpdateTeamRequest, update_team};

#[component]
pub fn EditTeamModal(
    on_close: EventHandler<()>,
    slug: String,
    team_name: String,
    team_description: Option<String>,
) -> Element {
    let form_fields = TeamFormFields::with_values(team_name, team_description);
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

                let req = UpdateTeamRequest {
                    name,
                    description: if description.is_empty() {
                        None
                    } else {
                        Some(description)
                    },
                };

                match update_team(slug, req).await {
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
                        "Edit Team"
                    }
                    p { class: "text-sm text-foreground-neutral-secondary mt-1",
                        "Update your team's name and description"
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
                        "Saving...".to_string()
                    } else {
                        "Save Changes".to_string()
                    },
                }
            }
        }
    }
}
