use dioxus::prelude::*;

use crate::{
    Route, auth::hooks::use_hackathon_role, components::Sidebar,
};

#[component]
pub fn HackathonLayout(slug: String) -> Element {
    let nav = navigator();
    let slug_for_hackathon = slug.clone();
    let slug_for_role = slug.clone();

    // Fetch hackathon data
    let hackathon_resource = use_resource(move || {
        let s = slug_for_hackathon.clone();
        async move {
            crate::hackathons::handlers::get_hackathon_by_slug(s).await
        }
    });

    // Fetch role
    let role_resource = use_hackathon_role(slug_for_role)?;

    // Read resources
    let hackathon_result = hackathon_resource.read();
    let role_result = role_resource.read();

    // Check if resources are loaded
    match (hackathon_result.as_ref(), role_result.as_ref()) {
        (Some(Ok(Some(hackathon))), Some(Ok(role_opt))) => {
            // Both loaded successfully
            let role = role_opt.as_ref();

            // Provide context for child pages as a signal so it can be updated
            let hackathon_signal = use_context_provider(|| Signal::new(hackathon.clone()));
            use_context_provider(|| role.cloned());

            rsx! {
                div { class: "flex flex-row p-7 gap-9 h-screen",
                    Sidebar { slug, hackathon_signal, role: role.cloned() }
                    main { class: "flex-1 overflow-y-auto", Outlet::<Route> {} }
                }
            }
        }
        (Some(Ok(None)), _) | (Some(Err(_)), _) => {
            // Hackathon not found or error, navigate to 404
            use_effect(move || {
                nav.push(Route::NotFound {
                    route: vec!["h".to_string(), slug.clone()],
                });
            });

            rsx! {
                div { class: "flex items-center justify-center h-screen",
                    p { class: "text-foreground-neutral-primary", "Redirecting..." }
                }
            }
        }
        _ => {
            // Loading state
            rsx! {
                div { class: "flex items-center justify-center h-screen",
                    p { class: "text-foreground-neutral-primary", "Loading..." }
                }
            }
        }
    }
}
