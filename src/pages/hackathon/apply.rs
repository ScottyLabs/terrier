use crate::{
    auth::{APPLY_ROLES, hooks::use_require_access_or_redirect},
    components::Button,
    hackathons::HackathonInfo,
    schemas::FormSchema,
};
use dioxus::prelude::*;

#[component]
pub fn HackathonApply(slug: String) -> Element {
    if let Some(no_access) = use_require_access_or_redirect(APPLY_ROLES) {
        return no_access;
    }

    let hackathon = use_context::<Signal<HackathonInfo>>();

    // Parse form config from hackathon
    let form_schema = use_memo(move || {
        hackathon
            .read()
            .form_config
            .as_ref()
            .and_then(|config| serde_json::from_value::<FormSchema>(config.clone()).ok())
    });

    rsx! {
        div { class: "p-7",
            div { class: "max-w-3xl mx-auto mt-8",
                if !hackathon.read().is_active {
                    div { class: "text-center py-12",
                        h2 { class: "text-2xl font-semibold text-foreground-neutral-primary mb-4",
                            "Registration Closed"
                        }
                        p { class: "text-foreground-neutral-secondary",
                            "Applications are not currently being accepted for this hackathon."
                        }
                    }
                } else if let Some(schema) = form_schema() {
                    ApplicationForm { schema }
                } else {
                    div { class: "text-center py-12",
                        h2 { class: "text-2xl font-semibold text-foreground-neutral-primary mb-4",
                            "Applications not open."
                        }
                        p { class: "text-foreground-neutral-secondary",
                            "The application form for this hackathon has not been configured yet."
                        }
                    }
                }
            }
        }
    }
}

#[component]
fn ApplicationForm(schema: FormSchema) -> Element {
    let hackathon = use_context::<Signal<HackathonInfo>>();
    let nav = navigator();

    // Store all form values in a shared context
    let form_values = use_signal(|| std::collections::HashMap::<String, String>::new());
    let mut is_submitting = use_signal(|| false);
    let mut error_message = use_signal(|| None::<String>);

    let handle_submit = move |_| {
        spawn(async move {
            is_submitting.set(true);
            error_message.set(None);

            let slug = hackathon.read().slug.clone();
            let form_data = serde_json::to_value(form_values()).unwrap_or_default();

            match crate::hackathons::handlers::applications::submit_application(slug, form_data)
                .await
            {
                Ok(_) => {
                    // Redirect to dashboard or show success
                    nav.push(format!("/h/{}", hackathon.read().slug));
                }
                Err(e) => {
                    error_message.set(Some(format!("Failed to submit application: {}", e)));
                    is_submitting.set(false);
                }
            }
        });
    };

    rsx! {
        div { class: "bg-background-neutral-primary rounded-lg p-8",
            h1 { class: "text-3xl font-bold text-foreground-neutral-primary mb-2",
                "{schema.title}"
            }
            if let Some(description) = &schema.description {
                p { class: "text-foreground-neutral-secondary mb-8", "{description}" }
            }

            if let Some(error) = error_message() {
                div { class: "mb-4 p-4 bg-status-danger-background text-status-danger-foreground rounded-lg",
                    "{error}"
                }
            }

            form { class: "flex flex-col gap-6", onsubmit: handle_submit,

                for field in schema.fields.iter() {
                    FormFieldRenderer {
                        field: field.clone(),
                        form_values,
                        hackathon_slug: hackathon.read().slug.clone(),
                    }
                }

                div { class: "mt-8",
                    Button {
                        button_type: "submit".to_string(),
                        disabled: is_submitting(),
                        if is_submitting() {
                            "Submitting..."
                        } else {
                            "Submit Application"
                        }
                    }
                }
            }
        }
    }
}

