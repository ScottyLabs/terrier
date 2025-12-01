use dioxus::prelude::*;

use crate::Route;

#[component]
pub fn NotFound(route: Vec<String>) -> Element {
    rsx! {
        div { class: "flex flex-col items-center justify-center h-screen bg-background-neutral-secondary-enabled",
            h1 { class: "text-4xl font-bold text-foreground-neutral-primary mb-4",
                "404 - Page Not Found"
            }
            p { class: "text-foreground-neutral-secondary mb-8",
                "The page you're looking for doesn't exist."
            }
            Link {
                to: Route::Home {},
                class: "px-4 py-2 bg-foreground-neutral-primary text-white rounded-lg hover:opacity-90",
                "Go Home"
            }
        }
    }
}
