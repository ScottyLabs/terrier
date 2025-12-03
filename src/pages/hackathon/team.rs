use crate::auth::{TEAM_ROLES, hooks::use_require_access_or_redirect};
use crate::components::{
    Button, ButtonSize, ButtonVariant, ButtonWithIcon, CreateTeamModal, JoinRequestModal,
    TabSwitcher, ViewTeamModal,
};
use crate::hackathons::HackathonInfo;
use crate::hackathons::handlers::teams::{
    JoinRequestResponse, TeamListItem, accept_join_request, get_all_teams, get_join_requests,
    get_my_team, leave_team, reject_join_request,
};
use dioxus::{logger::tracing, prelude::*};
use dioxus_free_icons::{
    Icon,
    icons::ld_icons::{LdPlus, LdSearch},
};

#[derive(Clone, Copy, PartialEq)]
enum MyTeamTab {
    Current,
    Requests,
}

#[component]
pub fn HackathonTeam(slug: String) -> Element {
    if let Some(no_access) = use_require_access_or_redirect(TEAM_ROLES) {
        return no_access;
    }

    let _hackathon = use_context::<Signal<HackathonInfo>>();
    let mut search_query = use_signal(|| String::new());
    let active_tab = use_signal(|| MyTeamTab::Current);

    // Modal states
    let mut show_create_modal = use_signal(|| false);
    let mut show_view_modal = use_signal(|| None::<i32>);
    let mut show_join_request_modal = use_signal(|| None::<(i32, String)>);

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

    let slug_for_requests = slug.clone();
    let mut join_requests = use_resource(move || {
        let slug = slug_for_requests.clone();
        async move {
            // Only fetch if user has a team
            match get_join_requests(slug).await {
                Ok(requests) => Some(requests),
                Err(e) => {
                    tracing::error!("Error fetching join requests: {:?}", e);
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

    let has_team = my_team.read().as_ref().and_then(|t| t.as_ref()).is_some();

    let slug_clone = slug.clone();
    let slug_for_leave = slug.clone();

    rsx! {
        div { class: "flex flex-col gap-14 pt-[60px]",

            // My Team Section
            if has_team {
                div { class: "flex flex-col gap-7",
                    // Header with Leave Team button outside
                    div { class: "flex justify-between items-center",
                        h1 { class: "text-[30px] font-semibold leading-[38px] text-foreground-neutral-primary",
                            "My Team"
                        }
                        // Leave Team button
                        if let Some(Some(team)) = &*my_team.read() {
                            {
                                let can_leave = !team.is_owner || team.member_count == 1;
                                if can_leave { Some(rsx! {
                                    ButtonWithIcon {
                                        icon: dioxus_free_icons::icons::ld_icons::LdLogOut,
                                        size: ButtonSize::Compact,
                                        variant: ButtonVariant::Outline,
                                        onclick: move |_| {
                                            let slug = slug_for_leave.clone();
                                            spawn(async move {
                                                match leave_team(slug).await {
                                                    Ok(_) => {
                                                        let _ = dioxus::document::eval("alert('Left team successfully')");
                                                        my_team.restart();
                                                        all_teams.restart();
                                                    }
                                                    Err(e) => {
                                                        let error_msg = e.to_string();
                                                        let _ = dioxus::document::eval(
                                                            &format!("alert('{}')", error_msg.replace("'", "\\'")),
                                                        );
                                                    }
                                                }
                                            });
                                        },
                                        "Leave Team"
                                    }
                                }) } else { None }
                            }
                        }
                    }

                    // First card, team info with Edit button
                    div { class: "bg-background-neutral-primary rounded-[20px] p-9 relative",
                        if let Some(Some(team)) = &*my_team.read() {
                            div { class: "flex flex-col gap-12",
                                // Team Name section
                                div { class: "flex flex-col gap-2",
                                    p { class: "text-base font-medium text-foreground-neutral-secondary",
                                        "Team Name"
                                    }
                                    h2 { class: "text-2xl font-medium text-foreground-neutral-primary",
                                        "{team.name}"
                                    }
                                }

                                // Description section
                                div { class: "flex flex-col gap-2",
                                    p { class: "text-base font-medium text-foreground-neutral-secondary",
                                        "Description"
                                    }
                                    p { class: "text-sm text-foreground-neutral-primary",
                                        {team.description.clone().unwrap_or_else(|| "No description".to_string())}
                                    }
                                }

                                // Edit button
                                if team.is_owner {
                                    div { class: "absolute top-6 right-6",
                                        ButtonWithIcon {
                                            icon: dioxus_free_icons::icons::ld_icons::LdPencil,
                                            size: ButtonSize::Compact,
                                            variant: ButtonVariant::Outline,
                                            onclick: move |_| {
                                                let _ = dioxus::document::eval("alert('Edit team functionality coming soon!')");
                                            },
                                            "Edit"
                                        }
                                    }
                                }
                            }
                        }
                    }

                    // Second card, Members section
                    div { class: "bg-background-neutral-primary rounded-[20px] p-9 relative",
                        if let Some(Some(team)) = &*my_team.read() {
                            div { class: "flex flex-col gap-12",
                                // Members heading section
                                div { class: "flex flex-col gap-4",
                                    h2 { class: "text-xl font-medium text-foreground-neutral-primary",
                                        "Members"
                                    }

                                    // Tabs
                                    if team.is_owner {
                                        div {
                                            TabSwitcher {
                                                active_tab,
                                                tabs: vec![
                                                    (MyTeamTab::Current, "Current".to_string()),
                                                    (MyTeamTab::Requests, "Requests".to_string()),
                                                ],
                                            }
                                        }
                                    }

                                    // Tab content
                                    if active_tab() == MyTeamTab::Current {
                                        // Members list
                                        div { class: "flex flex-col gap-3 mt-5",
                                            for (index , member) in team.members.iter().enumerate() {
                                                div {
                                                    key: "{member.user_id}",
                                                    class: "flex items-center gap-4",
                                                    if let Some(picture) = &member.picture {
                                                        img {
                                                            src: "{picture}",
                                                            class: "w-8 h-8 rounded-full object-cover",
                                                        }
                                                    } else {
                                                        div { class: "w-8 h-8 rounded-full bg-background-brand-subtle flex items-center justify-center text-foreground-brand-primary font-semibold text-sm",
                                                            {member.name.as_ref().and_then(|n| n.chars().next()).unwrap_or('U').to_string()}
                                                        }
                                                    }
                                                    p { class: "text-sm font-semibold text-foreground-neutral-primary",
                                                        {member.name.clone().unwrap_or_else(|| "Unknown".to_string())}
                                                        if index == 0 {
                                                            span { class: "ml-2 px-2 py-0.5 text-xs bg-background-brand-subtle text-foreground-brand-primary rounded-md font-normal",
                                                                "Owner"
                                                            }
                                                        }
                                                    }
                                                }
                                            }
                                        }
                                    } else {
                                        // Requests list
                                        div { class: "mt-5",
                                            match &*join_requests.read() {
                                                Some(Some(requests)) => rsx! {
                                                    if requests.is_empty() {
                                                        div { class: "text-center py-4 text-foreground-neutral-secondary text-sm",
                                                            "No pending join requests"
                                                        }
                                                    } else {
                                                        div { class: "flex flex-col gap-3",
                                                            for request in requests {
                                                                JoinRequestCard {
                                                                    key: "{request.id}",
                                                                    request: request.clone(),
                                                                    slug: slug_clone.clone(),
                                                                    on_action: move |_| {
                                                                        join_requests.restart();
                                                                        my_team.restart();
                                                                    },
                                                                }
                                                            }
                                                        }
                                                    }
                                                },
                                                Some(None) => rsx! {
                                                    div { class: "text-center py-8 text-status-danger-foreground", "Error loading requests" }
                                                },
                                                None => rsx! {
                                                    div { class: "text-center py-8 text-foreground-neutral-secondary", "Loading requests..." }
                                                },
                                            }
                                        }
                                    }
                                }

                                // Invite Members button
                                if team.is_owner {
                                    div { class: "absolute top-7 right-6",
                                        ButtonWithIcon {
                                            icon: dioxus_free_icons::icons::ld_icons::LdUserPlus,
                                            size: ButtonSize::Compact,
                                            variant: ButtonVariant::Outline,
                                            onclick: move |_| {
                                                let _ = dioxus::document::eval(
                                                    "alert('Invite members functionality coming soon!')",
                                                );
                                            },
                                            "Invite Members"
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }

            // All Teams Section
            div { class: "flex flex-col gap-7",
                div { class: "flex justify-between items-center",
                    h1 { class: "text-[30px] font-semibold leading-[38px] text-foreground-neutral-primary",
                        "All Teams"
                    }
                    if !has_team {
                        ButtonWithIcon {
                            icon: LdPlus,
                            size: ButtonSize::Compact,
                            variant: ButtonVariant::Default,
                            onclick: move |_| show_create_modal.set(true),
                            "Create New Team"
                        }
                    }
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
                                            on_view: move |team_id| show_view_modal.set(Some(team_id)),
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

        // Modals
        if show_create_modal() {
            {
                let slug = slug.clone();
                rsx! {
                    CreateTeamModal {
                        on_close: move |_| {
                            show_create_modal.set(false);
                            my_team.restart();
                            all_teams.restart();
                        },
                        slug,
                    }
                }
            }
        }

        if let Some(team_id) = show_view_modal() {
            {
                let slug = slug.clone();
                rsx! {
                    ViewTeamModal {
                        on_close: move |_| show_view_modal.set(None),
                        on_request_join: move |team_id| {
                            let team_name = all_teams
                                .read()
                                .as_ref()
                                .and_then(|teams| teams.as_ref())
                                .and_then(|teams| teams.iter().find(|t| t.id == team_id))
                                .map(|t| t.name.clone())
                                .unwrap_or_default();
                            show_view_modal.set(None);
                            show_join_request_modal.set(Some((team_id, team_name)));
                        },
                        slug,
                        team_id,
                        user_has_team: has_team,
                    }
                }
            }
        }

        if let Some((team_id, team_name)) = show_join_request_modal() {
            {
                let slug = slug.clone();
                rsx! {
                    JoinRequestModal {
                        on_close: move |_| {
                            show_join_request_modal.set(None);
                            let _ = dioxus::document::eval("alert('Join request sent!')");
                        },
                        slug,
                        team_id,
                        team_name,
                    }
                }
            }
        }
    }
}

#[component]
fn TeamListItemComponent(team: TeamListItem, on_view: EventHandler<i32>) -> Element {
    rsx! {
        div { class: "py-3 flex items-center justify-between",
            // Team name and description
            div { class: "flex flex-col gap-1 shrink-0",
                p { class: "text-base font-medium leading-6 text-foreground-neutral-primary",
                    "{team.name}"
                }
                if let Some(desc) = &team.description {
                    p { class: "text-xs leading-4 text-foreground-neutral-primary",
                        "{desc}"
                    }
                }
            }

            // Member count
            div { class: "flex items-center justify-center px-4 shrink-0",
                p { class: "text-xs font-medium leading-4 text-foreground-neutral-primary whitespace-nowrap",
                    "{team.member_count}/{team.max_size} Members"
                }
            }

            // Full badge and Details button
            div { class: "flex items-center gap-3 shrink-0",
                Button {
                    size: ButtonSize::Compact,
                    variant: ButtonVariant::Primary,
                    onclick: move |_| on_view.call(team.id),
                    "Details"
                }
            }
        }
    }
}

#[component]
fn JoinRequestCard(
    request: JoinRequestResponse,
    slug: String,
    on_action: EventHandler<()>,
) -> Element {
    let mut is_processing = use_signal(|| false);
    let slug_for_reject = slug.clone();
    let slug_for_accept = slug.clone();

    rsx! {
        div { class: "flex items-center gap-3 p-4 bg-background-neutral-secondary-enabled rounded-lg",
            if let Some(picture) = &request.user_picture {
                img {
                    src: "{picture}",
                    class: "w-12 h-12 rounded-full object-cover",
                }
            } else {
                div { class: "w-12 h-12 rounded-full bg-background-brand-subtle flex items-center justify-center text-foreground-brand-primary font-semibold text-lg",
                    {
                        request
                            .user_name
                            .as_ref()
                            .and_then(|n| n.chars().next())
                            .unwrap_or('U')
                            .to_string()
                    }
                }
            }
            div { class: "flex-1",
                div { class: "flex items-center gap-2 mb-1",
                    p { class: "text-sm font-semibold text-foreground-neutral-primary",
                        {request.user_name.clone().unwrap_or_else(|| "Unknown".to_string())}
                    }
                    span { class: "text-xs text-foreground-neutral-secondary", "{request.created_at}" }
                }
                p { class: "text-xs text-foreground-neutral-secondary mb-1", "{request.user_email}" }
                if let Some(message) = &request.message {
                    p { class: "text-sm text-foreground-neutral-primary mt-2 italic",
                        "\"{message}\""
                    }
                }
            }
            div { class: "flex items-center gap-2",
                Button {
                    variant: ButtonVariant::Danger,
                    onclick: move |_| {
                        let slug = slug_for_reject.clone();
                        let request_id = request.id;
                        spawn(async move {
                            is_processing.set(true);
                            match reject_join_request(slug, request_id).await {
                                Ok(_) => {
                                    on_action.call(());
                                }
                                Err(e) => {
                                    tracing::error!("Failed to reject request: {:?}", e);
                                    is_processing.set(false);
                                }
                            }
                        });
                    },
                    disabled: is_processing(),
                    "Reject"
                }
                Button {
                    variant: ButtonVariant::Success,
                    onclick: move |_| {
                        let slug = slug_for_accept.clone();
                        let request_id = request.id;
                        spawn(async move {
                            is_processing.set(true);
                            match accept_join_request(slug, request_id).await {
                                Ok(_) => {
                                    on_action.call(());
                                }
                                Err(e) => {
                                    tracing::error!("Failed to accept request: {:?}", e);
                                    is_processing.set(false);
                                }
                            }
                        });
                    },
                    disabled: is_processing(),
                    "Accept"
                }
            }
        }
    }
}
