use dioxus::prelude::*;

use crate::domain::teams::{UpdateTeamRequest, handlers::update_team};
use crate::ui::features::teams::form::{TeamForm, TeamFormFields};
use crate::ui::foundation::modals::ModalBase;
use crate::ui::foundation::hooks::use_async_action;

#[component]
pub fn EditTeamModal(
    on_close: EventHandler<()>,
    slug: String,
    team_name: String,
    team_description: Option<String>,
) -> Element {
    let form_fields = TeamFormFields::with_values(team_name, team_description);
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
                        "Edit Team"
                    }
                    p { class: "text-sm text-foreground-neutral-secondary mt-1",
                        "Update your team's name and description"
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
                        "Saving...".to_string()
                    } else {
                        "Save Changes".to_string()
                    },
                }
            }
        }
    }
}
