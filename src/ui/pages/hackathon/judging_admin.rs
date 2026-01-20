use dioxus::prelude::*;
use dioxus_free_icons::{
    Icon,
    icons::ld_icons::{LdPlus, LdX},
};

use crate::{
    auth::{JUDGING_ADMIN_ROLES, hooks::use_require_access_or_redirect},
    domain::{
        hackathons::types::HackathonInfo,
        judging::{
            handlers::{
                assign_judges, close_submissions, create_feature, delete_feature, get_features,
                get_features_with_judges, get_judging_status, reopen_submissions, reset_judging,
                start_judging, stop_judging, unassign_judge, update_feature,
            },
            types::{
                AssignJudgesRequest, CreateFeatureRequest, FeatureInfo, FeatureWithJudges,
                JudgeInfo, JudgingStatus, UpdateFeatureRequest,
            },
        },
        people::handlers::query::{HackathonPerson, get_hackathon_people},
    },
    ui::foundation::components::{Button, ButtonVariant},
};

#[component]
pub fn HackathonJudgingAdmin(slug: String) -> Element {
    if let Some(no_access) = use_require_access_or_redirect(JUDGING_ADMIN_ROLES) {
        return no_access;
    }

    let hackathon = use_context::<Signal<HackathonInfo>>();
    let mut status: Signal<Option<JudgingStatus>> = use_signal(|| None);
    let mut features: Signal<Vec<FeatureInfo>> = use_signal(|| Vec::new());
    let mut features_with_judges: Signal<Vec<FeatureWithJudges>> = use_signal(|| Vec::new());
    let mut available_users: Signal<Vec<(i32, String)>> = use_signal(|| Vec::new());
    let mut loading = use_signal(|| false);
    let mut error_msg: Signal<Option<String>> = use_signal(|| None);
    let mut success_msg: Signal<Option<String>> = use_signal(|| None);

    // Selected feature for editing
    let mut selected_feature: Signal<Option<FeatureInfo>> = use_signal(|| None);
    let mut selected_feature_judges: Signal<Vec<JudgeInfo>> = use_signal(|| Vec::new());
    let mut edit_name = use_signal(String::new);
    let mut edit_description = use_signal(String::new);
    let mut is_creating_new = use_signal(|| false);

    // Judge assignment UI state
    let mut show_judge_picker = use_signal(|| false);
    let mut judge_search = use_signal(String::new);

    // Fetch status and features on mount
    let slug_clone = slug.clone();
    use_effect(move || {
        let slug = slug_clone.clone();
        spawn(async move {
            // Fetch status
            match get_judging_status(slug.clone()).await {
                Ok(s) => status.set(Some(s)),
                Err(e) => error_msg.set(Some(format!("Failed to get status: {}", e))),
            }

            // Fetch features
            match get_features(slug.clone()).await {
                Ok(f) => features.set(f),
                Err(e) => error_msg.set(Some(format!("Failed to get features: {}", e))),
            }

            // Fetch features with judges
            match get_features_with_judges(slug.clone()).await {
                Ok(f) => features_with_judges.set(f),
                Err(e) => error_msg.set(Some(format!("Failed to get judge assignments: {}", e))),
            }

            // Fetch available users for judge assignment
            match get_hackathon_people(slug).await {
                Ok(users) => {
                    let user_list: Vec<(i32, String)> = users
                        .into_iter()
                        .map(|u: HackathonPerson| (u.user_id, u.name.unwrap_or_else(|| u.email)))
                        .collect();
                    available_users.set(user_list);
                }
                Err(_) => {} // Silently fail - users just won't be able to add judges
            }
        });
    });

    let refresh_data = {
        let slug = slug.clone();
        move || {
            let slug = slug.clone();
            spawn(async move {
                if let Ok(s) = get_judging_status(slug.clone()).await {
                    status.set(Some(s));
                }
                if let Ok(f) = get_features(slug.clone()).await {
                    features.set(f);
                }
                if let Ok(f) = get_features_with_judges(slug).await {
                    features_with_judges.set(f);
                }
            });
        }
    };

    let do_close_submissions = {
        let slug = slug.clone();
        let refresh = refresh_data.clone();
        move |_| {
            let slug = slug.clone();
            let refresh = refresh.clone();
            spawn(async move {
                loading.set(true);
                error_msg.set(None);
                success_msg.set(None);

                match close_submissions(slug).await {
                    Ok(()) => {
                        success_msg.set(Some("Submissions closed successfully.".to_string()));
                        refresh();
                    }
                    Err(e) => {
                        error_msg.set(Some(format!("Failed to close submissions: {}", e)));
                    }
                }
                loading.set(false);
            });
        }
    };

    let do_reopen_submissions = {
        let slug = slug.clone();
        let refresh = refresh_data.clone();
        move |_| {
            let slug = slug.clone();
            let refresh = refresh.clone();
            spawn(async move {
                loading.set(true);
                error_msg.set(None);
                success_msg.set(None);

                match reopen_submissions(slug).await {
                    Ok(()) => {
                        success_msg.set(Some("Submissions re-opened successfully.".to_string()));
                        refresh();
                    }
                    Err(e) => {
                        error_msg.set(Some(format!("Failed to re-open submissions: {}", e)));
                    }
                }
                loading.set(false);
            });
        }
    };

    let do_start_judging = {
        let slug = slug.clone();
        let refresh = refresh_data.clone();
        move |_| {
            let slug = slug.clone();
            let refresh = refresh.clone();
            spawn(async move {
                loading.set(true);
                error_msg.set(None);
                success_msg.set(None);

                match start_judging(slug).await {
                    Ok(()) => {
                        success_msg.set(Some("Judging started successfully.".to_string()));
                        refresh();
                    }
                    Err(e) => {
                        error_msg.set(Some(format!("Failed to start judging: {}", e)));
                    }
                }
                loading.set(false);
            });
        }
    };

    let do_stop_judging = {
        let slug = slug.clone();
        let refresh = refresh_data.clone();
        move |_| {
            let slug = slug.clone();
            let refresh = refresh.clone();
            spawn(async move {
                loading.set(true);
                error_msg.set(None);
                success_msg.set(None);

                match stop_judging(slug).await {
                    Ok(()) => {
                        success_msg.set(Some("Judging stopped successfully.".to_string()));
                        refresh();
                    }
                    Err(e) => {
                        error_msg.set(Some(format!("Failed to stop judging: {}", e)));
                    }
                }
                loading.set(false);
            });
        }
    };

    let do_reset_judging = {
        let slug = slug.clone();
        let refresh = refresh_data.clone();
        move |_| {
            let slug = slug.clone();
            let refresh = refresh.clone();
            spawn(async move {
                let confirmed = web_sys::window()
                    .and_then(|w| w.confirm_with_message("Are you sure you want to reset judging? This will delete all scores and visits.").ok())
                    .unwrap_or(false);

                if !confirmed {
                    return;
                }

                loading.set(true);
                error_msg.set(None);
                success_msg.set(None);

                match reset_judging(slug).await {
                    Ok(()) => {
                        success_msg.set(Some("Judging reset successfully.".to_string()));
                        refresh();
                    }
                    Err(e) => {
                        error_msg.set(Some(format!("Failed to reset judging: {}", e)));
                    }
                }
                loading.set(false);
            });
        }
    };

    let mut select_feature = move |feature: FeatureInfo| {
        edit_name.set(feature.name.clone());
        edit_description.set(feature.description.clone().unwrap_or_default());

        // Find judges for this feature from cached data
        let judges = features_with_judges
            .read()
            .iter()
            .find(|f| f.feature.id == feature.id)
            .map(|f| f.judges.clone())
            .unwrap_or_default();
        selected_feature_judges.set(judges);

        selected_feature.set(Some(feature));
        is_creating_new.set(false);
        show_judge_picker.set(false);
        judge_search.set(String::new());
    };

    let start_create_feature = move |_| {
        edit_name.set(String::new());
        edit_description.set(String::new());
        selected_feature.set(None);
        selected_feature_judges.set(Vec::new());
        is_creating_new.set(true);
        show_judge_picker.set(false);
        judge_search.set(String::new());
    };

    let do_save_feature = {
        let slug = slug.clone();
        let refresh = refresh_data.clone();
        move |_| {
            let slug = slug.clone();
            let refresh = refresh.clone();
            let name = edit_name.read().clone();
            let description = if edit_description.read().is_empty() {
                None
            } else {
                Some(edit_description.read().clone())
            };
            let is_new = *is_creating_new.read();
            let feature_id = selected_feature.read().as_ref().map(|f| f.id);

            spawn(async move {
                loading.set(true);
                error_msg.set(None);
                success_msg.set(None);

                if is_new {
                    // Create new feature
                    let request = CreateFeatureRequest { name, description };
                    match create_feature(slug, request).await {
                        Ok(_) => {
                            success_msg.set(Some("Feature created successfully.".to_string()));
                            refresh();
                            is_creating_new.set(false);
                            selected_feature.set(None);
                        }
                        Err(e) => {
                            error_msg.set(Some(format!("Failed to create feature: {}", e)));
                        }
                    }
                } else if let Some(fid) = feature_id {
                    // Update existing feature
                    let request = UpdateFeatureRequest { name, description };
                    match update_feature(slug, fid, request).await {
                        Ok(updated) => {
                            success_msg.set(Some("Feature updated successfully.".to_string()));
                            refresh();
                            selected_feature.set(Some(updated));
                        }
                        Err(e) => {
                            error_msg.set(Some(format!("Failed to update feature: {}", e)));
                        }
                    }
                }

                loading.set(false);
            });
        }
    };

    let do_delete_feature = {
        let slug = slug.clone();
        let refresh = refresh_data.clone();
        move |_| {
            if let Some(feature) = selected_feature.read().as_ref() {
                let slug = slug.clone();
                let refresh = refresh.clone();
                let feature_id = feature.id;

                spawn(async move {
                    loading.set(true);
                    error_msg.set(None);
                    success_msg.set(None);

                    match delete_feature(slug, feature_id).await {
                        Ok(_) => {
                            success_msg.set(Some("Feature deleted successfully.".to_string()));
                            refresh();
                            selected_feature.set(None);
                        }
                        Err(e) => {
                            error_msg.set(Some(format!("Failed to delete feature: {}", e)));
                        }
                    }

                    loading.set(false);
                });
            }
        }
    };

    let do_assign_judge = {
        let slug = slug.clone();
        let refresh = refresh_data.clone();
        move |judge_id: i32| {
            if let Some(feature) = selected_feature.read().as_ref() {
                let slug = slug.clone();
                let refresh = refresh.clone();
                let feature_id = feature.id;

                spawn(async move {
                    loading.set(true);
                    error_msg.set(None);

                    let request = AssignJudgesRequest {
                        judge_ids: vec![judge_id],
                    };

                    match assign_judges(slug.clone(), feature_id, request).await {
                        Ok(()) => {
                            success_msg.set(Some("Judge assigned successfully.".to_string()));
                            refresh();

                            // Reload features with judges and update selected feature judges
                            if let Ok(f) = get_features_with_judges(slug).await {
                                features_with_judges.set(f.clone());

                                // Update selected feature judges
                                let judges = f
                                    .iter()
                                    .find(|feat| feat.feature.id == feature_id)
                                    .map(|feat| feat.judges.clone())
                                    .unwrap_or_default();
                                selected_feature_judges.set(judges);
                            }

                            show_judge_picker.set(false);
                            judge_search.set(String::new());
                        }
                        Err(e) => {
                            error_msg.set(Some(format!("Failed to assign judge: {}", e)));
                        }
                    }

                    loading.set(false);
                });
            }
        }
    };

    let do_unassign_judge = {
        let slug = slug.clone();
        let refresh = refresh_data.clone();
        move |judge_id: i32| {
            if let Some(feature) = selected_feature.read().as_ref() {
                let slug = slug.clone();
                let refresh = refresh.clone();
                let feature_id = feature.id;

                spawn(async move {
                    loading.set(true);
                    error_msg.set(None);

                    match unassign_judge(slug.clone(), feature_id, judge_id).await {
                        Ok(()) => {
                            success_msg.set(Some("Judge unassigned successfully.".to_string()));
                            refresh();

                            // Reload features with judges and update selected feature judges
                            if let Ok(f) = get_features_with_judges(slug).await {
                                features_with_judges.set(f.clone());

                                // Update selected feature judges
                                let judges = f
                                    .iter()
                                    .find(|feat| feat.feature.id == feature_id)
                                    .map(|feat| feat.judges.clone())
                                    .unwrap_or_default();
                                selected_feature_judges.set(judges);
                            }
                        }
                        Err(e) => {
                            error_msg.set(Some(format!("Failed to unassign judge: {}", e)));
                        }
                    }

                    loading.set(false);
                });
            }
        }
    };

    let cancel_edit = move |_| {
        selected_feature.set(None);
        is_creating_new.set(false);
        show_judge_picker.set(false);
        judge_search.set(String::new());
    };

    let _hackathon_info = hackathon.read();
    let judging_not_started = status
        .read()
        .as_ref()
        .map(|s| !s.judging_started)
        .unwrap_or(true);

    rsx! {
        div { class: "pt-11 pb-7",
            // Header with Add button
            div { class: "flex items-center justify-between mb-8",
                h1 { class: "text-[30px] font-semibold leading-[38px] text-foreground-neutral-primary",
                    "Judging Admin"
                }

                // Add new feature button (only when judging not started)
                if judging_not_started {
                    button {
                        class: "flex items-center gap-2 px-4 py-2 bg-foreground-neutral-primary text-white font-semibold text-sm rounded-full cursor-pointer",
                        onclick: start_create_feature,
                        Icon { width: 16, height: 16, icon: LdPlus }
                        "Add new feature"
                    }
                }
            }

            // Error message
            if let Some(err) = error_msg.read().as_ref() {
                div { class: "mb-4 p-4 bg-background-danger-secondary rounded-lg",
                    p { class: "text-foreground-danger-primary", "{err}" }
                }
            }

            // Success message
            if let Some(msg) = success_msg.read().as_ref() {
                div { class: "mb-4 p-4 bg-background-success-secondary rounded-lg",
                    p { class: "text-foreground-success-primary", "{msg}" }
                }
            }

            // Status panel
            if let Some(s) = status.read().as_ref() {
                div { class: "mb-8 p-9 bg-background-neutral-primary rounded-[20px]",
                    h2 { class: "text-xl font-semibold text-foreground-neutral-primary mb-4",
                        "Judging Status"
                    }

                    div { class: "grid grid-cols-2 md:grid-cols-4 gap-4 mb-6",
                        div { class: "p-4 border border-stroke-neutral-1 rounded-lg",
                            div { class: "text-sm text-foreground-neutral-secondary",
                                "Submissions Closed"
                            }
                            div {
                                class: "text-lg font-semibold",
                                class: if s.submissions_closed { "text-foreground-success-primary" } else { "text-foreground-neutral-primary" },
                                if s.submissions_closed {
                                    "Yes"
                                } else {
                                    "No"
                                }
                            }
                        }
                        div { class: "p-4 border border-stroke-neutral-1 rounded-lg",
                            div { class: "text-sm text-foreground-neutral-secondary",
                                "Judging Active"
                            }
                            div {
                                class: "text-lg font-semibold",
                                class: if s.judging_started { "text-foreground-success-primary" } else { "text-foreground-neutral-primary" },
                                if s.judging_started {
                                    "Yes"
                                } else {
                                    "No"
                                }
                            }
                        }
                        div { class: "p-4 border border-stroke-neutral-1 rounded-lg",
                            div { class: "text-sm text-foreground-neutral-secondary",
                                "Total Submissions"
                            }
                            div { class: "text-lg font-semibold text-foreground-neutral-primary",
                                "{s.total_submissions}"
                            }
                        }
                        div { class: "p-4 border border-stroke-neutral-1 rounded-lg",
                            div { class: "text-sm text-foreground-neutral-secondary",
                                "Visited"
                            }
                            div { class: "text-lg font-semibold text-foreground-neutral-primary",
                                "{s.visited_submissions}/{s.total_submissions}"
                            }
                        }
                    }

                    div { class: "grid grid-cols-2 gap-4 mb-6",
                        div { class: "p-4 border border-stroke-neutral-1 rounded-lg",
                            div { class: "text-sm text-foreground-neutral-secondary",
                                "Total Visits"
                            }
                            div { class: "text-lg font-semibold text-foreground-neutral-primary",
                                "{s.total_visits}"
                            }
                        }
                        div { class: "p-4 border border-stroke-neutral-1 rounded-lg",
                            div { class: "text-sm text-foreground-neutral-secondary",
                                "Total Comparisons"
                            }
                            div { class: "text-lg font-semibold text-foreground-neutral-primary",
                                "{s.total_comparisons}"
                            }
                        }
                    }

                    // Control buttons
                    div { class: "flex flex-wrap gap-4",
                        if !s.submissions_closed {
                            Button {
                                disabled: *loading.read(),
                                onclick: do_close_submissions,
                                if *loading.read() {
                                    "Closing..."
                                } else {
                                    "Close Submissions"
                                }
                            }
                        }

                        if s.submissions_closed && !s.judging_started {
                            Button {
                                disabled: *loading.read(),
                                onclick: do_reopen_submissions,
                                if *loading.read() {
                                    "Re-opening..."
                                } else {
                                    "Re-open Submissions"
                                }
                            }
                        }

                        if s.submissions_closed && !s.judging_started {
                            Button {
                                disabled: *loading.read(),
                                onclick: do_start_judging,
                                if *loading.read() {
                                    "Starting..."
                                } else {
                                    "Start Judging"
                                }
                            }
                        }

                        if s.judging_started {
                            Button {
                                disabled: *loading.read(),
                                onclick: do_stop_judging,
                                if *loading.read() {
                                    "Stopping..."
                                } else {
                                    "Stop Judging"
                                }
                            }
                        }

                        if s.submissions_closed {
                            Button {
                                variant: ButtonVariant::Danger,
                                disabled: *loading.read(),
                                onclick: do_reset_judging,
                                if *loading.read() {
                                    "Resetting..."
                                } else {
                                    "Reset Judging Data"
                                }
                            }
                        }
                    }
                }
            } else {
                div { class: "mb-8 p-9 bg-background-neutral-primary rounded-[20px]",
                    p { class: "text-foreground-neutral-secondary", "Loading status..." }
                }
            }

            // Features section (only show when judging not started)
            if judging_not_started {
                div { class: "flex flex-col lg:flex-row gap-6",
                    // Left: Feature cards
                    div { class: "flex-1",
                        div { class: "p-9 bg-background-neutral-primary rounded-[20px]",
                            h2 { class: "text-xl font-semibold text-foreground-neutral-primary mb-4",
                                "Judging Features"
                            }

                            if features.read().is_empty() && !*is_creating_new.read() {
                                p { class: "text-foreground-neutral-secondary",
                                    "No judging features defined yet. Click \"Add new feature\" to create one."
                                }
                            } else {
                                div { class: "space-y-4",
                                    for feature in features.read().iter() {
                                        {
                                            let is_selected = selected_feature

                                                .read()
                                                .as_ref()
                                                .map(|f| f.id == feature.id)
                                                .unwrap_or(false);
                                            let feature_clone = feature.clone();
                                            rsx! {
                                                div {
                                                    key: "{feature.id}",
                                                    class: "p-6 border rounded-lg cursor-pointer transition-colors",
                                                    class: if is_selected { "border-foreground-neutral-primary bg-background-neutral-secondary-enabled" } else { "border-stroke-neutral-1 hover:border-stroke-neutral-2" },
                                                    onclick: move |_| select_feature(feature_clone.clone()),

                                                    div { class: "flex items-center justify-between mb-2",
                                                        h3 { class: "font-semibold text-lg text-foreground-neutral-primary", "{feature.name}" }
                                                        span { class: "px-3 py-1 text-xs font-semibold rounded-full bg-foreground-neutral-primary text-white",
                                                            "Edit"
                                                        }
                                                    }

                                                    if let Some(desc) = &feature.description {
                                                        p { class: "text-sm text-foreground-neutral-secondary line-clamp-3", "{desc}" }
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }

                    // Right: Edit panel (only show when editing or creating)
                    if selected_feature.read().is_some() || *is_creating_new.read() {
                        div { class: "w-full lg:w-96",
                            div { class: "p-6 bg-background-neutral-primary rounded-[20px] sticky top-4",
                                // Header
                                div { class: "mb-4",
                                    label { class: "text-xs text-foreground-neutral-secondary",
                                        "Feature name"
                                    }
                                    h3 { class: "text-lg font-semibold text-foreground-neutral-primary",
                                        if *is_creating_new.read() {
                                            "New Feature"
                                        } else {
                                            "{edit_name}"
                                        }
                                    }
                                }

                                // Name input
                                div { class: "mb-4",
                                    label { class: "block text-sm font-medium text-foreground-neutral-primary mb-1",
                                        "Name"
                                    }
                                    input {
                                        class: "w-full px-3 py-2 border border-stroke-neutral-1 rounded-lg text-foreground-neutral-primary bg-background-neutral-primary",
                                        r#type: "text",
                                        value: "{edit_name}",
                                        oninput: move |e| edit_name.set(e.value()),
                                        placeholder: "Feature name",
                                    }
                                }

                                // Description textarea
                                div { class: "mb-6",
                                    label { class: "block text-sm font-medium text-foreground-neutral-primary mb-1",
                                        "Description"
                                    }
                                    textarea {
                                        class: "w-full px-3 py-2 border border-stroke-neutral-1 rounded-lg text-foreground-neutral-primary bg-background-neutral-primary resize-none",
                                        rows: 6,
                                        value: "{edit_description}",
                                        oninput: move |e| edit_description.set(e.value()),
                                        placeholder: "Describe this feature...",
                                    }
                                }

                                // Judge assignment section (only for existing features)
                                if !*is_creating_new.read() {
                                    div { class: "mb-6",
                                        div { class: "flex items-center justify-between mb-2",
                                            label { class: "block text-sm font-medium text-foreground-neutral-primary",
                                                "Assigned Judges"
                                            }
                                            button {
                                                class: "text-sm text-foreground-brand-primary hover:text-foreground-brand-secondary",
                                                onclick: move |_| show_judge_picker.toggle(),
                                                if *show_judge_picker.read() {
                                                    "Cancel"
                                                } else {
                                                    "+ Add Judge"
                                                }
                                            }
                                        }

                                        // List of assigned judges
                                        if selected_feature_judges.read().is_empty() {
                                            p { class: "text-sm text-foreground-neutral-secondary italic",
                                                "No judges assigned yet"
                                            }
                                        } else {
                                            div { class: "space-y-2",
                                                for judge in selected_feature_judges.read().iter() {
                                                    {
                                                        let judge_id = judge.user_id;
                                                        let do_unassign = do_unassign_judge.clone();
                                                        rsx! {
                                                            div {
                                                                key: "{judge.user_id}",
                                                                class: "flex items-center justify-between p-2 bg-background-neutral-secondary-enabled rounded-lg",
                                                                div {
                                                                    p { class: "text-sm font-medium text-foreground-neutral-primary", "{judge.name}" }
                                                                    if let Some(email) = &judge.email {
                                                                        p { class: "text-xs text-foreground-neutral-secondary", "{email}" }
                                                                    }
                                                                }
                                                                button {
                                                                    class: "p-1 text-foreground-danger-primary hover:bg-background-danger-secondary rounded",
                                                                    onclick: move |_| do_unassign(judge_id),
                                                                    Icon { width: 16, height: 16, icon: LdX }
                                                                }
                                                            }
                                                        }
                                                    }
                                                }
                                            }
                                        }

                                        // Judge picker dropdown
                                        if *show_judge_picker.read() {
                                            div { class: "mt-3 p-3 border border-stroke-neutral-1 rounded-lg bg-background-neutral-primary",
                                                input {
                                                    class: "w-full px-3 py-2 border border-stroke-neutral-1 rounded-lg text-foreground-neutral-primary bg-background-neutral-primary mb-2",
                                                    r#type: "text",
                                                    value: "{judge_search}",
                                                    oninput: move |e| judge_search.set(e.value()),
                                                    placeholder: "Search users...",
                                                }

                                                div { class: "max-h-40 overflow-y-auto space-y-1",
                                                    {
                                                        let search_term = judge_search.read().to_lowercase();
                                                        let assigned_ids: Vec<i32> = selected_feature_judges
                                                            .read()
                                                            .iter()

                                                            .map(|j| j.user_id)
                                                            .collect();
                                                        let filtered_users: Vec<(i32, String)> = available_users
                                                            .read()
                                                            .iter()
                                                            .filter(|(id, name)| {
                                                                !assigned_ids.contains(id)
                                                                    && (search_term.is_empty()
                                                                        || name.to_lowercase().contains(&search_term))
                                                            })
                                                            .take(10)
                                                            .cloned()
                                                            .collect();
                                                        if filtered_users.is_empty() {
                                                            rsx! {
                                                                p { class: "text-sm text-foreground-neutral-secondary text-center py-2", "No matching users found" }
                                                            }
                                                        } else {
                                                            rsx! {
                                                                for (user_id , user_name) in filtered_users {
                                                                    {
                                                                        let do_assign = do_assign_judge.clone();
                                                                        rsx! {
                                                                            button {
                                                                                key: "{user_id}",
                                                                                class: "w-full text-left p-2 text-sm text-foreground-neutral-primary hover:bg-background-neutral-secondary-enabled rounded transition-colors",
                                                                                onclick: move |_| do_assign(user_id),
                                                                                "{user_name}"
                                                                            }
                                                                        }
                                                                    }
                                                                }
                                                            }
                                                        }
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }

                                // Action buttons
                                div { class: "flex gap-3 justify-end",
                                    if !*is_creating_new.read() {
                                        Button {
                                            variant: ButtonVariant::Danger,
                                            disabled: *loading.read(),
                                            onclick: do_delete_feature,
                                            "Delete"
                                        }
                                    }

                                    button {
                                        class: "px-4 py-2 text-sm font-medium text-foreground-neutral-primary border border-stroke-neutral-1 rounded-full cursor-pointer",
                                        onclick: cancel_edit,
                                        "Cancel"
                                    }

                                    Button {
                                        disabled: *loading.read() || edit_name.read().is_empty(),
                                        onclick: do_save_feature,
                                        if *loading.read() {
                                            "Saving..."
                                        } else {
                                            "Save"
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}