#[component]
fn FormFieldRenderer(
    field: crate::schemas::FormField,
    form_values: Signal<std::collections::HashMap<String, String>>,
    hackathon_slug: String,
) -> Element {
    use crate::schemas::FieldType;

    // Clone field data before hooks
    let field_id = field.id.clone();
    let field_name = field.name.clone();
    let field_label = field.label.clone();
    let field_type = field.field_type.clone();
    let field_required = field.required;
    let field_placeholder = field.placeholder.clone();
    let field_options = field.options.clone();
    let field_help_text = field.help_text.clone();
    let field_default = field.default_value.clone().unwrap_or_default();
    let field_conditional = field.conditional.clone();

    let mut value = use_signal(|| field_default);

    // Update form_values whenever this field changes
    {
        let field_name = field_name.clone();
        use_effect(move || {
            let current_value = value();
            if !current_value.is_empty() || form_values.read().contains_key(&field_name) {
                form_values
                    .write()
                    .insert(field_name.clone(), current_value);
            }
        });
    }

    // Check if field should be shown based on conditional
    let should_show = use_memo(move || {
        if let Some(condition) = &field_conditional {
            let values = form_values.read();
            if let Some(parent_value) = values.get(&condition.field) {
                // For single-select, check if value matches
                if condition.value.contains(parent_value) {
                    return true;
                }

                // For multi-select (comma-separated), check if any value is present
                let parent_values: Vec<&str> = parent_value.split(',').collect();
                for cv in &condition.value {
                    if parent_values.contains(&cv.as_str()) {
                        return true;
                    }
                }
                return false;
            }
            return false;
        }
        true // No condition means always show
    });

    if !should_show() {
        return rsx! {
            div { style: "display: none;" }
        };
    }

    rsx! {
        div { class: "flex flex-col gap-2",
            label {
                class: "text-base font-medium text-foreground-neutral-primary",
                r#for: "{field_id}",
                "{field_label}"
                if field_required {
                    span { class: "text-status-danger-foreground ml-1", "*" }
                }
            }

            match field_type {
                FieldType::Text | FieldType::Email | FieldType::Tel | FieldType::Url => {
                    let field_id = field_id.to_string();
                    let field_name = field_name.to_string();
                    let field_type = field_type.clone();
                    let placeholder = field_placeholder.as_deref().unwrap_or("");
                    rsx! {
                        input {
                            id: "{field_id}",
                            name: "{field_name}",
                            r#type: match field_type {
                                FieldType::Email => "email",
                                FieldType::Tel => "tel",
                                FieldType::Url => "url",
                                _ => "text",
                            },
                            class: "px-4 h-12 bg-background-neutral-primary text-foreground-brandNeutral-secondary text-sm font-normal placeholder:text-foreground-brandNeutral-secondary rounded-[0.625rem] border border-border-neutral-primary",
                            placeholder: "{placeholder}",
                            required: field_required,
                            value: "{value}",
                            oninput: move |evt| value.set(evt.value()),
                        }
                    }
                }
                FieldType::Number => {
                    let field_id = field_id.to_string();
                    let field_name = field_name.to_string();
                    let placeholder = field_placeholder.as_deref().unwrap_or("");
                    rsx! {
                        input {
                            id: "{field_id}",
                            name: "{field_name}",
                            r#type: "number",
                            class: "px-4 h-12 bg-background-neutral-primary text-foreground-brandNeutral-secondary text-sm font-normal rounded-[0.625rem] border border-border-neutral-primary",
                            placeholder: "{placeholder}",
                            required: field_required,
                            value: "{value}",
                            oninput: move |evt| value.set(evt.value()),
                        }
                    }
                }
                FieldType::Textarea => {
                    let field_id = field_id.to_string();
                    let field_name = field_name.to_string();
                    let placeholder = field_placeholder.as_deref().unwrap_or("");
                    rsx! {
                        textarea {
                            id: "{field_id}",
                            name: "{field_name}",
                            class: "px-4 py-3 min-h-32 bg-background-neutral-primary text-foreground-brandNeutral-secondary text-sm font-normal placeholder:text-foreground-brandNeutral-secondary rounded-[0.625rem] border border-border-neutral-primary",
                            placeholder: "{placeholder}",
                            required: field_required,
                            value: "{value}",
                            oninput: move |evt| value.set(evt.value()),
                        }
                    }
                }
                FieldType::Select => {
                    let field_id = field_id.to_string();
                    let field_name = field_name.to_string();
                    let placeholder = field_placeholder.as_deref().unwrap_or("Select an option");
                    rsx! {
                        select {
                            id: "{field_id}",
                            name: "{field_name}",
                            class: "px-4 h-12 bg-background-neutral-primary text-foreground-brandNeutral-secondary text-sm font-normal rounded-[0.625rem] border border-border-neutral-primary",
                            required: field_required,
                            value: "{value}",
                            onchange: move |evt| value.set(evt.value()),
                            option { value: "", disabled: true, selected: value().is_empty(), "{placeholder}" }
                            if let Some(ref options) = field_options {
                                for option in options {
                                    option { value: "{option.value}", "{option.label}" }
                                }
                            }
                        }
                    }
                }
                FieldType::Radio => {
                    let field_name = field_name.clone();
                    let options = field_options.clone();
                    if let Some(options) = options {
                        rsx! {
                            div { class: "flex flex-col gap-2",
                                for option in options {
                                    label { class: "flex items-center gap-2 cursor-pointer",
                                        input {
                                            r#type: "radio",
                                            name: "{field_name}",
                                            value: "{option.value}",
                                            required: field_required,
                                            checked: value() == option.value,
                                            onchange: move |_| value.set(option.value.clone()),
                                        }
                                        span { class: "text-sm text-foreground-neutral-secondary", "{option.label}" }
                                    }
                                }
                            }
                        }
                    } else {
                        rsx! {
                            div {}
                        }
                    }
                }
                FieldType::Checkbox => {
                    let field_id = field_id.to_string();
                    let field_name = field_name.to_string();
                    rsx! {
                        div { class: "flex items-center gap-2",
                            input {
                                id: "{field_id}",
                                name: "{field_name}",
                                r#type: "checkbox",
                                required: field_required,
                                checked: value() == "true",
                                onchange: move |evt| {
                                    value.set(if evt.checked() { "true".to_string() } else { "false".to_string() })
                                },
                            }
                            span { class: "text-sm text-foreground-neutral-secondary", "yes" }
                        }
                    }
                }
                FieldType::CheckboxGroup => {
                    let field_name = field_name.to_string();
                    rsx! {
                        div { class: "flex flex-col gap-2",
                            if let Some(options) = field_options.as_ref() {
                                for option in options {
                                    {
                                        let option_value = option.value.clone();
                                        let field_name = field_name.clone();
                                        rsx! {
                                            label { class: "flex items-center gap-2 cursor-pointer",
                                                input {
                                                    r#type: "checkbox",
                                                    name: "{field_name}[]",
                                                    value: "{option_value}",
                                                    onchange: move |evt| {
                                                        let current = value();
                                                        let values: Vec<&str> = if current.is_empty() {
                                                            vec![]
                                                        } else {
                                                            current.split(',').collect()
                                                        };
                                                        if evt.checked() {
                                                            let mut new_values = values.clone();
                                                            if !new_values.contains(&option_value.as_str()) {
                                                                new_values.push(&option_value);
                                                            }
                                                            value.set(new_values.join(","));
                                                        } else {
                                                            let new_values: Vec<&str> = values
                                                                .into_iter()
                                                                .filter(|v| *v != option_value.as_str())
                                                                .collect();
                                                            value.set(new_values.join(","));
                                                        }
                                                    },
                                                }
                                                span { class: "text-sm text-foreground-brandNeutral-secondary", "{option.label}" }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
                FieldType::Date => {
                    let field_id = field_id.to_string();
                    let field_name = field_name.to_string();
                    rsx! {
                        input {
                            id: "{field_id}",
                            name: "{field_name}",
                            r#type: "date",
                            class: "px-4 h-12 bg-background-neutral-primary text-foreground-brandNeutral-secondary text-sm font-normal rounded-[0.625rem] border border-border-neutral-primary",
                            required: field_required,
                            value: "{value}",
                            oninput: move |evt| value.set(evt.value()),
                        }
                    }
                }
                FieldType::File => {
                    let field_id = field_id.to_string();
                    let field_name = field_name.clone();
                    let hackathon_slug = hackathon_slug.clone();
                    let mut is_uploading = use_signal(|| false);
                    let mut upload_error = use_signal(|| None::<String>);
                    rsx! {
                        div { class: "flex flex-col gap-2",
                            input {
                                id: "{field_id}",
                                name: "{field_name}",
                                r#type: "file",
                                class: "px-4 py-2 bg-background-neutral-primary text-foreground-brandNeutral-secondary text-sm font-normal rounded-[0.625rem] border border-border-neutral-primary",
                                required: field_required,
                                disabled: is_uploading(),
                                onchange: move |evt| {
                                    let files = evt.files();
                                    let field_name = field_name.clone();
                                    let slug = hackathon_slug.clone();
                                    spawn(async move {
                                        is_uploading.set(true);
                                        upload_error.set(None);
                                        if let Some(file_info) = files.first() {
                                            let file_name = file_info.name();
                                            match file_info.read_bytes().await {
                                                Ok(file_contents) => {
                                                    match crate::hackathons::handlers::file_upload::upload_application_file(
                                                            slug,
                                                            field_name,
                                                            file_contents.to_vec(),
                                                            file_name,
                                                        )
                                                        .await
                                                    {
                                                        Ok(response) => {
                                                            value.set(response.url);
                                                        }
                                                        Err(e) => {
                                                            upload_error.set(Some(format!("Upload failed: {}", e)));
                                                        }
                                                    }
                                                }
                                                Err(e) => {
                                                    upload_error.set(Some(format!("Failed to read file: {}", e)));
                                                }
                                            }
                                        }
                                        is_uploading.set(false);
                                    });
                                },
                            }
                            if is_uploading() {
                                p { class: "text-sm text-foreground-neutral-secondary", "Uploading..." }
                            }
                            if let Some(error) = upload_error() {
                                p { class: "text-sm text-status-danger-foreground", "{error}" }
                            }
                            if !value().is_empty() && !is_uploading() {
                                p { class: "text-sm text-status-success-foreground", "File uploaded" }
                            }
                        }
                    }
                }
            }

            if let Some(help_text) = field_help_text {
                p { class: "text-sm text-foreground-neutral-secondary", "{help_text}" }
            }
        }
    }
}
