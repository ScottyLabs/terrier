use dioxus::{logger::tracing, prelude::*};
use dioxus_forms::*;

use crate::{
    auth::{SETTINGS_ROLES, hooks::use_require_access_or_redirect},
    components::{Button, SaveStatus, SaveStatusIndicator, TabSwitcher},
    forms::{HackathonForm, HackathonFormFields},
    hackathons::{
        HackathonInfo,
        handlers::{
            UpdateHackathonRequest, delete_banner, set_form_config, toggle_registration,
            update_hackathon, upload_background, upload_banner,
        },
    },
};

#[derive(Clone, Copy, PartialEq)]
enum SettingsTab {
    General,
    Participation,
    Application,
}

#[component]
pub fn HackathonSettings(slug: String) -> Element {
    if let Some(no_access) = use_require_access_or_redirect(SETTINGS_ROLES) {
        return no_access;
    }

    let mut hackathon = use_context::<Signal<HackathonInfo>>();
    let active_tab = use_signal(|| SettingsTab::General);
    let mut save_status = use_signal(|| SaveStatus::Saved);

    let mut banner_url = use_signal(|| hackathon.read().banner_url.clone());
    let mut banner_file = use_signal(|| None::<(Vec<u8>, String)>);
    let mut background_url = use_signal(|| hackathon.read().background_url.clone());
    let mut background_file = use_signal(|| None::<(Vec<u8>, String)>);

    // Create max team size field with validation
    let max_team_size_field = use_form_field(hackathon.read().max_team_size)
        .with_validator(validators::min_value(1, "Max team size must be at least 1"));

    // Create form fields with initial values
    let name_field = use_form_field(hackathon.read().name.clone())
        .with_validator(validators::required("Name is required"));
    let description_field =
        use_form_field(hackathon.read().description.clone().unwrap_or_default())
            .with_validator(validators::required("Description is required"));

    // Format dates for datetime-local input (YYYY-MM-DDTHH:MM)
    let start_date_str = hackathon
        .read()
        .start_date
        .format("%Y-%m-%dT%H:%M")
        .to_string();
    let end_date_str = hackathon
        .read()
        .end_date
        .format("%Y-%m-%dT%H:%M")
        .to_string();

    let start_date_field = use_form_field(start_date_str)
        .with_validator(validators::required("Start date is required"));
    let end_date_field =
        use_form_field(end_date_str).with_validator(validators::required("End date is required"));

    // Mark fields as clean on mount
    use_hook(|| {
        name_field.mark_clean();
        description_field.mark_clean();
        start_date_field.mark_clean();
        end_date_field.mark_clean();
        max_team_size_field.mark_clean();
    });

    // Group fields for convenience
    let form_fields = HackathonFormFields {
        name: name_field.clone(),
        description: description_field.clone(),
        start_date: start_date_field.clone(),
        end_date: end_date_field.clone(),
    };

    // Clone for dirty tracking effect
    let name_for_effect = name_field.clone();
    let desc_for_effect = description_field.clone();
    let max_team_size_for_effect = max_team_size_field.clone();

    // Effect to detect changes and update save status (only for General/Participation tabs)
    use_effect(move || {
        // Only run dirty detection for General and Participation tabs
        let current_tab = active_tab();
        if current_tab == SettingsTab::Application {
            return;
        }

        // Subscribe to signal changes by reading them
        let name_val = name_for_effect.value.read().clone();
        let desc_val = desc_for_effect.value.read().clone();
        let team_size_val = *max_team_size_for_effect.value.read();
        let current_banner = banner_url();
        let original_banner = hackathon.read().banner_url.clone();

        // Check if any values differ from originals
        let name_orig = name_for_effect.original_value();
        let desc_orig = desc_for_effect.original_value();
        let name_dirty = name_val != name_orig;
        let desc_dirty = desc_val != desc_orig;
        let team_size_dirty = team_size_val != max_team_size_for_effect.original_value();
        let banner_dirty = current_banner != original_banner;

        let has_changes = name_dirty || desc_dirty || team_size_dirty || banner_dirty;

        if has_changes && save_status() == SaveStatus::Saved {
            save_status.set(SaveStatus::Unsaved);
        } else if !has_changes && save_status() == SaveStatus::Unsaved {
            save_status.set(SaveStatus::Saved);
        }
    });

    let tabs = vec![
        (SettingsTab::General, "General".to_string()),
        (SettingsTab::Participation, "Participation".to_string()),
        (SettingsTab::Application, "Application".to_string()),
    ];

    rsx! {
        div {
            div { class: "flex flex-col md:flex-row justify-between md:items-center gap-3 pt-6 md:pt-11 pb-4 md:pb-7",
                h1 { class: "text-2xl md:text-[30px] font-semibold leading-8 md:leading-[38px] text-foreground-neutral-primary",
                    "Settings"
                }
                SaveStatusIndicator {
                    status: save_status(),
                    last_saved: Some(hackathon.read().updated_at),
                }
            }

            // Tab switcher
            div { class: "mb-4 md:mb-6",
                TabSwitcher { active_tab, tabs }
            }

            // Tab content
            div {
                match active_tab() {
                    SettingsTab::General => {
                        let name_field = form_fields.name.clone();
                        let desc_field = form_fields.description.clone();
                        let team_size_field = max_team_size_field.clone();
                        rsx! {
                            HackathonForm {
                                fields: form_fields,
                                banner_url,
                                banner_file,
                                background_url,
                                background_file,
                                on_submit: move |evt: FormEvent| {
                                    evt.prevent_default();
                                    let slug_clone = slug.clone();
                                    save_status.set(SaveStatus::Saving);
                                    let name_field_clean = name_field.clone();
                                    let desc_field_clean = desc_field.clone();
                                    let team_size_field_clean = team_size_field.clone();
                                    let banner_file_data = banner_file();
                                    let background_file_data = background_file();
                                    let name_val = name_field.value.read().clone();
                                    let desc_val = desc_field.value.read().clone();
                                    let team_size_val = *team_size_field.value.read();
                                    spawn(async move {
                                        let req = UpdateHackathonRequest {
                                            name: name_val,
                                            description: desc_val,
                                            max_team_size: team_size_val,
                                        };
                                        match update_hackathon(slug_clone.clone(), req).await {
                                            Ok(updated_info) => {
                                                tracing::info!("Settings updated successfully");
                                                hackathon.set(updated_info.clone());
                                                name_field_clean.mark_clean();
                                                desc_field_clean.mark_clean();
                                                team_size_field_clean.mark_clean();
                                                if let Some((file_data, content_type)) = banner_file_data {
                                                    tracing::info!(
                                                        "Banner file data present, uploading new banner. File size: {} bytes, content type: {}",
                                                        file_data.len(), content_type
                                                    );
                                                    match upload_banner(slug_clone.clone(), file_data, content_type)
                                                        .await
                                                    {
                                                        Ok(url) => {
                                                            tracing::info!("New banner uploaded successfully: {}", url);
                                                            banner_url.set(Some(url.clone()));
                                                            banner_file.set(None);
                                                            let mut h = hackathon.write();
                                                            h.banner_url = Some(url);
                                                        }
                                                        Err(e) => {
                                                            tracing::error!("Failed to upload banner: {:?}", e);
                                                            let error_msg = format!("Banner upload failed: {}", e);
                                                            let _ = document::eval(
                                                                &format!("alert('{}')", error_msg.replace("'", "\\'")),
                                                            );
                                                        }
                                                    }
                                                } else if banner_url().is_none() && updated_info.banner_url.is_some()
                                                {
                                                    match delete_banner(slug_clone.clone()).await {
                                                        Ok(_) => {
                                                            tracing::info!("Banner deleted successfully");
                                                            let mut h = hackathon.write();
                                                            h.banner_url = None;
                                                        }
                                                        Err(e) => tracing::error!("Failed to delete banner: {:?}", e),
                                                    }
                                                } else {
                                                    banner_url.set(updated_info.banner_url.clone());
                                                }

                            // Handle background upload

                            // Hidden fields for name, description, banner
                            // Registration Status Toggle
                            // Preset Selector
            
            

                                                if let Some((file_data, content_type)) = background_file_data {
                                                    tracing::info!(
                                                        "Background file data present, uploading new background. File size: {} bytes, content type: {}",
                                                        file_data.len(), content_type
                                                    );
                                                    match upload_background(
            
                                                            slug_clone.clone(),
                                                            file_data,
                                                            content_type,
                                                        )
                                                        .await
                                                    {
                                                        Ok(url) => {
                                                            tracing::info!(
                                                                "New background uploaded successfully: {}", url
                                                            );
                                                            background_url.set(Some(url.clone()));
                                                            background_file.set(None);
                                                            let mut h = hackathon.write();
                                                            h.background_url = Some(url);
                                                        }
                                                        Err(e) => {
                                                            tracing::error!("Failed to upload background: {:?}", e);
                                                            let error_msg = format!("Background upload failed: {}", e);
                                                            let _ = document::eval(
                                                                &format!("alert('{}')", error_msg.replace("'", "\\'")),
                                                            );
                                                        }
                                                    }
                                                } else {
                                                    background_url.set(updated_info.background_url.clone());
                                                }
                                                save_status.set(SaveStatus::Saved);
                                                let _ = document::eval("alert('Settings saved!')");
                                            }
                                            Err(e) => {
                                                tracing::error!("Failed to update settings: {:?}", e);
                                                save_status.set(SaveStatus::Unsaved);
                                                let error_msg = e.to_string().replace("'", "\\'");
                                                let _ = document::eval(
                                                    &format!("alert('Failed to save: {}')", error_msg),
                                                );
                                            }
                                        }
                                    });
                                },
                                submit_label: "Save".to_string(),
                            }
                        }
                    }
                    SettingsTab::Participation => {
                        let max_team_size_for_save = max_team_size_field.clone();
                        let max_team_size_for_validation = max_team_size_field.clone();
                        let mut max_team_size_for_input = max_team_size_field.clone();
                        let mut max_team_size_for_blur = max_team_size_field.clone();
                        let max_team_size_for_display = max_team_size_field.clone();
                        let max_team_size_for_error = max_team_size_field.clone();
                        let name_field2 = form_fields.name.clone();
                        let desc_field2 = form_fields.description.clone();
                        rsx! {
                            form {
                                class: "flex flex-col gap-5",
                                onsubmit: move |evt: FormEvent| {
                                    evt.prevent_default();
                                    let mut validation_clone = max_team_size_for_validation.clone();
                                    if !validation_clone.validate() {
                                        return;
                                    }
                                    let slug_clone = slug.clone();
                                    save_status.set(SaveStatus::Saving);
                                    let name_field2_clean = name_field2.clone();
                                    let desc_field2_clean = desc_field2.clone();
                                    let max_team_size_clean = max_team_size_for_save.clone();
                                    let name_val = name_field2.value.read().clone();
                                    let desc_val = desc_field2.value.read().clone();
                                    let team_size_val = *max_team_size_for_save.value.read();
                                    spawn(async move {
                                        let req = UpdateHackathonRequest {
                                            name: name_val,
                                            description: desc_val,
                                            max_team_size: team_size_val,
                                        };
                                        match update_hackathon(slug_clone.clone(), req).await {
                                            Ok(updated_info) => {
                                                tracing::info!("Hackathon updated successfully");
                                                hackathon.set(updated_info.clone());
                                                name_field2_clean.mark_clean();
                                                desc_field2_clean.mark_clean();
                                                max_team_size_clean.mark_clean();
                                                banner_url.set(updated_info.banner_url.clone());
                                                save_status.set(SaveStatus::Saved);
                                                let _ = document::eval("alert('Settings saved successfully!')");
                                            }
                                            Err(e) => {
                                                tracing::error!("Failed to update hackathon: {:?}", e);
                                                save_status.set(SaveStatus::Unsaved);
                                                let error_msg = e.to_string().replace("'", "\\'");
                                                let _ = document::eval(
                                                    &format!("alert('Failed to save settings: {}')", error_msg),
                                                );
                                            }
                                        }
                                    });
                                },
                                input {
                                    r#type: "hidden",
                                    name: "name",
                                    value: "{form_fields.name.value.read()}",
                                }
                                input {
                                    r#type: "hidden",
                                    name: "description",
                                    value: "{form_fields.description.value.read()}",
                                }
                                div { class: "flex flex-col gap-2",
                                    label { class: "text-base font-medium text-foreground-neutral-primary", "Max Team Size" }
                                    input {
                                        class: "px-4 h-12 bg-background-neutral-primary text-foreground-brandNeutral-secondary text-sm font-normal rounded-[0.625rem]",
                                        r#type: "number",
                                        name: "max_team_size",
                                        min: "1",
                                        value: "{max_team_size_for_display.value.read()}",
                                        oninput: move |evt| {
                                            if let Ok(num) = evt.value().parse::<i32>() {
                                                max_team_size_for_input.value.set(num);
                                            }
                                        },
                                        onblur: move |_| {
                                            max_team_size_for_blur.mark_touched();
                                            max_team_size_for_blur.validate();
                                        },
                                    }
                                    if max_team_size_for_error.is_touched() {
                                        if let Some(error) = max_team_size_for_error.error.read().as_ref() {
                                            span { class: "text-sm text-status-danger-foreground", "{error}" }
                                        }
                                    }
                                }
                                div { class: "mt-12",
                                    Button { button_type: "submit".to_string(), "Save" }
                                }
                            }
                        }
                    }
                    SettingsTab::Application => {
                        let mut is_active = use_signal(|| hackathon.read().is_active);
                        let initial_preset = hackathon
                            .read()
                            .form_config
                            .as_ref()
                            .and_then(|config| {
                                if let Ok(schema) = serde_json::from_value::<
                                    crate::schemas::FormSchema,
                                >(config.clone()) {
                                    if schema.fields.iter().any(|f| f.id == "mlh_code_of_conduct") {
                                        return Some("tartanhacks".to_string());
                                    }
                                }
                                None
                            })
                            .unwrap_or_else(|| "none".to_string());
                        let preset_for_selected = initial_preset.clone();
                        let preset_for_original = initial_preset.clone();
                        let mut selected_preset = use_signal(move || preset_for_selected);
                        let original_preset = use_signal(move || preset_for_original);
                        let slug_for_toggle = slug.clone();
                        let slug_for_preset = slug.clone();
                        let mut status = save_status;
                        rsx! {
                            div { class: "flex flex-col gap-6",
                                div { class: "flex flex-col gap-4",
                                    h2 { class: "text-xl font-semibold", "Registration Status" }
                                    p { class: "text-foreground-neutral-secondary",
                                        if is_active() {
                                            "Registration is currently open."
                                        } else {
                                            "Registration is currently closed."
                                        }
                                    }
                                    div { class: "flex",
                                        Button {
                                            button_type: "button".to_string(),
                                            onclick: move |_| {
                                                let slug = slug_for_toggle.clone();
                                                spawn(async move {
                                                    match toggle_registration(slug).await {
                                                        Ok(new_status) => {
                                                            is_active.set(new_status);
                                                            let mut h = hackathon.write();
                                                            h.is_active = new_status;
                                                            let status_text = if new_status { "open" } else { "closed" };
                                                            let _ = document::eval(
                                                                &format!("alert('Registration is now {}')", status_text),
                                                            );
                                                        }
                                                        Err(e) => {
                                                            let error_msg = e.to_string().replace("'", "\\'");
                                                            let _ = document::eval(
                                                                &format!("alert('Failed to toggle registration: {}')", error_msg),
                                                            );
                                                        }
                                                    }
                                                });
                                            },
                                            if is_active() {
                                                "Close Registration"
                                            } else {
                                                "Open Registration"
                                            }
                                        }
                                    }
                                }
                                div { class: "flex flex-col gap-4",
                                    h2 { class: "text-xl font-semibold", "Application Form" }
                                    div { class: "flex flex-col gap-2",
                                        label { class: "text-base font-medium text-foregrournd-neutral-primary", "Form Preset" }
                                        select {
                                            class: "px-4 h-12 bg-background-neutral-primary text-foreground-brandNeutral-secondary text-sm font-normal rounded-[0.625rem] border border-border-neutral-primary",
                                            value: "{selected_preset}",
                                            onchange: move |evt| {
                                                let new_value = evt.value();
                                                selected_preset.set(new_value.clone());
                                                if new_value != original_preset() {
                                                    status.set(SaveStatus::Unsaved);
                                                } else {
                                                    status.set(SaveStatus::Saved);
                                                }
                                            },
                                            option { value: "none", "Select a preset" }
                                            option { value: "tartanhacks", "TartanHacks" }
                                        }
                                    }
                                    if selected_preset() != "none" {
                                        div { class: "flex gap-4",
                                            Button {
                                                button_type: "button".to_string(),
                                                onclick: move |_| {
                                                    status.set(SaveStatus::Saving);
                                                    let slug = slug_for_preset.clone();
                                                    let preset = selected_preset();
                                                    spawn(async move {
                                                        use crate::schemas::presets::tartanhacks_preset;
                                                        let form_schema = match preset.as_str() {
                                                            "tartanhacks" => tartanhacks_preset(),
                                                            _ => return,
                                                        };
                                                        match set_form_config(slug, form_schema).await {
                                                            Ok(_) => {
                                                                status.set(SaveStatus::Saved);
                                                                let _ = document::eval("alert('Form preset applied successfully!')");
                                                            }
                                                            Err(e) => {
                                                                status.set(SaveStatus::Unsaved);
                                                                let error_msg = e.to_string().replace("'", "\\'");
                                                                let _ = document::eval(
                                                                    &format!("alert('Failed to apply preset: {}')", error_msg),
                                                                );
                                                            }
                                                        }
                                                    });
                                                },
                                                "Apply Preset"
                                            }
                                        }
                                    }
                                    if hackathon.read().form_config.is_some() {
                                        p { class: "text-sm text-foreground-neutral-secondary",
                                            "A form is currently configured for this hackathon."
                                        }
                                    } else {
                                        p { class: "text-sm text-foreground-neutral-secondary",
                                            "No form configured yet. Select and apply a preset to enable applications."
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}
