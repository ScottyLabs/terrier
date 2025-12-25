use dioxus::{core::Element, logger::tracing, prelude::*};

use crate::Route;
use crate::domain::auth::handlers::get_current_user;
use crate::domain::hackathons::handlers::create::{
    CreateHackathonRequest, create_hackathon, upload_background, upload_banner,
};
use crate::ui::features::hackathon::form::{HackathonForm, HackathonFormFields};
use crate::ui::foundation::components::Header;

#[component]
pub fn CreateHackathon() -> Element {
    let user_future = use_server_future(get_current_user)?;
    let user = user_future();

    if !user
        .as_ref()
        .and_then(|r| r.as_ref().ok())
        .and_then(|u| u.as_ref())
        .map(|u| u.is_admin)
        .unwrap_or(false)
    {
        return rsx! {
            div { class: "p-7", "Unauthorized" }
        };
    }

    let form_fields = HackathonFormFields::new();
    let banner_url = use_signal(|| None::<String>);
    let banner_file = use_signal(|| None::<(Vec<u8>, String)>);
    let background_url = use_signal(|| None::<String>);
    let background_file = use_signal(|| None::<(Vec<u8>, String)>);
    let nav = navigator();

    let on_submit = move |evt: FormEvent| {
        evt.prevent_default();

        let name = form_fields.name.value.read().clone();
        let description = form_fields.description.value.read().clone();
        let start_date = form_fields.start_date.value.read().clone();
        let end_date = form_fields.end_date.value.read().clone();
        let banner_file_data = banner_file();
        let background_file_data = background_file();

        spawn(async move {
            // Create hackathon first
            let req = CreateHackathonRequest {
                name,
                description,
                start_date,
                end_date,
                max_team_size: None,
            };

            match create_hackathon(req).await {
                Ok(hackathon) => {
                    tracing::info!("Hackathon created: {}", hackathon.slug);

                    // Upload banner if present
                    if let Some((file_data, content_type)) = banner_file_data {
                        match upload_banner(hackathon.slug.clone(), file_data, content_type).await {
                            Ok(_) => tracing::info!("Banner uploaded successfully"),
                            Err(e) => tracing::error!("Failed to upload banner: {:?}", e),
                        }
                    }

                    // Upload background if present
                    if let Some((file_data, content_type)) = background_file_data {
                        match upload_background(hackathon.slug.clone(), file_data, content_type)
                            .await
                        {
                            Ok(_) => tracing::info!("Background uploaded successfully"),
                            Err(e) => tracing::error!("Failed to upload background: {:?}", e),
                        }
                    }

                    nav.push(Route::Home {});
                }
                Err(e) => {
                    tracing::error!("Failed to create hackathon: {:?}", e);

                    let error_msg = e.to_string().replace("'", "\\'");
                    let _ = document::eval(&format!(
                        "alert('Failed to create hackathon: {}')",
                        error_msg
                    ));
                }
            }
        });
    };

    rsx!(
        div { class: "flex flex-col p-7",
            Header {}
            h1 { class: "font-semibold text-3xl mt-3", "Create Hackathon" }

            div { class: "mt-7",
                HackathonForm {
                    fields: form_fields,
                    banner_url,
                    banner_file,
                    background_url,
                    background_file,
                    on_submit,
                    submit_label: "Create Hackathon".to_string(),
                }
            }
        }
    )
}
