use dioxus::prelude::*;
use dioxus_free_icons::{Icon, icons::ld_icons::LdSearch};

use super::{Button, ButtonVariant, ModalBase};
use crate::hackathons::handlers::teams::{
    UserWithoutTeam, get_users_without_team, send_invitation, SendInvitationRequest,
};

#[component]
pub fn InviteMembersModal(on_close: EventHandler<()>, slug: String) -> Element {
    let mut search_query = use_signal(|| String::new());
    let mut selected_user = use_signal(|| None::<UserWithoutTeam>);
    let mut is_sending = use_signal(|| false);
    let mut error_message = use_signal(|| None::<String>);

    // Fetch users without team
    let mut users_resource = use_resource(move || {
        let slug = slug.clone();
        let search = search_query();
        async move {
            let result: Result<Vec<UserWithoutTeam>, _> = get_users_without_team(
                slug,
                if search.is_empty() {
                    None
                } else {
                    Some(search)
                },
            )
            .await;
            result.ok()
        }
    });

    let on_send_invitation = move |user: UserWithoutTeam| {
        spawn({
            let slug = slug.clone();
            async move {
                is_sending.set(true);
                error_message.set(None);

                let req = SendInvitationRequest {
                    user_id: user.id,
                    message: None,
                };

                match send_invitation(slug, req).await {
                    Ok(_) => {
                        on_close.call(());
                    }
                    Err(e) => {
                        error_message.set(Some(e.to_string()));
                        is_sending.set(false);
                    }
                }
            }
        });
    };

    rsx! {
        ModalBase {
            on_close,
            width: "600px",
            max_height: "80vh",

            div { class: "p-7 flex flex-col h-full",
                // Header
                div { class: "mb-6",
                    h2 { class: "text-2xl font-semibold leading-8 text-foreground-neutral-primary",
                        "Invite Members"
                    }
                    p { class: "text-sm text-foreground-neutral-secondary mt-1",
                        "Send invitations to users without a team"
                    }
                }

                // Error message
                if let Some(error) = error_message() {
                    div { class: "mb-4 p-3 bg-status-danger-background text-status-danger-foreground rounded-lg text-sm",
                        "{error}"
                    }
                }

                // Search bar
                div { class: "mb-4",
                    div { class: "w-full h-10 border border-stroke-neutral-1 rounded-full flex items-center px-3 py-1",
                        Icon {
                            width: 20,
                            height: 20,
                            icon: LdSearch,
                            class: "text-foreground-neutral-tertiary",
                        }
                        input {
                            class: "flex-1 px-2.5 text-sm leading-5 text-foreground-neutral-tertiary outline-none bg-transparent",
                            placeholder: "Search by name or email",
                            r#type: "text",
                            value: "{search_query}",
                            oninput: move |e| {
                                search_query.set(e.value());
                                users_resource.restart();
                            },
                        }
                    }
                }

                // Users list
                div { class: "flex-1 overflow-y-auto",
                    match users_resource.read().as_ref() {
                        Some(Some(users)) => rsx! {
                            if users.is_empty() {
                                div { class: "flex items-center justify-center h-full",
                                    p { class: "text-foreground-neutral-secondary", "No users found" }
                                }
                            } else {
                                div { class: "flex flex-col gap-3",
                                    for user in users {
                                        div {
                                            key: "{user.id}",
                                            class: "flex items-center justify-between p-4 bg-background-neutral-secondary-enabled rounded-lg hover:bg-background-neutral-subtle cursor-pointer",
                                            onclick: move |_| selected_user.set(Some(user.clone())),
                                            div { class: "flex items-center gap-3",
                                                if let Some(picture) = &user.picture {
                                                    img {
                                                        src: "{picture}",
                                                        class: "w-10 h-10 rounded-full object-cover",
                                                    }
                                                } else {
                                                    div { class: "w-10 h-10 rounded-full bg-background-brand-subtle flex items-center justify-center text-foreground-brand-primary font-semibold text-sm",
                                                        {user.name.as_ref().and_then(|n| n.chars().next()).unwrap_or('U').to_string()}
                                                    }
                                                }
                                                div { class: "flex flex-col",
                                                    p { class: "text-sm font-semibold text-foreground-neutral-primary",
                                                        {user.name.clone().unwrap_or_else(|| "Unknown".to_string())}
                                                    }
                                                    p { class: "text-xs text-foreground-neutral-secondary",
                                                        "{user.email}"
                                                    }
                                                }
                                            }
                                            Button {
                                                variant: ButtonVariant::Primary,
                                                disabled: is_sending(),
                                                onclick: move |_| {
                                                    on_send_invitation(user.clone());
                                                },
                                                if is_sending() && selected_user().as_ref().map(|u| u.id) == Some(user.id) {
                                                    "Sending..."
                                                } else {
                                                    "Invite"
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        },
                        Some(None) => rsx! {
                            div { class: "flex items-center justify-center h-full",
                                p { class: "text-foreground-neutral-secondary", "Error loading users" }
                            }
                        },
                        None => rsx! {
                            div { class: "flex items-center justify-center h-full",
                                p { class: "text-foreground-neutral-secondary", "Loading users..." }
                            }
                        },
                    }
                }
            }
        }
    }
}
