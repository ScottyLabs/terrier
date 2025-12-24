use crate::auth::{TEAM_ROLES, hooks::use_require_access_or_redirect};
use crate::domain::applications::handlers::get_application;
use crate::domain::hackathons::types::HackathonInfo;
use crate::domain::teams::{
    InvitationResponse, JoinRequestResponse, OutgoingJoinRequestResponse, TeamListItem,
    handlers::{
        accept_invitation, accept_join_request, cancel_outgoing_join_request, decline_invitation,
        get_all_teams, get_join_requests, get_my_invitations, get_my_team,
        get_outgoing_join_requests, kick_member, leave_team, leave_team_force, reject_join_request,
        transfer_ownership,
    },
};
use crate::ui::features::teams::{
    CreateTeamModal, EditTeamModal, InviteMembersModal, JoinRequestModal, ViewTeamModal,
};
use crate::ui::foundation::components::{
    Button, ButtonSize, ButtonVariant, ButtonWithIcon, TabSwitcher,
};
use crate::ui::foundation::modals::ModalBase;
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

    let nav = navigator();
    let slug_for_app_check = slug.clone();

    // Check if user has submitted application
    let application_resource = use_resource(move || {
        let slug = slug_for_app_check.clone();
        async move { get_application(slug).await.ok() }
    });

    // If application is not submitted (status == "draft" or doesn't exist), redirect to apply page
    if let Some(app_opt) = application_resource.read().as_ref() {
        match app_opt {
            None => {
                // No application found, redirect to apply page
                use_effect(move || {
                    nav.push(crate::Route::HackathonApply { slug: slug.clone() });
                });
                return rsx! {
                    div { class: "flex items-center justify-center h-full",
                        p { class: "text-foreground-neutral-primary", "Redirecting to application..." }
                    }
                };
            }
            Some(app) if app.status == "draft" => {
                // Application exists but not submitted, redirect to apply page
                use_effect(move || {
                    nav.push(crate::Route::HackathonApply { slug: slug.clone() });
                });
                return rsx! {
                    div { class: "flex items-center justify-center h-full",
                        p { class: "text-foreground-neutral-primary",
                            "Please submit your application first..."
                        }
                    }
                };
            }
            _ => {
                // Application submitted, continue
            }
        }
    } else {
        // Still loading application, show loading state
        return rsx! {
            div { class: "flex items-center justify-center h-full",
                p { class: "text-foreground-neutral-primary", "Loading..." }
            }
        };
    }

    let _hackathon = use_context::<Signal<HackathonInfo>>();
    let mut search_query = use_signal(String::new);
    let active_tab = use_signal(|| MyTeamTab::Current);

    // Modal states
    let mut show_create_modal = use_signal(|| false);
    let mut show_view_modal = use_signal(|| None::<i32>);
    let mut show_join_request_modal = use_signal(|| None::<(i32, String)>);
    let mut show_edit_modal = use_signal(|| false);
    let mut show_invite_modal = use_signal(|| false);
    let mut show_kick_modal = use_signal(|| None::<(i32, String)>); // (user_id, user_name)
    let mut show_transfer_modal = use_signal(|| false);
    let mut show_leave_confirm_modal = use_signal(|| false);
    let mut show_outgoing_request_modal = use_signal(|| None::<OutgoingJoinRequestResponse>);
    let mut show_cancel_requests_confirm = use_signal(|| None::<PendingAction>); // Action to perform after canceling requests

    #[derive(Clone)]
    enum PendingAction {
        CreateTeam,
        AcceptInvitation(i32), // invitation_id
    }

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

    let slug_for_invitations = slug.clone();
    let mut my_invitations = use_resource(move || {
        let slug = slug_for_invitations.clone();
        async move {
            match get_my_invitations(slug).await {
                Ok(invitations) => Some(invitations),
                Err(e) => {
                    tracing::error!("Error fetching invitations: {:?}", e);
                    None
                }
            }
        }
    });

    let slug_for_outgoing = slug.clone();
    let mut outgoing_requests = use_resource(move || {
        let slug = slug_for_outgoing.clone();
        async move {
            match get_outgoing_join_requests(slug).await {
                Ok(requests) => Some(requests),
                Err(e) => {
                    tracing::error!("Error fetching outgoing requests: {:?}", e);
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
            // My Team section
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
                                let is_owner = team.is_owner;
                                let member_count = team.member_count;
                                rsx! {
                                    ButtonWithIcon {
                                        icon: dioxus_free_icons::icons::ld_icons::LdLogOut,
                                        size: ButtonSize::Compact,
                                        variant: ButtonVariant::Outline,
                                        onclick: move |_| {
                                            if is_owner && member_count > 1 {
                                                show_leave_confirm_modal.set(true);
                                            } else {
                                                let slug = slug_for_leave.clone();
                                                spawn(async move {
                                                    match leave_team(slug).await {
                                                        Ok(_) => {
                                                            let _ = dioxus::document::eval(
                                                                "alert('Left team successfully')",
                                                            );
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
                                            }
                                        },
                                        "Leave Team"
                                    }
                                }
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
                                            onclick: move |_| show_edit_modal.set(true),
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
                                                {
                                                    let is_owner_row = index == 0;
                                                    let member_id = member.user_id;
                                                    let member_name = member.name.clone().unwrap_or_else(|| "Unknown".to_string());
                                                    rsx! {
                                                        div { key: "{member.user_id}", class: "flex items-center justify-between gap-4",
                                                            div { class: "flex items-center gap-4",
                                                                if let Some(picture) = &member.picture {
                                                                    img { src: "{picture}", class: "w-8 h-8 rounded-full object-cover" }
                                                                } else {
                                                                    div { class: "w-8 h-8 rounded-full bg-background-brand-subtle flex items-center justify-center text-foreground-brand-primary font-semibold text-sm",
                                                                        {member.name.as_ref().and_then(|n| n.chars().next()).unwrap_or('U').to_string()}
                                                                    }
                                                                }
                                                                p { class: "text-sm font-semibold text-foreground-neutral-primary",
                                                                    "{member_name}"
                                                                    if is_owner_row {
                                                                        span { class: "ml-2 px-2 py-0.5 text-xs bg-background-brand-subtle text-foreground-brand-primary rounded-md font-normal",
                                                                            "Owner"
                                                                        }
                                                                    }
                                                                }
                                                            }
                                                            // Owner controls
                                                            if team.is_owner && !is_owner_row {
                                                                div { class: "flex items-center gap-2",
                                                                    Button {
                                                                        size: ButtonSize::Compact,
                                                                        variant: ButtonVariant::Outline,
                                                                        onclick: move |_| {
                                                                            show_transfer_modal.set(true);
                                                                        },
                                                                        "Make Owner"
                                                                    }
                                                                    Button {
                                                                        size: ButtonSize::Compact,
                                                                        variant: ButtonVariant::Danger,
                                                                        onclick: move |_| {
                                                                            show_kick_modal.set(Some((member_id, member_name.clone())));
                                                                        },
                                                                        "Kick"
                                                                    }
                                                                }
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
                                            onclick: move |_| show_invite_modal.set(true),
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
                            onclick: move |_| {
                                let has_outgoing = outgoing_requests
                                    .read()
                                    .as_ref()
                                    .and_then(|o| o.as_ref())
                                    .map(|requests| !requests.is_empty())
                                    .unwrap_or(false);
                                if has_outgoing {
                                    show_cancel_requests_confirm.set(Some(PendingAction::CreateTeam));
                                } else {
                                    show_create_modal.set(true);
                                }
                            },
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
                        Some(Some(teams)) => {
                            let invitations = my_invitations.read();
                            let outgoing = outgoing_requests.read();
                            let has_invitations = !has_team
                                && invitations
                                    .as_ref()
                                    .and_then(|i| i.as_ref())
                                    .map(|i| !i.is_empty())
                                    .unwrap_or(false);
                            let has_outgoing = !has_team
                                && outgoing
                                    .as_ref()
                                    .and_then(|o| o.as_ref())
                                    .map(|o| !o.is_empty())
                                    .unwrap_or(false);
                            let is_empty = teams.is_empty() && !has_invitations && !has_outgoing;
                            rsx! {
                                if is_empty {
                                    div { class: "text-center text-foreground-neutral-tertiary", "No teams found" }
                                } else {
                                    div { class: "divide-y divide-stroke-neutral-1",
                                        // Show invitations first if user has no team
                                        if !has_team && let Some(Some(invitations)) = invitations.as_ref() {
                                            for invitation in invitations {
                                                InvitationCard {
                                                    key: "inv-{invitation.id}",
                                                    invitation: invitation.clone(),
                                                    slug: slug_clone.clone(),
                                                    on_action: move |_| {
                                                        my_invitations.restart();
                                                        my_team.restart();
                                                        all_teams.restart();
                                                    },
                                                    on_accept_click: {
                                                        let slug_for_accept = slug_clone.clone();
                                                        let my_invitations_for_accept = my_invitations.clone();
                                                        let my_team_for_accept = my_team.clone();
                                                        let all_teams_for_accept = all_teams.clone();
                                                        move |invitation_id| {
                                                            let has_outgoing = outgoing_requests
                                                                .read()
                                                                .as_ref()
                                                                .and_then(|o| o.as_ref())
                                                                .map(|requests| !requests.is_empty())
                                                                .unwrap_or(false);
                                                            if has_outgoing {
                                                                show_cancel_requests_confirm
                                                                    .set(Some(PendingAction::AcceptInvitation(invitation_id)));
                                                            } else {
                                                                let slug = slug_for_accept.clone();
                                                                let mut my_invitations = my_invitations_for_accept.clone();
                                                                let mut my_team = my_team_for_accept.clone();
                                                                let mut all_teams = all_teams_for_accept.clone();
                                                                spawn(async move {
                                                                    match accept_invitation(slug, invitation_id).await {
                                                                        Ok(_) => {
                                                                            my_invitations.restart();
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
                                                            }
                                                        }
                                                    },
                                                }
                                            }
                                        }

                                        // Show outgoing requests if user has no team
                                        if !has_team && let Some(Some(requests)) = outgoing.as_ref() {
                                            for request in requests {
                                                OutgoingRequestCard {
                                                    key: "out-{request.id}",
                                                    request: request.clone(),
                                                    on_click: move |req| show_outgoing_request_modal.set(Some(req)),
                                                }
                                            }
                                        }

                                        // Then show all teams (excluding teams with invitations or outgoing requests)
                                        {
                                            let mut excluded_team_ids: Vec<i32> = if !has_team {
                                                invitations
                                                    .as_ref()
                                                    .and_then(|inv| inv.as_ref())
                                                    .map(|invs| invs.iter().map(|i| i.team_id).collect())
                                                    .unwrap_or_default()
                                            } else {
                                                Vec::new()
                                            };
                                            if !has_team && let Some(Some(requests)) = outgoing.as_ref() {
                                                excluded_team_ids.extend(requests.iter().map(|r| r.team_id));
                                            }
                                            rsx! {
                                                for team_item in teams.iter().filter(|t| !excluded_team_ids.contains(&t.id)) {
                                                    TeamListItemComponent {
                                                        key: "{team_item.id}",
                                                        team: team_item.clone(),
                                                        on_view: move |team_id| show_view_modal.set(Some(team_id)),
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
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
                            join_requests.restart();
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
                            outgoing_requests.restart();
                            all_teams.restart();
                        },
                        slug,
                        team_id,
                        team_name,
                    }
                }
            }
        }

        if show_edit_modal() {
            if let Some(Some(team)) = &*my_team.read() {
                {
                    let slug = slug.clone();
                    let team_name = team.name.clone();
                    let team_description = team.description.clone();
                    rsx! {
                        EditTeamModal {
                            on_close: move |_| {
                                show_edit_modal.set(false);
                                my_team.restart();
                                all_teams.restart();
                            },
                            slug,
                            team_name,
                            team_description,
                        }
                    }
                }
            }
        }

        if show_invite_modal() {
            {
                let slug = slug.clone();
                rsx! {
                    InviteMembersModal {
                        on_close: move |_| {
                            show_invite_modal.set(false);
                            my_team.restart();
                            my_invitations.restart();
                            all_teams.restart();
                        },
                        slug,
                    }
                }
            }
        }

        // Kick member confirmation modal
        if let Some((user_id, user_name)) = show_kick_modal() {
            {
                let slug = slug.clone();
                rsx! {
                    ModalBase {
                        on_close: move |_| show_kick_modal.set(None),
                        width: "500px",
                        max_height: "none",
                        div { class: "p-7",
                            h2 { class: "text-2xl font-semibold text-foreground-neutral-primary mb-4",
                                "Kick Member"
                            }
                            p { class: "text-base text-foreground-neutral-secondary mb-6",
                                "Are you sure you want to kick {user_name} from the team?"
                            }
                            div { class: "flex gap-3 justify-end",
                                Button {
                                    variant: ButtonVariant::Tertiary,
                                    onclick: move |_| show_kick_modal.set(None),
                                    "Cancel"
                                }
                                Button {
                                    variant: ButtonVariant::Danger,
                                    onclick: move |_| {
                                        let slug = slug.clone();
                                        spawn(async move {
                                            match kick_member(slug, user_id).await {
                                                Ok(_) => {
                                                    show_kick_modal.set(None);
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
                                    "Kick"
                                }
                            }
                        }
                    }
                }
            }
        }

        // Transfer ownership modal
        if show_transfer_modal() {
            if let Some(Some(team)) = &*my_team.read() {
                {
                    let slug = slug.clone();
                    let members = team.members.clone();
                    rsx! {
                        ModalBase {
                            on_close: move |_| show_transfer_modal.set(false),
                            width: "500px",
                            max_height: "none",
                            div { class: "p-7",
                                h2 { class: "text-2xl font-semibold text-foreground-neutral-primary mb-4",
                                    "Transfer Ownership"
                                }
                                p { class: "text-base text-foreground-neutral-secondary mb-6",
                                    "Select a team member to transfer ownership to:"
                                }
                                div { class: "flex flex-col gap-2 mb-6",
                                    for member in members.iter().skip(1) {
                                        {
                                            let member_id = member.user_id;
                                            let member_name = member.name.clone().unwrap_or_else(|| "Unknown".to_string());
                                            let slug_for_btn = slug.clone();
                                            rsx! {
                                                button {
                                                    key: "{member.user_id}",
                                                    class: "flex items-center gap-3 p-3 rounded-lg border border-stroke-neutral-1 hover:bg-background-neutral-secondary-enabled transition-colors",
                                                    onclick: move |_| {
                                                        let slug = slug_for_btn.clone();
                                                        spawn(async move {
                                                            match transfer_ownership(slug, member_id).await {
                                                                Ok(_) => {
                                                                    show_transfer_modal.set(false);
                                                                    let _ = dioxus::document::eval(
                                                                        "alert('Ownership transferred successfully')",
                                                                    );
                                                                    my_team.restart();
                                                                    all_teams.restart();
                                                                    join_requests.restart();
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
                                                    if let Some(picture) = &member.picture {
                                                        img { src: "{picture}", class: "w-10 h-10 rounded-full object-cover" }
                                                    } else {
                                                        div { class: "w-10 h-10 rounded-full bg-background-brand-subtle flex items-center justify-center text-foreground-brand-primary font-semibold",
                                                            {member.name.as_ref().and_then(|n| n.chars().next()).unwrap_or('U').to_string()}
                                                        }
                                                    }
                                                    p { class: "text-sm font-semibold text-foreground-neutral-primary", "{member_name}" }
                                                }
                                            }
                                        }
                                    }
                                }
                                div { class: "flex justify-end",
                                    Button {
                                        variant: ButtonVariant::Tertiary,
                                        onclick: move |_| show_transfer_modal.set(false),
                                        "Cancel"
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        // Leave team confirmation modal (for owner with members)
        if show_leave_confirm_modal() {
            if let Some(Some(team)) = &*my_team.read() {
                {
                    let slug = slug.clone();
                    let member_count = team.member_count;
                    let other_members = member_count - 1;
                    let member_text = if other_members == 1 {
                        "the 1 other member".to_string()
                    } else {
                        format!("all {other_members} members")
                    };
                    rsx! {
                        ModalBase {
                            on_close: move |_| show_leave_confirm_modal.set(false),
                            width: "500px",
                            max_height: "none",
                            div { class: "p-7",
                                h2 { class: "text-2xl font-semibold text-foreground-neutral-primary mb-4",
                                    "Leave and Delete Team"
                                }
                                p { class: "text-base text-foreground-neutral-secondary mb-6",
                                    "As the team owner, leaving will kick {member_text} and delete the team. This action cannot be undone."
                                }
                                div { class: "flex gap-3 justify-end",
                                    Button {
                                        variant: ButtonVariant::Tertiary,
                                        onclick: move |_| show_leave_confirm_modal.set(false),
                                        "Cancel"
                                    }
                                    Button {
                                        variant: ButtonVariant::Danger,
                                        onclick: move |_| {
                                            let slug = slug.clone();
                                            spawn(async move {
                                                match leave_team_force(slug, true).await {
                                                    Ok(_) => {
                                                        show_leave_confirm_modal.set(false);
                                                        let _ = dioxus::document::eval("alert('Team deleted successfully')");
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
                                        "Leave and Delete Team"
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        // Outgoing join request modal
        if let Some(request) = show_outgoing_request_modal() {
            {
                let slug = slug.clone();
                let request_id = request.id;
                let team_name = request.team_name.clone();
                rsx! {
                    ModalBase {
                        on_close: move |_| show_outgoing_request_modal.set(None),
                        width: "500px",
                        max_height: "none",
                        div { class: "p-7",
                            h2 { class: "text-2xl font-semibold text-foreground-neutral-primary mb-4",
                                "Join Request for {team_name}"
                            }
                            div { class: "mb-6",
                                p { class: "text-base text-foreground-neutral-secondary mb-2",
                                    "You have a pending join request for this team."
                                }
                                if let Some(message) = &request.message {
                                    div { class: "mt-4 p-3 bg-background-neutral-secondary-enabled rounded-lg",
                                        p { class: "text-sm text-foreground-neutral-secondary mb-1", "Your message:" }
                                        p { class: "text-sm text-foreground-neutral-primary italic", "\"{message}\"" }
                                    }
                                }
                                p { class: "text-xs text-foreground-neutral-secondary mt-4",
                                    "Requested on {request.created_at}"
                                }
                            }
                            div { class: "flex gap-3 justify-end",
                                Button {
                                    variant: ButtonVariant::Tertiary,
                                    onclick: move |_| show_outgoing_request_modal.set(None),
                                    "Close"
                                }
                                Button {
                                    variant: ButtonVariant::Danger,
                                    onclick: move |_| {
                                        let slug = slug.clone();
                                        spawn(async move {
                                            match cancel_outgoing_join_request(slug, request_id).await {
                                                Ok(_) => {
                                                    show_outgoing_request_modal.set(None);
                                                    let _ = dioxus::document::eval("alert('Join request cancelled')");
                                                    outgoing_requests.restart();
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
                                    "Cancel Request"
                                }
                            }
                        }
                    }
                }
            }
        }

        // Cancel pending requests confirmation modal
        if let Some(pending_action) = show_cancel_requests_confirm() {
            {
                let slug = slug.clone();
                let requests_count = outgoing_requests
                    .read()
                    .as_ref()
                    .and_then(|o| o.as_ref())
                    .map(|requests| requests.len())
                    .unwrap_or(0);
                let request_text = if requests_count == 1 { "request" } else { "requests" };
                let action_text = match &pending_action {
                    PendingAction::CreateTeam => "create a new team",
                    PendingAction::AcceptInvitation(_) => "join this team",
                };
                rsx! {
                    ModalBase {
                        on_close: move |_| show_cancel_requests_confirm.set(None),
                        width: "500px",
                        max_height: "none",
                        div { class: "p-7",
                            h2 { class: "text-2xl font-semibold text-foreground-neutral-primary mb-4",
                                "Cancel Pending Requests?"
                            }
                            p { class: "text-base text-foreground-neutral-secondary mb-6",
                                "You have {requests_count} pending join {request_text}. To {action_text}, these requests will be automatically cancelled."
                            }
                            div { class: "flex gap-3 justify-end",
                                Button {
                                    variant: ButtonVariant::Tertiary,
                                    onclick: move |_| show_cancel_requests_confirm.set(None),
                                    "Cancel"
                                }
                                Button {
                                    variant: ButtonVariant::Primary,
                                    onclick: move |_| {
                                        let slug = slug.clone();
                                        let action = pending_action.clone();
                                        spawn(async move {
                                            let request_ids: Vec<i32> = {
                                                let requests = outgoing_requests.read();
                                                requests
                                                    .as_ref()
                                                    .and_then(|o| o.as_ref())
                                                    .map(|reqs| reqs.iter().map(|r| r.id).collect())
                                                    .unwrap_or_default()
                                            };
                                            for request_id in request_ids {
                                                if let Err(e) = cancel_outgoing_join_request(slug.clone(), request_id)
                                                    .await
                                                {
                                                    let error_msg = e.to_string();
                                                    let _ = dioxus::document::eval(
                                                        &format!(
                                                            "alert('Error canceling request: {}')",
                                                            error_msg.replace("'", "\\'"),
                                                        ),
                                                    );
                                                    show_cancel_requests_confirm.set(None);
                                                    return;
                                                }
                                            }
                                            outgoing_requests.restart();
                                            match action {
                                                PendingAction::CreateTeam => {
                                                    show_cancel_requests_confirm.set(None);
                                                    show_create_modal.set(true);
                                                }
                                                PendingAction::AcceptInvitation(invitation_id) => {
                                                    match accept_invitation(slug, invitation_id).await {
                                                        Ok(_) => {
                                                            show_cancel_requests_confirm.set(None);
                                                            my_invitations.restart();
                                                            my_team.restart();
                                                            all_teams.restart();
                                                        }
                                                        Err(e) => {
                                                            show_cancel_requests_confirm.set(None);
                                                            let error_msg = e.to_string();
                                                            let _ = dioxus::document::eval(
                                                                &format!("alert('{}')", error_msg.replace("'", "\\'")),
                                                            );
                                                        }
                                                    }
                                                }
                                            }
                                        });
                                    },
                                    "Confirm"
                                }
                            }
                        }
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
            div { class: "flex flex-col gap-1 min-w-0 flex-1",
                p { class: "text-base font-medium leading-6 text-foreground-neutral-primary truncate",
                    "{team.name}"
                }
                if let Some(desc) = &team.description {
                    p { class: "text-xs leading-4 text-foreground-neutral-primary truncate",
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
                    size: ButtonSize::Compact,
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
                    size: ButtonSize::Compact,
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

#[component]
fn InvitationCard(
    invitation: InvitationResponse,
    slug: String,
    on_action: EventHandler<()>,
    on_accept_click: EventHandler<i32>, // Called with invitation_id when accept is clicked
) -> Element {
    let mut is_processing = use_signal(|| false);
    let slug_for_decline = slug.clone();

    rsx! {
        div { class: "py-3 flex items-center justify-between",
            // Team info
            div { class: "flex flex-col gap-1 shrink-0",
                div { class: "flex items-center gap-2",
                    p { class: "text-base font-medium leading-6 text-foreground-neutral-primary",
                        "{invitation.team_name}"
                    }
                    span { class: "px-2 py-0.5 text-xs bg-background-brandNeutral-secondary text-foreground-brandNeutral-primary rounded-md font-semibold",
                        "Invitation"
                    }
                }
                if let Some(message) = &invitation.message {
                    p { class: "text-xs leading-4 text-foreground-neutral-primary italic",
                        "\"{message}\""
                    }
                }
                p { class: "text-xs text-foreground-neutral-secondary",
                    "Invited {invitation.created_at}"
                }
            }

            // Accept/Decline buttons
            div { class: "flex items-center gap-2 shrink-0",
                Button {
                    size: ButtonSize::Compact,
                    variant: ButtonVariant::Danger,
                    onclick: move |_| {
                        let slug = slug_for_decline.clone();
                        let invitation_id = invitation.id;
                        spawn(async move {
                            is_processing.set(true);
                            match decline_invitation(slug, invitation_id).await {
                                Ok(_) => {
                                    on_action.call(());
                                }
                                Err(e) => {
                                    tracing::error!("Failed to decline invitation: {:?}", e);
                                    is_processing.set(false);
                                }
                            }
                        });
                    },
                    disabled: is_processing(),
                    "Decline"
                }
                Button {
                    size: ButtonSize::Compact,
                    variant: ButtonVariant::Success,
                    onclick: move |_| {
                        on_accept_click.call(invitation.id);
                    },
                    disabled: is_processing(),
                    "Accept"
                }
            }
        }
    }
}

#[component]
fn OutgoingRequestCard(
    request: OutgoingJoinRequestResponse,
    on_click: EventHandler<OutgoingJoinRequestResponse>,
) -> Element {
    rsx! {
        div { class: "py-3 flex items-center justify-between",
            // Team info
            div { class: "flex flex-col gap-1 shrink-0",
                div { class: "flex items-center gap-2",
                    p { class: "text-base font-medium leading-6 text-foreground-neutral-primary",
                        "{request.team_name}"
                    }
                    span { class: "px-2 py-0.5 text-xs bg-background-status-info text-foreground-neutral-primary rounded-md font-semibold",
                        "Pending Request"
                    }
                }
                if let Some(message) = &request.message {
                    p { class: "text-xs leading-4 text-foreground-neutral-primary italic",
                        "\"{message}\""
                    }
                }
                p { class: "text-xs text-foreground-neutral-secondary",
                    "Requested {request.created_at}"
                }
            }

            // View Details button
            div { class: "flex items-center gap-2 shrink-0",
                Button {
                    size: ButtonSize::Compact,
                    variant: ButtonVariant::Outline,
                    onclick: move |_| on_click.call(request.clone()),
                    "View Details"
                }
            }
        }
    }
}
