use dioxus::{core::Element, prelude::*};

use crate::{
    Route,
    auth::UserInfo,
    domain::hackathons,
    ui::foundation::components::{Button, Header, HeaderSize},
};

#[component]
pub fn Home() -> Element {
    #[cfg(feature = "mobile")]
    {
        if let Some(slug) = crate::config::DEFAULT_HACKATHON_SLUG {
            let nav = navigator();
            use_effect(move || {
                nav.push(Route::HackathonDashboard {
                    slug: slug.to_string(),
                });
            });
            return rsx!();
        }
    }

    let user = use_context::<Signal<Option<UserInfo>>>();
    let is_admin = user().map(|u| u.is_admin).unwrap_or(false);

    let hackathons_future = use_server_future(hackathons::handlers::query::get_hackathons)?;
    let hackathons_list = hackathons_future().and_then(|r| r.ok()).unwrap_or_default();

    rsx!(
        main { class: "p-7",
            header { class: "flex justify-between mb-12",
                Header { size: HeaderSize::Large }
                if is_admin {
                    Link { to: Route::CreateHackathon {},
                        Button { "New Hackathon" }
                    }
                }
            }

            if hackathons_list.is_empty() {
                p { class: "text-foreground-neutral-primary", "No hackathons available." }
            } else {
                div { class: "flex flex-wrap gap-4",
                    for hackathon in hackathons_list {
                        Link {
                            key: "{hackathon.id}",
                            to: Route::HackathonDashboard {
                                slug: hackathon.slug.clone(),
                            },
                            class: "group cursor-pointer",
                            div { class: "relative w-80 aspect-1/2 rounded-lg overflow-hidden bg-background-neutral-secondary",
                                if let Some(banner_url) = &hackathon.banner_url {
                                    img {
                                        src: "{banner_url}",
                                        alt: "{hackathon.name}",
                                        class: "w-full h-full object-cover group-hover:scale-105 transition-transform duration-200",
                                    }
                                }
                                div { class: "absolute inset-0 bg-linear-to-t from-black/60 to-transparent" }
                                div { class: "absolute bottom-0 left-0 right-0 p-4",
                                    h3 { class: "text-white font-bold text-xl mb-1",
                                        "{hackathon.name}"
                                    }
                                    if let Some(desc) = &hackathon.description {
                                        p { class: "text-white/80 text-sm line-clamp-2",
                                            "{desc}"
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    )
}
