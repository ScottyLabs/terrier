mod auth;
mod config;
mod core;
#[cfg(feature = "server")]
mod docs;
mod domain;
#[cfg(feature = "server")]
mod entities;
#[cfg(feature = "server")]
mod server;
mod ui;

use dioxus::prelude::*;
use ui::pages::*;

#[cfg(feature = "server")]
use config::Config;
#[cfg(feature = "server")]
use dioxus_fullstack::FullstackContext;
#[cfg(feature = "server")]
use dioxus_fullstack::extract::FromRef;

#[cfg(feature = "server")]
#[derive(Clone)]
pub struct AppState {
    pub config: Config,
    pub db: sea_orm::DatabaseConnection,
    pub s3: minio::s3::client::Client,
}

#[cfg(feature = "server")]
impl FromRef<FullstackContext> for AppState {
    fn from_ref(state: &FullstackContext) -> Self {
        state.extension::<AppState>().unwrap()
    }
}

#[derive(Debug, Clone, Routable, PartialEq)]
#[rustfmt::skip]
pub enum Route {
    #[route("/")]
    Home {},
    #[route("/h/new")]
    CreateHackathon {},
    #[nest("/h/:slug")]
        #[layout(HackathonLayout)]
            #[route("/")]
                HackathonDashboard {
                    slug: String
                },
            #[route("/applicants")]
                HackathonApplicants {
                    slug: String
                },
            #[route("/people")]
                HackathonPeople {
                    slug: String
                },
            #[route("/team")]
                HackathonTeam {
                    slug: String
                },
            #[route("/schedule")]
                HackathonSchedule {
                    slug: String
                },
            #[route("/messages")]
                HackathonMessages {
                    slug: String
                },
            #[route("/submission")]
                HackathonSubmission {
                    slug: String
                },
            #[route("/checkin")]
                HackathonCheckin {
                    slug: String
                },
            #[route("/profile")]
                HackathonProfile {
                    slug: String
                },
            #[route("/settings")]
                HackathonSettings {
                    slug: String
                },
            #[route("/apply")]
                HackathonApply {
                    slug: String
                },
        #[end_layout]
    #[end_nest]
    #[route("/:..route")]
    NotFound {
        route: Vec<String>
    },
}

const TAILWIND_CSS: Asset = asset!("/assets/tailwind.css");

fn main() {
    #[cfg(feature = "server")]
    {
        tokio::runtime::Runtime::new()
            .unwrap()
            .block_on(async move {
                server::setup().await;
            });
    }

    #[cfg(not(feature = "server"))]
    {
        dioxus_logger::init(dioxus_logger::tracing::Level::DEBUG).expect("failed to init logger");
        dioxus::launch(App);
    }
}

#[component]
fn App() -> Element {
    let user_future = use_server_future(domain::auth::handlers::get_current_user)?;
    let user = use_signal(|| user_future().and_then(|r| r.ok()).flatten());
    use_context_provider(|| user);

    let mut is_mobile = use_signal(|| false);
    use_context_provider(|| is_mobile);

    // Update is_mobile on client-side after hydration
    #[cfg(target_arch = "wasm32")]
    use_effect(move || {
        use wasm_bindgen::{JsCast, closure::Closure};

        let check_mobile = || {
            web_sys::window()
                .and_then(|w| w.inner_width().ok())
                .and_then(|w| w.as_f64())
                .map(|w| w < 768.0)
                .unwrap_or(false)
        };

        // Set initial value
        is_mobile.set(check_mobile());

        // Set up resize listener
        if let Some(window) = web_sys::window() {
            let closure = Closure::<dyn FnMut()>::new(move || {
                is_mobile.set(check_mobile());
            });

            let _ =
                window.add_event_listener_with_callback("resize", closure.as_ref().unchecked_ref());

            // Keep closure alive - it will be dropped when component unmounts
            closure.forget();
        }
    });

    rsx! {
        document::Link { rel: "stylesheet", href: TAILWIND_CSS }
        div { class: "font-sans text-primary bg-background-neutral-secondary-enabled min-h-screen",
            Router::<Route> {}
        }
    }
}
