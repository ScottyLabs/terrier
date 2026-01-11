use dioxus::prelude::*;

use crate::{Route, auth::hooks::use_hackathon_role, ui::foundation::layout::Sidebar};

#[component]
pub fn HackathonLayout(slug: String) -> Element {
    let nav = navigator();
    let slug_for_hackathon = slug.clone();
    let slug_for_role = slug.clone();
    let is_mobile = use_context::<Signal<bool>>();
    // Fetch hackathon data
    let hackathon_resource = use_resource(move || {
        let s = slug_for_hackathon.clone();
        async move { crate::domain::hackathons::handlers::query::get_hackathon_by_slug(s).await }
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
            // IMPORTANT: These must be provided BEFORE any conditional returns
            // so that child pages can always access them
            let hackathon_signal = use_context_provider(|| Signal::new(hackathon.clone()));
            use_context_provider(|| role.cloned());

            // Provide a refresh trigger for application status
            let application_refresh_trigger = use_context_provider(|| Signal::new(0u32));

            // Redirect applicants to Apply page if they're on the dashboard
            let current_route = use_route::<Route>();
            let should_redirect = matches!(current_route, Route::HackathonDashboard { .. })
                && role.map(|r| r.role == "applicant").unwrap_or(false);

            let slug_for_redirect = slug.clone();
            use_effect(move || {
                if should_redirect {
                    nav.push(Route::HackathonApply {
                        slug: slug_for_redirect.clone(),
                    });
                }
            });

            rsx! {
                div {
                    class: "flex bg-cover bg-center bg-no-repeat w-screen h-screen flex-col md:flex-row md:h-screen md:gap-9 md:p-7",
                    style: if let Some(bg_url) = &hackathon.background_url { format!("background-image: url('{}')", bg_url) } else { String::new() },
                    Sidebar {
                        slug,
                        hackathon_signal,
                        role: role.cloned(),
                        application_refresh_trigger,
                    }
                    main { class: "flex-1 p-2 min-w-0 overflow-auto", Outlet::<Route> {} }
                }
            }
        }
        (Some(Ok(None)), _) => {
            // Hackathon not found, navigate to 404
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
        (Some(Err(_)), _) | (_, Some(Err(_))) => {
            // Error fetching hackathon or role - redirect to home
            use_effect(move || {
                nav.push(Route::Home {});
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
