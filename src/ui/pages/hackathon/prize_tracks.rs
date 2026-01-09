use dioxus::prelude::*;
use dioxus_free_icons::{Icon, icons::ld_icons::LdPlus};

use crate::{
    auth::{PRIZE_TRACKS_ROLES, hooks::use_require_access_or_redirect},
    domain::prizes::handlers::{
        CreatePrizeRequest, PrizeInfo, create_prize, delete_prize, get_prizes,
    },
    ui::{
        features::prizes::PrizeCard,
        foundation::{
            components::{Button, ButtonSize, ButtonVariant},
            modals::base::ModalBase,
        },
    },
};

#[component]
pub fn HackathonPrizeTracks(slug: String) -> Element {
    if let Some(no_access) = use_require_access_or_redirect(PRIZE_TRACKS_ROLES) {
        return no_access;
    }

    let mut show_create_modal = use_signal(|| false);
    let mut selected_prize = use_signal(|| None::<PrizeInfo>);

    // Form state
    let mut name = use_signal(String::new);
    let mut description = use_signal(String::new);
    let mut image_url = use_signal(String::new);
    let mut category = use_signal(String::new);
    let mut value = use_signal(String::new);

    // Fetch prizes
    let mut prizes_resource = use_resource({
        let slug = slug.clone();
        move || {
            let slug = slug.clone();
            async move { get_prizes(slug).await.ok() }
        }
    });

    let mut reset_form = move || {
        name.set(String::new());
        description.set(String::new());
        image_url.set(String::new());
        category.set(String::new());
        value.set(String::new());
    };

    rsx! {
        div { class: "flex flex-col h-full",
            // Header
            div { class: "flex flex-col md:flex-row justify-between md:items-center gap-3 pt-6 md:pt-11 pb-4 md:pb-7",
                h1 { class: "text-2xl md:text-[30px] font-semibold leading-8 md:leading-[38px] text-foreground-neutral-primary",
                    "Prize Tracks"
                }
                Button {
                    size: ButtonSize::Compact,
                    onclick: move |_| show_create_modal.set(true),
                    Icon {
                        width: 16,
                        height: 16,
                        icon: LdPlus,
                        class: "text-white mr-1 inline-block",
                    }
                    "Add Prize"
                }
            }

            // Prize grid
            div { class: "flex-1 overflow-y-auto",
                match prizes_resource.read().as_ref() {
                    Some(Some(prizes)) if !prizes.is_empty() => rsx! {
                        div { class: "grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-4",
                            for prize in prizes.iter() {
                                {
                                    let prize_clone = prize.clone();
                                    rsx! {
                                        PrizeCard {
                                            key: "{prize.id}",
                                            prize: prize.clone(),
                                            on_click: move |_| selected_prize.set(Some(prize_clone.clone())),
                                        }
                                    }
                                }
                            }
                        }
                    },
                    Some(Some(_)) => rsx! {
                        div { class: "bg-background-neutral-primary rounded-2xl p-6 text-center",
                            p { class: "text-foreground-neutral-secondary",
                                "No prizes configured yet. Click \"Add Prize\" to create one."
                            }
                        }
                    },
                    Some(None) => rsx! {
                        div { class: "bg-background-neutral-primary rounded-2xl p-6 text-center",
                            p { class: "text-status-danger-foreground", "Failed to load prizes." }
                        }
                    },
                    None => rsx! {
                        div { class: "bg-background-neutral-primary rounded-2xl p-6 text-center",
                            p { class: "text-foreground-neutral-secondary", "Loading prizes..." }
                        }
                    },
                }
            }
        }

        // Create prize modal
        if show_create_modal() {
            ModalBase {
                on_close: move |_| {
                    show_create_modal.set(false);
                    reset_form();
                },
                width: "500px",
                max_height: "90vh",

                div { class: "p-7",
                    h2 { class: "text-2xl font-semibold text-foreground-neutral-primary mb-6",
                        "Create New Prize"
                    }

                    form {
                        class: "flex flex-col gap-4",
                        onsubmit: {
                            let slug = slug.clone();
                            move |evt: FormEvent| {
                                evt.prevent_default();
                                let slug = slug.clone();
                                let request = CreatePrizeRequest {
                                    name: name(),
                                    description: if description().is_empty() {
                                        None
                                    } else {
                                        Some(description())
                                    },
                                    image_url: if image_url().is_empty() { None } else { Some(image_url()) },
                                    category: if category().is_empty() { None } else { Some(category()) },
                                    value: value(),
                                };
                                spawn(async move {
                                    if create_prize(slug, request).await.is_ok() {
                                        show_create_modal.set(false);
                                        reset_form();
                                        prizes_resource.restart();
                                    }
                                });
                            }
                        },

                        // Name field
                        div { class: "flex flex-col gap-2",
                            label { class: "text-sm font-medium text-foreground-neutral-primary",
                                "Name *"
                            }
                            input {
                                class: "px-4 h-12 bg-background-neutral-secondary text-foreground-neutral-primary text-sm font-normal rounded-[0.625rem] border border-border-neutral-primary",
                                r#type: "text",
                                placeholder: "Prize name",
                                required: true,
                                value: "{name}",
                                oninput: move |e| name.set(e.value()),
                            }
                        }

                        // Description field
                        div { class: "flex flex-col gap-2",
                            label { class: "text-sm font-medium text-foreground-neutral-primary",
                                "Description"
                            }
                            textarea {
                                class: "px-4 py-3 bg-background-neutral-secondary text-foreground-neutral-primary text-sm font-normal rounded-[0.625rem] border border-border-neutral-primary min-h-[100px]",
                                placeholder: "Prize description",
                                value: "{description}",
                                oninput: move |e| description.set(e.value()),
                            }
                        }

                        // Category field
                        div { class: "flex flex-col gap-2",
                            label { class: "text-sm font-medium text-foreground-neutral-primary",
                                "Category"
                            }
                            input {
                                class: "px-4 h-12 bg-background-neutral-secondary text-foreground-neutral-primary text-sm font-normal rounded-[0.625rem] border border-border-neutral-primary",
                                r#type: "text",
                                placeholder: "e.g., Grand Prize, Best Design, etc.",
                                value: "{category}",
                                oninput: move |e| category.set(e.value()),
                            }
                        }

                        // Value field
                        div { class: "flex flex-col gap-2",
                            label { class: "text-sm font-medium text-foreground-neutral-primary",
                                "Value *"
                            }
                            input {
                                class: "px-4 h-12 bg-background-neutral-secondary text-foreground-neutral-primary text-sm font-normal rounded-[0.625rem] border border-border-neutral-primary",
                                r#type: "text",
                                placeholder: "e.g., $1000, MacBook Pro, etc.",
                                required: true,
                                value: "{value}",
                                oninput: move |e| value.set(e.value()),
                            }
                        }

                        // Image URL field
                        div { class: "flex flex-col gap-2",
                            label { class: "text-sm font-medium text-foreground-neutral-primary",
                                "Image URL"
                            }
                            input {
                                class: "px-4 h-12 bg-background-neutral-secondary text-foreground-neutral-primary text-sm font-normal rounded-[0.625rem] border border-border-neutral-primary",
                                r#type: "url",
                                placeholder: "https://example.com/prize-image.jpg",
                                value: "{image_url}",
                                oninput: move |e| image_url.set(e.value()),
                            }
                        }

                        // Buttons
                        div { class: "flex gap-3 justify-end mt-4",
                            Button {
                                variant: ButtonVariant::Tertiary,
                                button_type: "button".to_string(),
                                onclick: move |_| {
                                    show_create_modal.set(false);
                                    reset_form();
                                },
                                "Cancel"
                            }
                            Button { button_type: "submit".to_string(), "Create Prize" }
                        }
                    }
                }
            }
        }

        // Prize detail modal
        if let Some(prize) = selected_prize() {
            {
                let prize_id = prize.id;
                rsx! {
                    ModalBase {
                        on_close: move |_| selected_prize.set(None),
                        width: "500px",
                        max_height: "90vh",

                    // Prize image if available








                        div { class: "p-7",
                            if let Some(img_url) = &prize.image_url {
                                div { class: "mb-4 rounded-lg overflow-hidden",
                                    img {
                                        src: "{img_url}",
                                        alt: "{prize.name}",
                                        class: "w-full h-48 object-cover",
                                    }
                                }
                            }

                            h2 { class: "text-2xl font-semibold text-foreground-neutral-primary mb-2",
                                "{prize.name}"
                            }

                            if let Some(cat) = &prize.category {
                                span { class: "inline-block px-3 py-1 bg-background-neutral-secondary text-foreground-neutral-secondary text-sm rounded-full mb-4",
                                    "{cat}"
                                }
                            }

                            div { class: "mb-4",
                                p { class: "text-lg font-medium text-foreground-brand-primary", "{prize.value}" }
                            }

                            if let Some(desc) = &prize.description {
                                p { class: "text-foreground-neutral-secondary mb-6", "{desc}" }
                            }

                            div { class: "flex gap-3 justify-end",
                                Button {
                                    variant: ButtonVariant::Default,
                                    onclick: move |_| selected_prize.set(None),
                                    "Close"
                                }
                                Button {
                                    variant: ButtonVariant::Danger,
                                    onclick: {
                                        let slug = slug.clone();
                                        move |_| {
                                            let slug = slug.clone();
                                            spawn(async move {
                                                if delete_prize(slug, prize_id).await.is_ok() {
                                                    selected_prize.set(None);
                                                    prizes_resource.restart();
                                                }
                                            });
                                        }
                                    },
                                    "Delete"
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}
