use dioxus::prelude::*;
use dioxus_forms::*;
use dioxus_free_icons::{
    Icon,
    icons::ld_icons::{LdFileText, LdTrash2},
};

use crate::components::{Button, ButtonVariant, ButtonWithIcon, Input, InputHeight, InputVariant};

#[derive(Clone)]
pub struct HackathonFormFields {
    pub name: FormField<String>,
    pub description: FormField<String>,
    pub start_date: FormField<String>,
    pub end_date: FormField<String>,
}

impl PartialEq for HackathonFormFields {
    fn eq(&self, other: &Self) -> bool {
        // Compare by reference - each form instance is unique
        std::ptr::eq(self, other)
    }
}

impl HackathonFormFields {
    pub fn new() -> Self {
        Self {
            name: use_form_field(String::new())
                .with_validator(validators::required("Name is required")),
            description: use_form_field(String::new())
                .with_validator(validators::required("Description is required")),
            start_date: use_form_field(String::new())
                .with_validator(validators::required("Start date is required")),
            end_date: use_form_field(String::new())
                .with_validator(validators::required("End date is required")),
        }
    }

    pub fn validate_all(&mut self) -> bool {
        let name_valid = self.name.validate();
        let desc_valid = self.description.validate();
        let start_valid = self.start_date.validate();
        let end_valid = self.end_date.validate();
        name_valid && desc_valid && start_valid && end_valid
    }
}

