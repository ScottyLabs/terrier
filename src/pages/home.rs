use crate::backend;
use dioxus::prelude::*;
use chrono::Local;
use dioxus::fullstack::{Form, SetCookie, SetHeader};
use dioxus::events::FormData;

#[cfg(feature = "server")]
use {
    dioxus::fullstack::{Cookie, TypedHeader},
    std::sync::LazyLock,
};

#[component]
pub fn Home() -> Element {
    let mut hackathons = use_resource(|| async move {
        backend::list_public_hackathons().await
    });

    let mut status = use_resource(|| async move {
        backend::user_status().await
    });

    let mut is_dialog_open = use_signal(|| false);
    let mut is_loading = use_signal(|| false);
    let mut hackathon_error = use_signal(|| None::<String>);

    let mut form_values = use_signal(|| Vec::<(String, String)>::new());

    // Initialize default date/time values
    let today = Local::now().format("%Y-%m-%d").to_string();
    let current_time = Local::now().format("%H:%M").to_string();

    let mut add_hackathon = use_action(backend::create_hackathon);

    rsx! {
        div { class: "min-h-screen bg-secondary text-selected flex flex-col",
            // Header
            div { class: "m-7 mr-auto",
                a { href: "/", class: "gap-2 flex",
                    // ScottyLabsFilled icon component would go here
                    span { class: "text-2xl font-medium", "Terrier" }
                }
            }

            main { class: "mx-7",
                // Show "New Hackathon" button only for admins
                if let Some(Ok(user_status)) = status.read().as_ref() {
                    if user_status.is_admin {
                        button {
                            class: "bg-gray-300 text-primary cursor-pointer font-semibold px-5 py-3.5 flex gap-2 rounded-4xl mb-6",
                            onclick: move |_| is_dialog_open.set(true),
                            // PlusIcon would go here
                            span { "New Hackathon" }
                        }
                    }
                }

                // Hackathons grid
                div { class: "w-full flex flex-col gap-5",
                    match hackathons.read().as_ref() {
                        Some(Ok(list)) => rsx! {
                            if list.is_empty() {
                                p { class: "text-center", "No hackathons found." }
                            } else {
                                div { class: "grid gap-4 [grid-template-columns:repeat(auto-fill,minmax(16rem,16rem))]",
                                    for hackathon in list.iter() {
                                        a {
                                            href: "/h/{hackathon.slug}/dashboard",
                                            class: "size-64 p-6 bg-primary rounded-lg shadow-sm hover:shadow-md duration-250 transition-shadow",
                                            h2 { class: "text-2xl font-bold", "{hackathon.name}" }
                                            p { class: "text-gray-600",
                                                {hackathon.description.as_deref().unwrap_or("No description")}
                                            }
                                        }
                                    }
                                }
                            }
                        },
                        Some(Err(_)) => rsx! {
                            p { class: "text-center text-red-600", "Error loading hackathons." }
                        },
                        None => rsx! {
                            p { class: "text-center", "Loading hackathons..." }
                        }
                    }
                }
            }

            // Dialog/Modal
            if is_dialog_open() {
                div { class: "fixed inset-0 z-50 flex items-center justify-center",
                    // Overlay
                    div {
                        class: "fixed inset-0 bg-black/80",
                        onclick: move |_| is_dialog_open.set(false)
                    }

                    // Dialog content
                    div { class: "rounded-[1.75rem] bg-white fixed z-50 w-full max-w-[calc(100%-2rem)] p-7 sm:max-w-[490px]",
                        // Close button
                        button {
                            class: "absolute right-5 top-5 cursor-pointer p-2 rounded-full hover:bg-gray-200",
                            onclick: move |_| is_dialog_open.set(false),
                            "×"
                            span { class: "sr-only", "Close" }
                        }

                        h2 { class: "text-2xl", "Create new hackathon" }

                        form {
                            class: "my-7 flex flex-col gap-5",
                            onsubmit: move |ev: FormEvent| async move {
                                ev.prevent_default();
                                let data = ev.data();
                                let mut pairs: Vec<(String, String)> = Vec::new();
                                for (name, value) in data.values() {
                                    if let FormValue::Text(text) = value {
                                        pairs.push((name.clone(), text.clone()));
                                    }
                                }
                                let values = backend::CreateHackathonForm::from_vectors(
                                    pairs.iter().map(|(k, _)| k.clone()).collect(),
                                    pairs.iter().map(|(_, v)| v.clone()).collect(),
                                ).unwrap();
                                println!("Form values: {:#?}", ev.values());
                                add_hackathon.call(Form(values)).await;
                            },

                            // Name field
                            div { class: "flex flex-col gap-2",
                                label { class: "text-label text-sm font-medium", r#for: "name", "Name" }
                                input {
                                    id: "name",
                                    name: "name",
                                    r#type: "text",
                                    class: "text-input h-10 bg-primary rounded-lg px-4 py-2",
                                    placeholder: "Name",
                                }
                            }

                            // Slug field
                            div { class: "flex flex-col gap-2",
                                label { class: "text-label text-sm font-medium", r#for: "slug", "Slug" }
                                input {
                                    id: "slug",
                                    name: "slug",
                                    r#type: "text",
                                    class: "h-10 bg-primary rounded-lg px-4 py-2",
                                    placeholder: "name",
                                }
                            }

                            // Description field
                            div { class: "flex flex-col gap-2",
                                label { class: "text-label text-sm font-medium", r#for: "description", "Description" }
                                textarea {
                                    id: "description",
                                    name: "description",
                                    class: "text-input h-20 bg-primary rounded-lg px-4 py-2 resize-none",
                                    placeholder: "An amazing hackathon..."
                                }
                            }

                            // Start date/time
                            div { class: "flex flex-col gap-2",
                                label { class: "text-label text-sm font-medium", "Start Date & Time" }
                                div { class: "flex gap-2",
                                    input {
                                        name: "start_date",
                                        r#type: "date",
                                        class: "text-input h-10 bg-primary rounded-lg px-4 py-2 flex-1",
                                        value: "{today}"
                                    }
                                    input {
                                        name: "start_time",
                                        r#type: "time",
                                        class: "text-input h-10 bg-primary rounded-lg px-4 py-2",
                                        value: "{current_time}"
                                    }
                                }
                            }

                            // End date/time
                            div { class: "flex flex-col gap-2",
                                label { class: "text-label text-sm font-medium", "End Date & Time" }
                                div { class: "flex gap-2",
                                    input {
                                        name: "end_date",
                                        r#type: "date",
                                        class: "text-input h-10 bg-primary rounded-lg px-4 py-2 flex-1",
                                        value: "{today}"
                                    }
                                    input {
                                        name: "end_time",
                                        r#type: "time",
                                        class: "text-input h-10 bg-primary rounded-lg px-4 py-2",
                                        value: "{current_time}"
                                    }
                                }
                            }

                            if let Some(error) = hackathon_error() {
                                p { class: "text-red-600 text-sm", "{error}" }
                            }

                            div { class: "flex justify-end gap-3",
                                button {
                                    r#type: "button",
                                    class: "cursor-pointer font-semibold px-5 py-3.5 rounded-4xl hover:bg-gray-200",
                                    onclick: move |_| is_dialog_open.set(false),
                                    "Cancel"
                                }

                                input {
                                    r#type: "submit",
                                    class: "bg-selected text-primary cursor-pointer font-semibold px-5 py-3.5 rounded-4xl",
                                    disabled: is_loading(),
                                    "Create"
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}

// Helper function
fn name_to_slug(name: &str) -> String {
    name.trim()
        .to_lowercase()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join("-")
}
