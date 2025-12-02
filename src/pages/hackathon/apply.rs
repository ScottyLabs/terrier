use crate::{
    auth::{APPLY_ROLES, HackathonRole, hooks::use_require_access_or_redirect},
    components::{
        ApplicationStatus, ApplicationStatusVariant, Button, Checkbox, CheckboxGroup,
        FormSelectOption, Input, InputHeight, InputVariant, RadioGroup, SaveStatus,
        SaveStatusIndicator, Select,
    },
    hackathons::HackathonInfo,
    schemas::FormSchema,
};
use dioxus::prelude::*;
use dioxus_free_icons::{
    Icon,
    icons::ld_icons::{LdCheck, LdFile, LdFileText, LdX},
};

#[component]
pub fn HackathonApply(slug: String) -> Element {
    if let Some(no_access) = use_require_access_or_redirect(APPLY_ROLES) {
        return no_access;
    }

    let hackathon = use_context::<Signal<HackathonInfo>>();
    let role = use_context::<Option<HackathonRole>>();

    // Check if user has already submitted an application
    let slug_clone = slug.clone();
    let application_status = use_resource(move || {
        let slug = slug_clone.clone();
        async move {
            match crate::hackathons::handlers::applications::get_application(slug).await {
                Ok(app) => Some(app.status),
                Err(_) => None,
            }
        }
    });

    // Parse form config from hackathon
    let form_schema = use_memo(move || {
        hackathon
            .read()
            .form_config
            .as_ref()
            .and_then(|config| serde_json::from_value::<FormSchema>(config.clone()).ok())
    });

    rsx! {
        div { class: "h-full flex flex-col",
            div { class: "flex-1 flex items-center justify-center",
                if !hackathon.read().is_active {
                    div { class: "text-center py-12",
                        h2 { class: "text-2xl font-semibold text-foreground-neutral-primary mb-4",
                            "Registration Closed"
                        }
                        p { class: "text-foreground-neutral-secondary",
                            "Applications are not currently being accepted for this hackathon."
                        }
                    }
                } else if role.as_ref().map(|r| r.role == "participant").unwrap_or(false) {
                    // User has been accepted and is now a participant
                    ApplicationStatus {
                        variant: ApplicationStatusVariant::Accepted,
                        hackathon_slug: slug.clone(),
                        application_status,
                    }
                } else if let Some(Some(status)) = application_status.read().as_ref() {
                    // User has submitted an application
                    if status == "pending" || status == "accepted" || status == "rejected" {
                        ApplicationStatus {
                            variant: ApplicationStatusVariant::Submitted,
                            hackathon_slug: slug.clone(),
                            application_status,
                        }
                    } else if let Some(schema) = form_schema() {
                        // Draft or other status, show form
                        ApplicationForm { schema, application_status }
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
                } else if let Some(schema) = form_schema() {
                    // No application yet, show form
                    ApplicationForm { schema, application_status }
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
fn ApplicationForm(schema: FormSchema, application_status: Resource<Option<String>>) -> Element {
    let hackathon = use_context::<Signal<HackathonInfo>>();
    let _nav = navigator();

    let mut form_values = use_signal(|| std::collections::HashMap::<String, String>::new());
    let mut is_submitting = use_signal(|| false);
    let mut error_message = use_signal(|| None::<String>);
    let mut save_status = use_signal(|| SaveStatus::Saved);
    let mut last_saved = use_signal(|| None::<chrono::NaiveDateTime>);
    let mut last_saved_values = use_signal(|| std::collections::HashMap::<String, String>::new());
    let autosave_trigger = use_signal(|| 0u32);
    let mut draft_loaded = use_signal(|| false);

    // If application submitted, let parent render ApplicationStatus
    if let Some(Some(status)) = application_status.read().as_ref() {
        if status == "pending" || status == "accepted" || status == "rejected" {
            return rsx! {
                div { class: "flex items-center justify-center py-12",
                    p { class: "text-foreground-neutral-primary", "Loading..." }
                }
            };
        }
    }

    use_effect(move || {
        if *draft_loaded.peek() {
            return;
        }
        draft_loaded.set(true);

        let slug = hackathon.read().slug.clone();
        spawn(async move {
            match crate::hackathons::handlers::applications::get_application(slug).await {
                Ok(app_data) if app_data.status == "draft" => {
                    if let Ok(data_map) = serde_json::from_value::<
                        std::collections::HashMap<String, String>,
                    >(app_data.form_data)
                    {
                        if !data_map.is_empty() {
                            form_values.set(data_map.clone());
                            last_saved_values.set(data_map);
                        }
                    }
                }
                _ => {}
            }
        });
    });

    {
        let mut debounce_timer = use_signal(|| 0u32);
        use_effect(move || {
            autosave_trigger();

            let values = form_values.peek().clone();
            if values.is_empty() || values == *last_saved_values.peek() {
                return;
            }

            let current = *debounce_timer.peek();
            debounce_timer.set(current + 1);
            let current_timer = current + 1;
            let slug = hackathon.read().slug.clone();

            spawn(async move {
                gloo_timers::future::sleep(std::time::Duration::from_millis(1000)).await;
                if *debounce_timer.peek() != current_timer {
                    return;
                }

                if *save_status.peek() != SaveStatus::Saving {
                    save_status.set(SaveStatus::Saving);
                    let form_data = serde_json::to_value(&values).unwrap_or_default();
                    match crate::hackathons::handlers::applications::update_application(
                        slug, form_data,
                    )
                    .await
                    {
                        Ok(app_data) => {
                            last_saved_values.set(values);
                            save_status.set(SaveStatus::Saved);
                            if let Ok(timestamp) = chrono::NaiveDateTime::parse_from_str(
                                &app_data.updated_at,
                                "%Y-%m-%d %H:%M:%S%.f",
                            ) {
                                last_saved.set(Some(timestamp));
                            }
                        }
                        Err(_) => {
                            save_status.set(SaveStatus::Unsaved);
                        }
                    }
                }
            });
        });
    }

    let handle_submit = move |evt: Event<FormData>| {
        evt.prevent_default();
        spawn(async move {
            is_submitting.set(true);
            error_message.set(None);

            let slug = hackathon.read().slug.clone();
            let form_data = serde_json::to_value(form_values()).unwrap_or_default();

            match crate::hackathons::handlers::applications::submit_application(slug, form_data)
                .await
            {
                Ok(_) => {
                    // Clear form values and update last saved to match
                    last_saved_values.set(form_values());

                    // Reload application status to show submitted view
                    application_status.restart();
                }
                Err(e) => {
                    error_message.set(Some(format!("Failed to submit application: {}", e)));
                    is_submitting.set(false);
                }
            }
        });
    };

    // Group fields by section, preserving order
    let sections = use_memo(move || {
        let mut grouped: std::collections::HashMap<String, Vec<crate::schemas::FormField>> =
            std::collections::HashMap::new();

        for field in schema.fields.iter() {
            let section_name = field.section.clone().unwrap_or_else(|| "Other".to_string());
            grouped
                .entry(section_name)
                .or_insert_with(Vec::new)
                .push(field.clone());
        }

        // Convert to vector and sort by minimum order in each section
        let mut sections_vec: Vec<(String, Vec<crate::schemas::FormField>)> =
            grouped.into_iter().collect();
        sections_vec.sort_by_key(|(_, fields)| fields.iter().map(|f| f.order).min().unwrap_or(0));

        sections_vec
    });

    rsx! {
        div { class: "flex flex-col gap-6 max-w-3xl",
            div { class: "flex justify-between items-center pt-11 pb-7",
                h1 { class: "text-[30px] font-semibold leading-[38px] text-foreground-neutral-primary",
                    "Application"
                }
                SaveStatusIndicator { status: save_status(), last_saved: last_saved() }
            }

            if let Some(error) = error_message() {
                div { class: "mb-4 p-4 bg-status-danger-background text-status-danger-foreground rounded-lg",
                    "{error}"
                }
            }

            form { class: "flex flex-col gap-6", onsubmit: handle_submit,
                for (section_name , fields) in sections().iter() {
                    div { class: "bg-background-neutral-primary rounded-lg p-8",
                        h2 { class: "text-xl font-semibold text-foreground-neutral-primary mb-6",
                            "{section_name}"
                        }
                        div { class: "flex flex-col gap-6",
                            for field in fields.iter() {
                                FormFieldRenderer {
                                    field: field.clone(),
                                    form_values,
                                    autosave_trigger,
                                    hackathon_slug: hackathon.read().slug.clone(),
                                }
                            }
                        }
                    }
                }

                div { class: "mt-2",
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
    autosave_trigger: Signal<u32>,
    hackathon_slug: String,
) -> Element {
    use crate::schemas::FieldType;

    // Clone field data before hooks
    let field_id = field.id.clone();
    let field_name = field.name.clone();
    let field_label = field.label.clone();
    let field_type = field.field_type.clone();
    let field_required = field.required;
    let field_help_text = field.help_text.clone();
    let field_default = field.default_value.clone().unwrap_or_default();
    let field_conditional = field.conditional.clone();

    let field_name_for_handlers = field_name.clone();
    let field_name_for_sync = field_name.clone();

    let initial_value = form_values
        .peek()
        .get(&field_name)
        .cloned()
        .unwrap_or(field_default.clone());
    let mut value = use_signal(|| initial_value);

    // Sync value signal when form_values changes
    use_effect(move || {
        let current_form_value = form_values.read().get(&field_name_for_sync).cloned();
        if let Some(new_value) = current_form_value {
            if new_value != *value.peek() {
                value.set(new_value);
            }
        }
    });

    let should_show = || {
        if let Some(condition) = &field_conditional {
            let values = form_values.peek();
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
    };

    if !should_show() {
        return rsx! {
            div { style: "display: none;" }
        };
    }

    rsx! {
        div { class: "flex flex-col gap-2",
            match field_type.clone() {
                FieldType::Text { placeholder, .. }
                | FieldType::Email { placeholder, .. }
                | FieldType::Tel { placeholder }
                | FieldType::Url { placeholder } => {
                    let input_type = match field_type {
                        FieldType::Email { .. } => "email",
                        FieldType::Tel { .. } => "tel",
                        FieldType::Url { .. } => "url",
                        _ => "text",
                    };
                    rsx! {
                        Input {
                            label: field_label,
                            placeholder,
                            value,
                            variant: InputVariant::Default,
                            input_type: input_type.to_string(),
                            name: Some(field_name),
                            id: Some(field_id),
                            required: field_required,
                            help_text: field_help_text.clone(),
                            oninput: move |evt: Event<FormData>| {
                                let new_value = evt.value();
                                {
                                    let mut values = form_values.write();
                                    if !new_value.is_empty() {
                                        values.insert(field_name_for_handlers.clone(), new_value.clone());
                                    } else {
                                        values.remove(&field_name_for_handlers);
                                    }
                                }
                                let current = *autosave_trigger.peek();
                                autosave_trigger.set(current + 1);
                            },
                        }
                    }
                }
                FieldType::Number { placeholder, .. } => {
                    rsx! {
                        Input {
                            label: field_label,
                            placeholder,
                            value,
                            variant: InputVariant::Default,
                            input_type: "number".to_string(),
                            name: Some(field_name),
                            id: Some(field_id),
                            required: field_required,
                            help_text: field_help_text.clone(),
                            oninput: move |evt: Event<FormData>| {
                                let new_value = evt.value();
                                {
                                    let mut values = form_values.write();
                                    if !new_value.is_empty() {
                                        values.insert(field_name_for_handlers.clone(), new_value.clone());
                                    } else {
                                        values.remove(&field_name_for_handlers);
                                    }
                                }
                                let current = *autosave_trigger.peek();
                                autosave_trigger.set(current + 1);
                            },
                        }
                    }
                }
                FieldType::Textarea { placeholder } => {
                    rsx! {
                        Input {
                            label: field_label,
                            placeholder,
                            value,
                            height: InputHeight::Tall,
                            variant: InputVariant::Default,
                            name: Some(field_name),
                            id: Some(field_id),
                            required: field_required,
                            help_text: field_help_text.clone(),
                            oninput: move |evt: Event<FormData>| {
                                let new_value = evt.value();
                                {
                                    let mut values = form_values.write();
                                    if !new_value.is_empty() {
                                        values.insert(field_name_for_handlers.clone(), new_value.clone());
                                    } else {
                                        values.remove(&field_name_for_handlers);
                                    }
                                }
                                let current = *autosave_trigger.peek();
                                autosave_trigger.set(current + 1);
                            },
                        }
                    }
                }
                FieldType::Select { options, placeholder } => {
                    let select_options = options
                        .into_iter()
                        .map(|o| FormSelectOption {
                            label: o.label,
                            value: o.value,
                        })
                        .collect();
                    rsx! {
                        Select {
                            label: field_label,
                            options: select_options,
                            value,
                            placeholder,
                            name: Some(field_name),
                            id: Some(field_id),
                            required: field_required,
                            onchange: move |evt: Event<FormData>| {
                                let new_value = evt.value();
                                {
                                    let mut values = form_values.write();
                                    if !new_value.is_empty() {
                                        values.insert(field_name_for_handlers.clone(), new_value.clone());
                                    } else {
                                        values.remove(&field_name_for_handlers);
                                    }
                                }
                                let current = *autosave_trigger.peek();
                                autosave_trigger.set(current + 1);
                            },
                        }
                    }
                }
                FieldType::Radio { options } => {
                    let radio_options = options
                        .into_iter()
                        .map(|o| FormSelectOption {
                            label: o.label,
                            value: o.value,
                        })
                        .collect();
                    rsx! {
                        RadioGroup {
                            label: field_label,
                            options: radio_options,
                            value,
                            name: Some(field_name),
                            required: field_required,
                            onchange: move |new_value: String| {
                                {
                                    let mut values = form_values.write();
                                    if !new_value.is_empty() {
                                        values.insert(field_name_for_handlers.clone(), new_value.clone());
                                    } else {
                                        values.remove(&field_name_for_handlers);
                                    }
                                }
                                let current = *autosave_trigger.peek();
                                autosave_trigger.set(current + 1);
                            },
                        }
                    }
                }
                FieldType::Checkbox => {
                    rsx! {
                        Checkbox {
                            label: field_label,
                            value,
                            name: Some(field_name),
                            id: Some(field_id),
                            required: field_required,
                            onchange: move |new_value: String| {
                                {
                                    let mut values = form_values.write();
                                    if !new_value.is_empty() {
                                        values.insert(field_name_for_handlers.clone(), new_value.clone());
                                    } else {
                                        values.remove(&field_name_for_handlers);
                                    }
                                }
                                let current = *autosave_trigger.peek();
                                autosave_trigger.set(current + 1);
                            },
                        }
                    }
                }
                FieldType::CheckboxGroup { options } => {
                    let checkbox_options = options
                        .into_iter()
                        .map(|o| FormSelectOption {
                            label: o.label,
                            value: o.value,
                        })
                        .collect();
                    rsx! {
                        CheckboxGroup {
                            label: field_label,
                            options: checkbox_options,
                            value,
                            name: Some(field_name),
                            required: field_required,
                            onchange: move |new_value: String| {
                                {
                                    let mut values = form_values.write();
                                    if !new_value.is_empty() {
                                        values.insert(field_name_for_handlers.clone(), new_value.clone());
                                    } else {
                                        values.remove(&field_name_for_handlers);
                                    }
                                }
                                let current = *autosave_trigger.peek();
                                autosave_trigger.set(current + 1);
                            },
                        }
                    }
                }
                FieldType::Date => {
                    rsx! {
                        Input {
                            label: field_label,
                            placeholder: None,
                            value,
                            variant: InputVariant::Default,
                            input_type: "date".to_string(),
                            name: Some(field_name),
                            id: Some(field_id),
                            required: field_required,
                            help_text: field_help_text.clone(),
                            oninput: move |evt: Event<FormData>| {
                                let new_value = evt.value();
                                {
                                    let mut values = form_values.write();
                                    if !new_value.is_empty() {
                                        values.insert(field_name_for_handlers.clone(), new_value.clone());
                                    } else {
                                        values.remove(&field_name_for_handlers);
                                    }
                                }
                                let current = *autosave_trigger.peek();
                                autosave_trigger.set(current + 1);
                            },
                        }
                    }
                }
                FieldType::File { validation, .. } => {
                    let field_id = field_id.to_string();
                    let field_name_for_file = field_name.clone();
                    let field_name_for_input = field_name_for_file.clone();
                    let field_name_for_delete = field_name_for_file.clone();
                    let field_name_for_upload_handler = field_name_for_handlers.clone();
                    let field_name_for_delete_handler = field_name_for_handlers.clone();
                    let hackathon_slug = hackathon_slug.clone();
                    let hackathon_slug_for_delete = hackathon_slug.clone();
                    let accept_attr = validation.as_ref().and_then(|v| v.accept.clone());
                    let mut is_uploading = use_signal(|| false);
                    let mut is_deleting = use_signal(|| false);
                    let mut upload_error = use_signal(|| None::<String>);
                    let mut selected_file = use_signal(|| None::<String>);
                    rsx! {
                        label {
                            class: "text-base font-medium text-foreground-neutral-primary",
                            r#for: "{field_id}",
                            "{field_label}"
                            if field_required {
                                span { class: "text-status-danger-foreground ml-1", "*" }
                            }
                        }
                        div { class: "flex flex-col gap-2",
                            input {
                                id: "{field_id}",
                                name: "{field_name_for_input}",
                                r#type: "file",
                                class: "hidden",
                                accept: accept_attr,
                                required: field_required && value().is_empty(),
                                disabled: is_uploading() || is_deleting(),
                                onchange: move |evt| {
                                    let files = evt.files();
                                    let field_name = field_name.clone();
                                    let field_name_for_values = field_name_for_upload_handler.clone();
                                    let slug = hackathon_slug.clone();
                                    spawn(async move {
                                        is_uploading.set(true);
                                        upload_error.set(None);
                                        if let Some(file_info) = files.first() {
                                            let file_name = file_info.name();
                                            selected_file.set(Some(file_name.clone()));
                                            match file_info.read_bytes().await {
                                                Ok(file_contents) => {
                                                    match crate::hackathons::handlers::file_upload::upload_application_file(
                                                            slug,
                                                            field_name.clone(),
                                                            file_contents.to_vec(),
                                                            file_name,
                                                        )
                                                        .await
                                                    {
                                                        Ok(response) => {
                                                            let new_value = response.url;
                                                            value.set(new_value.clone());
                                                            form_values
                                                                .write()
                                                                .insert(field_name_for_values.clone(), new_value);
                                                            autosave_trigger.set(autosave_trigger() + 1);
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

                            if !value().is_empty() && !is_uploading() {
                                div { class: "flex items-center gap-3 p-4 bg-background-brandNeutral-secondary rounded-lg",
                                    Icon {
                                        width: 32,
                                        height: 32,
                                        icon: LdFile,
                                        class: "text-foreground-neutral-secondary shrink-0",
                                    }
                                    div { class: "flex-1 min-w-0",
                                        p { class: "text-sm font-medium text-foreground-neutral-primary truncate",
                                            if let Some(file) = selected_file() {
                                                "{file}"
                                            } else {
                                                "Uploaded file"
                                            }
                                        }
                                        div { class: "flex items-center gap-1",
                                            Icon {
                                                width: 12,
                                                height: 12,
                                                icon: LdCheck,
                                                class: "text-status-success-foreground",
                                            }
                                            p { class: "text-xs text-status-success-foreground", "Uploaded" }
                                        }
                                    }
                                    button {
                                        r#type: "button",
                                        class: "p-2 hover:bg-background-neutral-tertiary rounded-md transition-colors",
                                        disabled: is_deleting(),
                                        onclick: move |_| {
                                            let field_name = field_name_for_delete.clone();
                                            let field_name_for_values = field_name_for_delete_handler.clone();
                                            let slug = hackathon_slug_for_delete.clone();
                                            spawn(async move {
                                                is_deleting.set(true);
                                                upload_error.set(None);
                                                match crate::hackathons::handlers::file_upload::delete_application_file(
                                                        slug,
                                                        field_name.clone(),
                                                    )
                                                    .await
                                                {
                                                    Ok(_) => {
                                                        value.set(String::new());
                                                        form_values.write().remove(&field_name_for_values);
                                                        selected_file.set(None);
                                                        autosave_trigger.set(autosave_trigger() + 1);
                                                    }
                                                    Err(e) => {
                                                        upload_error.set(Some(format!("Delete failed: {}", e)));
                                                    }
                                                }
                                                is_deleting.set(false);
                                            });
                                        },
                                        Icon {
                                            width: 20,
                                            height: 20,
                                            icon: LdX,
                                            class: "text-status-danger-foreground",
                                        }
                                    }
                                }
                            }

                            label {
                                r#for: "{field_id}",
                                class: "flex items-center justify-center gap-2 h-12 px-4 bg-background-brandNeutral-secondary text-foreground-brandNeutral-secondary text-sm font-normal rounded-[0.625rem] cursor-pointer hover:opacity-90",
                                Icon { width: 20, height: 20, icon: LdFileText }
                                if !value().is_empty() && !is_uploading() {
                                    "Change file"
                                } else {
                                    "Choose file"
                                }
                            }

                            if is_uploading() {
                                p { class: "text-sm text-foreground-neutral-secondary", "Uploading..." }
                            }
                            if is_deleting() {
                                p { class: "text-sm text-foreground-neutral-secondary", "Deleting..." }
                            }
                            if let Some(error) = upload_error() {
                                p { class: "text-sm text-status-danger-foreground", "{error}" }
                            }
                        }
                    }
                }
            }
        }
    }
}
