use dioxus::prelude::*;

use crate::domain::teams::handlers::get_team_details;
use crate::ui::foundation::components::{Button, ButtonVariant};
use crate::ui::foundation::modals::base::ModalBase;

#[component]
pub fn ViewTeamModal(
    on_close: EventHandler<()>,
    on_request_join: EventHandler<i32>,
    slug: String,
    team_id: i32,
    #[props(default = false)] user_has_team: bool,
) -> Element {
    let slug_clone = slug.clone();
    let team_data = use_resource(move || {
        let slug = slug_clone.clone();
        async move { get_team_details(slug, team_id).await }
    });

    rsx! {
        ModalBase {
            on_close,
            width: "600px",
            max_height: "80vh",

            div { class: "p-7",
                match &*team_data.read_unchecked() {
                    Some(Ok(team)) => rsx! {
                        // Header
                        div { class: "mb-6",
                            h2 { class: "text-2xl font-semibold leading-8 text-foreground-neutral-primary",
                                "{team.name}"
                            }
                            if let Some(description) = &team.description {
                                p { class: "text-sm text-foreground-neutral-secondary mt-2",
                                    "{description}"
                                }
                            }
                            div { class: "flex items-center gap-2 mt-3",
                                span { class: "text-sm text-foreground-neutral-secondary",
                                    "{team.member_count} / {team.max_size} members"
                                }
                                if team.member_count >= team.max_size as usize {
                                    span { class: "px-2 py-1 text-xs bg-status-danger-background text-status-danger-foreground rounded-md",
                                        "Full"
                                    }
                                }
                            }
                        }

                        // Members list
                        div { class: "mb-6",
                            h3 { class: "text-base font-semibold text-foreground-neutral-primary mb-3",
                                "Members"
                            }
                            div { class: "space-y-2",
                                for (index , member) in team.members.iter().enumerate() {
                                    div {
                                        key: "{member.user_id}",
                                        class: "flex items-center gap-3 p-3 bg-background-neutral-primary rounded-lg",
                                        if let Some(picture) = &member.picture {
                                            img {
                                                src: "{picture}",
                                                class: "w-10 h-10 rounded-full object-cover",
                                            }
                                        } else {
                                            div {
                                                class: "w-10 h-10 rounded-full bg-background-brand-subtle flex items-center justify-center text-foreground-brand-primary font-semibold",
                                                {
                                                    member
                                                        .name
                                                        .as_ref()
                                                        .and_then(|n| n.chars().next())
                                                        .unwrap_or('U')
                                                        .to_string()
                                                }
                                            }
                                        }
                                        div { class: "flex-1",
                                            div { class: "flex items-center gap-2",
                                                p { class: "text-sm font-medium text-foreground-neutral-primary",
                                                    {member.name.clone().unwrap_or_else(|| "Unknown".to_string())}
                                                }
                                                if index == 0 {
                                                    span { class: "px-2 py-0.5 text-xs bg-background-brand-subtle text-foreground-brand-primary rounded-md",
                                                        "Owner"
                                                    }
                                                }
                                            }
                                            p { class: "text-xs text-foreground-neutral-secondary",
                                                "{member.email}"
                                            }
                                        }
                                    }
                                }
                            }
                        }

                        // Action button (only show if user doesn't have a team and isn't a member)
                        if !team.is_member && !user_has_team {
                            {
                                let is_full = team.member_count >= team.max_size as usize;
                                rsx! {
                                    div { class: "flex justify-end pt-4",
                                        Button {
                                            variant: ButtonVariant::Default,
                                            onclick: move |_| on_request_join.call(team_id),
                                            disabled: is_full,
                                            if is_full {
                                                "Team Full"
                                            } else {
                                                "Request to Join"
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    },
                    Some(Err(e)) => rsx! {
                        div { class: "text-status-danger-foreground",
                            "Error loading team: {e}"
                        }
                    },
                    None => rsx! {
                        div { class: "text-foreground-neutral-secondary",
                            "Loading..."
                        }
                    }
                }
            }
        }
    }
}
