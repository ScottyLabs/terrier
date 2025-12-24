use dioxus::prelude::*;

use crate::domain::teams::{CreateTeamRequest, handlers::create_team};
use crate::ui::features::teams::form::{TeamForm, TeamFormFields};
use crate::ui::foundation::modals::ModalBase;
use crate::ui::foundation::hooks::use_async_action;

#[component]
pub fn CreateTeamModal(on_close: EventHandler<()>, slug: String) -> Element {
    let form_fields = TeamFormFields::new();
    let action = use_async_action();

    let on_submit = move |evt: FormEvent| {
        evt.prevent_default();

        let name = form_fields.name.value.read().clone();
        let description = form_fields.description.value.read().clone();
        let mut action = action.clone();

        spawn({
            let slug = slug.clone();
            async move {
                action.set_loading(true);
                action.clear_error();

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
                        action.set_error(Some(e.to_string()));
                        action.set_loading(false);
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
                if let Some(error) = action.error() {
                    div { class: "mb-4 p-3 bg-status-danger-background text-status-danger-foreground rounded-lg text-sm",
                        "{error}"
                    }
                }

                // Form
                TeamForm {
                    fields: form_fields,
                    on_submit,
                    submit_label: if action.is_loading() {
                        "Creating...".to_string()
                    } else {
                        "Create Team".to_string()
                    },
                }
            }
        }
    }
}