#[component]
pub fn HackathonForm(
    fields: HackathonFormFields,
    banner_url: Signal<Option<String>>,
    banner_file: Signal<Option<(Vec<u8>, String)>>,
    on_submit: EventHandler<FormEvent>,
    submit_label: String,
) -> Element {
    let mut selected_file = use_signal(|| None::<String>);

    let on_remove_banner = move |_| {
        banner_url.set(None);
    };

    let (name_value, name_oninput, name_onblur) = use_field_bind(&fields.name);
    let (desc_value, desc_oninput, desc_onblur) = use_field_bind(&fields.description);
    let (start_value, start_oninput, start_onblur) = use_field_bind(&fields.start_date);
    let (end_value, end_oninput, end_onblur) = use_field_bind(&fields.end_date);

    // Create signals for the Input component
    let mut name_signal = use_signal(|| name_value.clone());
    let mut desc_signal = use_signal(|| desc_value.clone());
    let mut start_signal = use_signal(|| start_value.clone());
    let mut end_signal = use_signal(|| end_value.clone());

    // Sync signals with field values
    use_effect(move || {
        name_signal.set(name_value.clone());
    });
    use_effect(move || {
        desc_signal.set(desc_value.clone());
    });
    use_effect(move || {
        start_signal.set(start_value.clone());
    });
    use_effect(move || {
        end_signal.set(end_value.clone());
    });

    rsx! {
        form {
            enctype: "multipart/form-data",
            onsubmit: move |evt| {
                evt.prevent_default();
                on_submit.call(evt);
            },
            div { class: "flex flex-col gap-5",
                div { class: "flex flex-col gap-2",
                    Input {
                        label: "Hackathon Name".to_string(),
                        placeholder: Some("Enter hackathon name".to_string()),
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
                        placeholder: Some("Enter description".to_string()),
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

                div { class: "flex gap-4",
                    div { class: "flex flex-col gap-2 flex-1",
                        Input {
                            label: "Start Date & Time".to_string(),
                            placeholder: Some("Enter start date & time".to_string()),
                            value: start_signal,
                            variant: InputVariant::Secondary,
                            input_type: "datetime-local".to_string(),
                            name: Some("start_date".to_string()),
                            oninput: Some(start_oninput),
                            onblur: Some(start_onblur),
                        }
                        if fields.start_date.is_touched() {
                            if let Some(error) = fields.start_date.error.read().as_ref() {
                                span { class: "text-sm text-status-danger-foreground",
                                    "{error}"
                                }
                            }
                        }
                    }

                    div { class: "flex flex-col gap-2 flex-1",
                        Input {
                            label: "End Date & Time".to_string(),
                            placeholder: Some("Enter end date & time".to_string()),
                            value: end_signal,
                            variant: InputVariant::Secondary,
                            input_type: "datetime-local".to_string(),
                            name: Some("end_date".to_string()),
                            oninput: Some(end_oninput),
                            onblur: Some(end_onblur),
                        }
                        if fields.end_date.is_touched() {
                            if let Some(error) = fields.end_date.error.read().as_ref() {
                                span { class: "text-sm text-status-danger-foreground",
                                    "{error}"
                                }
                            }
                        }
                    }
                }

                // Banner upload
                div { class: "flex flex-col gap-2",
                    label { class: "text-base font-medium text-foreground-neutral-primary",
                        "Banner Image "
                        span { class: "text-sm font-normal text-foreground-neutral-secondary",
                            "(max 2MB)"
                        }
                    }

                    if let Some(url) = banner_url() {
                        div { class: "flex flex-col gap-2",
                            div { class: "relative w-40 aspect-1/2 rounded-lg overflow-hidden border border-border-neutral-primary",
                                img {
                                    src: url,
                                    class: "w-full h-full object-cover",
                                }
                                div { class: "absolute top-2 right-2",
                                    ButtonWithIcon {
                                        icon: LdTrash2,
                                        variant: ButtonVariant::Danger,
                                        onclick: on_remove_banner,
                                    }
                                }
                            }
                            // Allow changing banner
                            input {
                                r#type: "file",
                                name: "banner",
                                accept: "image/*",
                                id: "banner-upload",
                                class: "hidden",
                                onchange: move |evt| async move {
                                    let files = evt.files();
                                    if let Some(file) = files.first() {
                                        let file_name = file.name().to_string();
                                        selected_file.set(Some(file_name.clone()));
                                        let content_type = file_name
                                            .split('.')
                                            .last()
                                            .map(|ext| match ext {
                                                "jpg" | "jpeg" => "image/jpeg",
                                                "png" => "image/png",
                                                "webp" => "image/webp",
                                                "gif" => "image/gif",
                                                _ => "image/jpeg",
                                            })
                                            .unwrap_or("image/jpeg")
                                            .to_string();
                                        dioxus_logger::tracing::info!(
                                            "Reading file: {} with content_type: {}", file_name, content_type
                                        );
                                        match file.read_bytes().await {
                                            Ok(bytes) => {
                                                let vec = bytes.to_vec();
                                                dioxus_logger::tracing::info!(
                                                    "File read successfully: {} bytes", vec.len()
                                                );
                                                banner_file.set(Some((vec, content_type)));
                                            }
                                            Err(e) => {
                                                dioxus_logger::tracing::error!("Failed to read file: {:?}", e);
                                            }
                                        }
                                    }
                                },
                            }
                            label {
                                r#for: "banner-upload",
                                class: "flex items-center justify-center gap-2 h-10 px-4 bg-background-neutral-primary text-foreground-neutral-primary text-sm font-normal rounded-[0.625rem] cursor-pointer hover:opacity-90",
                                Icon { width: 18, height: 18, icon: LdFileText }
                                "Change banner"
                            }
                            if let Some(file) = selected_file() {
                                div { class: "text-sm text-foreground-neutral-secondary",
                                    "New file selected: {file}"
                                }
                            }
                        }
                    } else {
                        div { class: "flex flex-col gap-2",
                            input {
                                r#type: "file",
                                name: "banner",
                                accept: "image/*",
                                id: "banner-upload",
                                class: "hidden",
                                onchange: move |evt| async move {
                                    let files = evt.files();
                                    if let Some(file) = files.first() {
                                        let file_name = file.name().to_string();
                                        selected_file.set(Some(file_name.clone()));
                                        let content_type = file_name
                                            .split('.')
                                            .last()
                                            .map(|ext| match ext {
                                                "jpg" | "jpeg" => "image/jpeg",
                                                "png" => "image/png",
                                                "webp" => "image/webp",
                                                "gif" => "image/gif",
                                                _ => "image/jpeg",
                                            })
                                            .unwrap_or("image/jpeg")
                                            .to_string();
                                        dioxus_logger::tracing::info!(
                                            "Reading file: {} with content_type: {}", file_name, content_type
                                        );
                                        match file.read_bytes().await {
                                            Ok(bytes) => {
                                                let vec = bytes.to_vec();
                                                dioxus_logger::tracing::info!(
                                                    "File read successfully: {} bytes", vec.len()
                                                );
                                                banner_file.set(Some((vec, content_type)));
                                            }
                                            Err(e) => {
                                                dioxus_logger::tracing::error!("Failed to read file: {:?}", e);
                                            }
                                        }
                                    }
                                },
                            }
                            label {
                                r#for: "banner-upload",
                                class: "flex items-center justify-center gap-2 h-12 px-4 bg-background-neutral-primary text-foreground-neutral-primary text-sm font-normal rounded-[0.625rem] cursor-pointer hover:opacity-90",
                                Icon { width: 20, height: 20, icon: LdFileText }
                                "Choose file"
                            }
                            if let Some(file) = selected_file() {
                                div { class: "text-sm text-foreground-neutral-secondary",
                                    "Selected: {file}"
                                }
                            }
                        }
                    }
                }
            }

            div { class: "mt-12",
                Button {
                    button_type: "submit".to_string(),
                    variant: ButtonVariant::Default,
                    "{submit_label}"
                }
            }
        }
    }
}
