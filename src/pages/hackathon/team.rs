use crate::auth::{TEAM_ROLES, hooks::use_require_access_or_redirect};
use crate::components::{Input, InputVariant};
use crate::hackathons::HackathonInfo;
use crate::hackathons::handlers::teams::{
    JoinTeamRequest, TeamListItem, UpdateTeamRequest, get_all_teams, get_my_team, join_team,
    leave_team, update_team,
};
use dioxus::{logger::tracing, prelude::*};
use dioxus_free_icons::{
    Icon,
    icons::ld_icons::{LdSearch, LdUsers},
};

#[component]
pub fn HackathonTeam(slug: String) -> Element {
    if let Some(no_access) = use_require_access_or_redirect(TEAM_ROLES) {
        return no_access;
    }

    let _hackathon = use_context::<Signal<HackathonInfo>>();
    let mut search_query = use_signal(|| String::new());
    let mut is_editing = use_signal(|| false);
    let mut edit_name = use_signal(|| String::new());
    let mut edit_description = use_signal(|| String::new());
    let mut error_message = use_signal(|| None::<String>);
    let mut success_message = use_signal(|| None::<String>);

    let slug_for_team = slug.clone();
    let mut my_team = use_resource(move || {
        let slug = slug_for_team.clone();
        async move {
            match get_my_team(slug).await {
                Ok(team_opt) => team_opt,
                Err(e) => {
                    tracing::error!("Error fetching team: {:?}", e);
                    None
                }
            }
        }
    });

    let slug_for_all_teams = slug.clone();
    let mut all_teams = use_resource(move || {
        let slug = slug_for_all_teams.clone();
        let search = search_query();
        async move {
            match get_all_teams(
                slug,
                if search.is_empty() {
                    None
                } else {
                    Some(search)
                },
            )
            .await
            {
                Ok(teams) => Some(teams),
                Err(e) => {
                    tracing::error!("Error fetching teams: {:?}", e);
                    None
                }
            }
        }
    });

    let slug_clone1 = slug.clone();
    let slug_clone2 = slug.clone();
    let slug_clone3 = slug.clone();

    rsx! {
        div { class: "flex flex-col gap-14 pt-[60px]",
            // My Team Section
            div { class: "flex flex-col gap-7",
                div { class: "flex justify-between items-center",
                    h1 { class: "text-[30px] font-semibold leading-[38px] text-foreground-neutral-primary",
                        "My Team"
                    }
                    if let Some(Some(team)) = &*my_team.read() {
                        if team.member_count > 1 {
                            button {
                                class: "border border-stroke-neutral-1 rounded-full px-4 py-2.5 flex gap-2 items-center hover:bg-background-neutral-secondary-enabled cursor-pointer",
                                onclick: move |_| {
                                    let slug = slug_clone2.clone();
                                    spawn(async move {
                                        match leave_team(slug).await {
                                            Ok(_) => {
                                                success_message.set(Some("Left team successfully".to_string()));
                                                my_team.restart();
                                            }
                                            Err(e) => {
                                                error_message.set(Some(format!("Failed to leave team: {}", e)));
                                            }
                                        }
                                    });
                                },
                                Icon {
                                    width: 20,
                                    height: 20,
                                    icon: dioxus_free_icons::icons::ld_icons::LdLogOut,
                                    class: "text-foreground-neutral-primary",
                                }
                                span { class: "font-semibold text-sm leading-5 text-foreground-neutral-primary",
                                    "Leave Team"
                                }
                            }
                        }
                    }
                }

                if let Some(error) = error_message() {
                    div { class: "p-4 bg-background-neutral-secondary-enabled text-status-danger-foreground rounded-lg border border-status-danger-foreground",
                        "{error}"
                    }
                }

                if let Some(success) = success_message() {
                    div { class: "p-4 bg-background-neutral-secondary-enabled text-status-success-foreground rounded-lg border border-status-success-foreground",
                        "{success}"
                    }
                }

                div { class: "bg-background-neutral-primary rounded-[20px] p-9 relative",
                    match &*my_team.read() {
                        Some(Some(team)) => rsx! {
                            if is_editing() {
                                // Edit mode
                                button {
                                    class: "absolute top-6 right-6 border border-stroke-neutral-1 rounded-full px-4 py-2 flex gap-1.5 items-center hover:bg-background-neutral-secondary-enabled cursor-pointer",
                                    onclick: move |_| {
                                        is_editing.set(false);
                                        error_message.set(None);
                                    },
                                    Icon {
                                        width: 16,
                                        height: 16,
                                        icon: dioxus_free_icons::icons::ld_icons::LdX,
                                        class: "text-foreground-neutral-primary",
                                    }
                                    span { class: "font-medium text-xs leading-4 text-foreground-neutral-primary",
                                        "Cancel"
                                    }
                                }
                                form {
                                    class: "flex flex-col gap-12",
                                    onsubmit: move |_| {
                                        let slug = slug_clone1.clone();
                                        let name = edit_name();
                                        let description = edit_description();
                                        spawn(async move {
                                            let desc = if description.is_empty() { None } else { Some(description) };
                                            match update_team(
                                                    slug,
                                                    UpdateTeamRequest {
                                                        name,
                                                        description: desc,
                                                    },
                                                )
                                                .await
                                            {
                                                Ok(_) => {
                                                    success_message.set(Some("Team updated successfully".to_string()));
                                                    is_editing.set(false);
                                                    my_team.restart();
                                                }
                                                Err(e) => {
                                                    error_message.set(Some(format!("Failed to update team: {}", e)));
                                                }
                                            }
                                        });
                                    },
                                    Input {
                                        label: "Team Name".to_string(),
                                        value: edit_name,
                                        variant: InputVariant::Secondary,
                                    }
                                    Input {
                                        label: "Description".to_string(),
                                        value: edit_description,
                                        variant: InputVariant::Secondary,
                                    }
                                }
                            } else {
                                // View mode
                                if team.is_owner {
                                    {
                                        let team_name = team.name.clone();
                                        let team_desc = team.description.clone().unwrap_or_default();
                                        rsx! {
                                            button {
                                                class: "absolute top-6 right-6 border border-stroke-neutral-1 rounded-full px-4 py-2 flex gap-1.5 items-center hover:bg-background-neutral-secondary-enabled cursor-pointer",
                                                onclick: move |_| {
                                                    edit_name.set(team_name.clone());
                                                    edit_description.set(team_desc.clone());
                                                    is_editing.set(true);
                                                },
                                                Icon {
                                                    width: 16,
                                                    height: 16,
                                                    icon: dioxus_free_icons::icons::ld_icons::LdPencil,
                                                    class: "text-foreground-neutral-primary",
                                                }
                                                span { class: "font-medium text-xs leading-4 text-foreground-neutral-primary", "Edit" }
                                            }
                                        }
                                    }
                                }
                                div { class: "flex flex-col gap-12",
                                    div { class: "flex flex-col gap-2",
                                        p { class: "text-base font-medium text-foreground-neutral-secondary",
                                            "Team Name"
                                        }
                                        p { class: "text-[24px] font-medium text-foreground-neutral-primary",
                                            "{team.name}"
                                        }
                                    }

                                    if let Some(desc) = &team.description {
                                        div { class: "flex flex-col gap-2",
                                            p { class: "text-base font-medium text-foreground-neutral-secondary",
                                                "Description"
                                            }
                                            p { class: "text-sm text-foreground-neutral-primary", "{desc}" }
                                        }
                                    }

                                    div { class: "flex flex-col gap-4",
                                        p { class: "text-xl font-medium text-foreground-neutral-primary", "Members" }
                                        div { class: "flex flex-col gap-3",
                                            for member in &team.members {
                                                div { class: "flex items-center gap-4",
                                                    if let Some(picture) = &member.picture {
                                                        img {
                                                            src: "{picture}",
                                                            class: "w-8 h-8 rounded-full",
                                                            alt: "Member avatar",
                                                        }
                                                    } else {
                                                        div { class: "w-8 h-8 rounded-full bg-background-neutral-secondary-enabled flex items-center justify-center",
                                                            Icon {
                                                                width: 16,
                                                                height: 16,
                                                                icon: LdUsers,
                                                                class: "text-foreground-neutral-tertiary",
                                                            }
                                                        }
                                                    }
                                                    p { class: "text-sm font-semibold text-foreground-neutral-primary",
                                                        {member.name.as_ref().unwrap_or(&member.email).as_str()}
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        },
                        Some(None) => rsx! {
                            div { class: "text-center text-foreground-neutral-tertiary",
                                "You are not in a team yet. Join a team below!"
                            }
                        },
                        None => rsx! {
                            div { class: "text-center text-foreground-neutral-tertiary", "Loading your team..." }
                        },
                    }
                }
            }

            // All Teams Section
            div { class: "flex flex-col gap-7",
                h1 { class: "text-[30px] font-semibold leading-[38px] text-foreground-neutral-primary",
                    "All Teams"
                }

                div { class: "flex items-center gap-2",
                    div { class: "w-[405px] h-10 border border-stroke-neutral-1 rounded-full flex items-center px-3 py-1",
                        Icon {
                            width: 20,
                            height: 20,
                            icon: LdSearch,
                            class: "text-foreground-neutral-tertiary",
                        }
                        input {
                            class: "flex-1 px-2.5 text-sm leading-5 text-foreground-neutral-tertiary outline-none bg-transparent",
                            r#type: "text",
                            placeholder: "Search teams...",
                            value: "{search_query}",
                            oninput: move |evt| {
                                search_query.set(evt.value());
                                all_teams.restart();
                            },
                        }
                    }
                }

                div { class: "bg-background-neutral-primary rounded-[20px] p-7",
                    match &*all_teams.read() {
                        Some(Some(teams)) => rsx! {
                            if teams.is_empty() {
                                div { class: "text-center text-foreground-neutral-tertiary", "No teams found" }
                            } else {
                                div { class: "divide-y divide-stroke-neutral-1",
                                    for team_item in teams {
                                        TeamListItemComponent {
                                            key: "{team_item.id}",
                                            team: team_item.clone(),
                                            slug: slug_clone3.clone(),
                                        }
                                    }
                                }
                            }
                        },
                        Some(None) => rsx! {
                            div { class: "text-center text-foreground-neutral-tertiary", "Error loading teams" }
                        },
                        None => rsx! {
                            div { class: "text-center text-foreground-neutral-tertiary", "Loading teams..." }
                        },
                    }
                }
            }
        }
    }
}

#[component]
fn TeamListItemComponent(team: TeamListItem, slug: String) -> Element {
    let error_message = use_signal(|| None::<String>);
    let is_loading = use_signal(|| false);
    let join_success = use_signal(|| false);

    let can_join = !team.is_full;

    rsx! {
        div { class: "py-3",
            div { class: "flex items-center justify-between",
                div { class: "flex flex-col gap-2.5 flex-1",
                    p { class: "text-base font-medium text-foreground-neutral-primary",
                        "{team.name}"
                    }
                    if let Some(desc) = &team.description {
                        p { class: "text-xs text-foreground-neutral-primary", "{desc}" }
                    }
                }

                div { class: "flex items-center gap-3",
                    p { class: "text-xs font-medium text-foreground-neutral-primary px-4",
                        "{team.member_count}/{team.max_size} Members"
                    }

                    if let Some(error) = error_message() {
                        div { class: "text-sm text-status-danger-foreground", "{error}" }
                    }

                    div { class: "flex gap-3",
                        button { class: "bg-background-brandneutral-secondary-enabled text-foreground-brandneutral-primary font-semibold text-sm leading-5 rounded-full px-4 py-2.5 hover:bg-background-brandneutral-secondary-hover cursor-pointer",
                            "Details"
                        }
                    }
                }
            }
        }
    }
}
