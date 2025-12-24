use crate::components::{Button, ButtonVariant};
use dioxus::prelude::*;

#[derive(Clone, Copy, PartialEq)]
pub enum ApplicationStatusVariant {
    Submitted,
    Accepted,
    Confirmed,
}

#[component]
pub fn ApplicationStatus(
    variant: ApplicationStatusVariant,
    hackathon_slug: String,
    application_status: Resource<Option<String>>,
    application_refresh_trigger: Signal<u32>,
) -> Element {
    let mut is_loading = use_signal(|| false);

    let slug_for_team = hackathon_slug.clone();
    let slug_for_unsubmit = hackathon_slug.clone();
    let slug_for_decline = hackathon_slug.clone();
    let slug_for_confirm = hackathon_slug.clone();
    let slug_for_undo = hackathon_slug.clone();

    match variant {
        ApplicationStatusVariant::Submitted => {
            rsx! {
                div { class: "bg-background-neutral-primary rounded-[20px] shadow-[0px_4px_12px_0px_rgba(0,0,0,0.25)] p-9 w-full max-w-[498px]",
                    div { class: "flex flex-col gap-6 mb-9",
                        p { class: "text-[18px] font-medium leading-[26px] text-center w-full",
                            "Your Status"
                        }
                        div { class: "bg-background-brandNeutral-secondary rounded-xl p-3 flex items-center justify-center",
                            p { class: "text-[24px] font-medium leading-8 text-black",
                                "SUBMITTED"
                            }
                        }
                    }
                    p { class: "text-[14px] font-normal leading-5 text-black mb-9",
                        "Thank you for submitting your application! We'll review it and get back to you soon."
                    }

                    div { class: "flex gap-3 w-full",
                        Button {
                            variant: ButtonVariant::Tertiary,
                            class: "flex-1",
                            onclick: move |_| {
                                let nav = navigator();
                                nav.push(format!("/h/{}/team", slug_for_team));
                            },
                            "Find a Team"
                        }
                        Button {
                            variant: ButtonVariant::Default,
                            class: "flex-1",
                            disabled: is_loading(),
                            onclick: move |_| {
                                let slug = slug_for_unsubmit.clone();
                                spawn(async move {
                                    is_loading.set(true);
                                    match crate::hackathons::handlers::applications::unsubmit_application(
                                            slug.clone(),
                                        )
                                        .await
                                    {
                                        Ok(_) => {
                                            application_status.restart();
                                            let current = *application_refresh_trigger.read();
                                            application_refresh_trigger.set(current + 1);
                                            is_loading.set(false);
                                        }
                                        Err(e) => {
                                            let error_msg = format!("Failed to unsubmit: {}", e);
                                            let _ = dioxus::document::eval(
                                                &format!("alert('{}')", error_msg.replace("'", "\\'")),
                                            );
                                            is_loading.set(false);
                                        }
                                    }
                                });
                            },
                            "Unsubmit"
                        }
                    }
                }
            }
        }
        ApplicationStatusVariant::Accepted => {
            rsx! {
                div { class: "bg-background-neutral-primary rounded-[20px] shadow-[0px_4px_12px_0px_rgba(0,0,0,0.25)] p-9 w-full max-w-[498px]",
                    div { class: "flex flex-col gap-6 mb-9",
                        p { class: "text-[18px] font-medium leading-[26px] text-center w-full",
                            "Your Status"
                        }
                        div { class: "bg-background-brandNeutral-secondary rounded-xl p-3 flex items-center justify-center",
                            p { class: "text-[24px] font-medium leading-8 text-black",
                                "ADMITTED"
                            }
                        }
                    }
                    p { class: "text-[14px] font-normal leading-5 text-black mb-9",
                        "Congratulations! You've been accepted. Please confirm your attendance below to see the dashboard."
                    }

                    div { class: "flex gap-3 w-full",
                        Button {
                            variant: ButtonVariant::Tertiary,
                            class: "flex-1",
                            disabled: is_loading(),
                            onclick: move |_| {
                                let slug = slug_for_decline.clone();
                                spawn(async move {
                                    is_loading.set(true);
                                    match crate::hackathons::handlers::applications::decline_attendance(
                                            slug.clone(),
                                        )
                                        .await
                                    {
                                        Ok(_) => {
                                            application_status.restart();
                                            let current = *application_refresh_trigger.read();
                                            application_refresh_trigger.set(current + 1);
                                            is_loading.set(false);
                                        }
                                        Err(e) => {
                                            let error_msg = format!("Failed to decline: {}", e);
                                            let _ = dioxus::document::eval(
                                                &format!("alert('{}')", error_msg.replace("'", "\\'")),
                                            );
                                            is_loading.set(false);
                                        }
                                    }
                                });
                            },
                            if is_loading() {
                                "Processing..."
                            } else {
                                "Decline Attendance"
                            }
                        }
                        Button {
                            variant: ButtonVariant::Default,
                            class: "flex-1",
                            disabled: is_loading(),
                            onclick: move |_| {
                                let slug = slug_for_confirm.clone();
                                spawn(async move {
                                    is_loading.set(true);
                                    match crate::hackathons::handlers::applications::confirm_attendance(
                                            slug.clone(),
                                        )
                                        .await
                                    {
                                        Ok(_) => {
                                            application_status.restart();
                                            let current = *application_refresh_trigger.read();
                                            application_refresh_trigger.set(current + 1);
                                            is_loading.set(false);

                                            // Redirect to dashboard after successful confirmation
                                            let nav = navigator();
                                            nav.push(crate::Route::HackathonDashboard { slug });
                                        }
                                        Err(e) => {
                                            let error_msg = format!("Failed to confirm: {}", e);
                                            let _ = dioxus::document::eval(
                                                &format!("alert('{}')", error_msg.replace("'", "\\'")),
                                            );
                                            is_loading.set(false);
                                        }
                                    }
                                });
                            },
                            if is_loading() {
                                "Processing..."
                            } else {
                                "Confirm"
                            }
                        }
                    }
                }
            }
        }
        ApplicationStatusVariant::Confirmed => {
            rsx! {
                div { class: "bg-background-neutral-primary rounded-[20px] shadow-[0px_4px_12px_0px_rgba(0,0,0,0.25)] p-9 w-full max-w-[498px]",
                    div { class: "flex flex-col gap-6 mb-9",
                        p { class: "text-[18px] font-medium leading-[26px] text-center w-full",
                            "Your Status"
                        }
                        div { class: "bg-background-brandNeutral-secondary rounded-xl p-3 flex items-center justify-center",
                            p { class: "text-[24px] font-medium leading-8 text-black",
                                "CONFIRMED"
                            }
                        }
                    }
                    p { class: "text-[14px] font-normal leading-5 text-black mb-9",
                        "You're all set! You can now access the dashboard and start forming or joining a team."
                    }

                    div { class: "flex gap-3 w-full",
                        Button {
                            variant: ButtonVariant::Tertiary,
                            class: "flex-1",
                            onclick: move |_| {
                                let nav = navigator();
                                nav.push(format!("/h/{}/team", slug_for_team));
                            },
                            "Find a Team"
                        }
                        Button {
                            variant: ButtonVariant::Default,
                            class: "flex-1",
                            disabled: is_loading(),
                            onclick: move |_| {
                                let slug = slug_for_undo.clone();
                                spawn(async move {
                                    is_loading.set(true);
                                    match crate::hackathons::handlers::applications::undo_confirmation(
                                            slug.clone(),
                                        )
                                        .await
                                    {
                                        Ok(_) => {
                                            application_status.restart();
                                            let current = *application_refresh_trigger.read();
                                            application_refresh_trigger.set(current + 1);
                                            is_loading.set(false);
                                        }
                                        Err(e) => {
                                            let error_msg = format!("Failed to undo: {}", e);
                                            let _ = dioxus::document::eval(
                                                &format!("alert('{}')", error_msg.replace("'", "\\'")),
                                            );
                                            is_loading.set(false);
                                        }
                                    }
                                });
                            },
                            if is_loading() {
                                "Processing..."
                            } else {
                                "Undo"
                            }
                        }
                    }
                }
            }
        }
    }
}
