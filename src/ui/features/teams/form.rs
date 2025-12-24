use dioxus::prelude::*;
use dioxus_forms::*;

use crate::ui::foundation::components::{Button, ButtonVariant, Input, InputHeight, InputVariant};

#[derive(Clone)]
pub struct TeamFormFields {
    pub name: FormField<String>,
    pub description: FormField<String>,
}

impl PartialEq for TeamFormFields {
    fn eq(&self, other: &Self) -> bool {
        // Compare by reference, each form instance is unique
        std::ptr::eq(self, other)
    }
}

impl TeamFormFields {
    pub fn new() -> Self {
        Self {
            name: use_form_field(String::new())
                .with_validator(validators::required("Team name is required")),
            description: use_form_field(String::new()),
        }
    }

    pub fn with_values(name: String, description: Option<String>) -> Self {
        Self {
            name: use_form_field(name)
                .with_validator(validators::required("Team name is required")),
            description: use_form_field(description.unwrap_or_default()),
        }
    }

    pub fn validate_all(&mut self) -> bool {
        self.name.validate()
    }
}

#[component]
pub fn TeamForm(
    fields: TeamFormFields,
    on_submit: EventHandler<FormEvent>,
    submit_label: String,
) -> Element {
    let (name_value, name_oninput, name_onblur) = use_field_bind(&fields.name);
    let (desc_value, desc_oninput, desc_onblur) = use_field_bind(&fields.description);

    // Create signals for the Input component
    let mut name_signal = use_signal(|| name_value.clone());
    let mut desc_signal = use_signal(|| desc_value.clone());

    // Sync signals with field values
    use_effect(move || {
        name_signal.set(name_value.clone());
    });
    use_effect(move || {
        desc_signal.set(desc_value.clone());
    });

    rsx! {
        form {
            onsubmit: move |evt| {
                evt.prevent_default();
                on_submit.call(evt);
            },
            div { class: "flex flex-col gap-5",
                div { class: "flex flex-col gap-2",
                    Input {
                        label: "Team Name".to_string(),
                        placeholder: Some("Enter team name".to_string()),
                        value: name_signal,
                        variant: InputVariant::Secondary,
                        name: Some("name".to_string()),
                        oninput: Some(name_oninput),
                        onblur: Some(name_onblur),
                    }
                    if fields.name.is_touched() {
                        if let Some(error) = fields.name.error.read().as_ref() {
                            span { class: "text-sm text-status-danger-foreground", "{error}" }
                        }
                    }
                }

                div { class: "flex flex-col gap-2",
                    Input {
                        label: "Description".to_string(),
                        placeholder: Some("Enter team description (optional)".to_string()),
                        value: desc_signal,
                        height: InputHeight::Tall,
                        variant: InputVariant::Secondary,
                        name: Some("description".to_string()),
                        oninput: Some(desc_oninput),
                        onblur: Some(desc_onblur),
                    }
                    if fields.description.is_touched() {
                        if let Some(error) = fields.description.error.read().as_ref() {
                            span { class: "text-sm text-status-danger-foreground", "{error}" }
                        }
                    }
                }
            }

            div { class: "mt-6",
                Button {
                    button_type: "submit".to_string(),
                    variant: ButtonVariant::Default,
                    "{submit_label}"
                }
            }
        }
    }
}
