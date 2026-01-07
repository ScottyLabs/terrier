use dioxus::prelude::*;
use dioxus_free_icons::{
    Icon,
    icons::ld_icons::{LdExpand, LdQrCode},
};

use crate::{auth::HackathonRole, ui::foundation::utils::generate_qr_svg};

/// QR Tile component - QR code (using nayuki/QR-Code-generator) with deep link to check-in page with hacker id,
/// "Check in QR Code" text, and an expandable QR code
#[component]
pub fn QRTile() -> Element {
    let user_role = use_context::<Option<HackathonRole>>();
    let user_id = user_role.as_ref().map(|r| r.user_id).unwrap_or(-1);
    let checkin_url = format!("terrier://check-in/{}", user_id);
    let qr_svg = generate_qr_svg(&checkin_url);
    let mut show_modal = use_signal(|| false);
    let is_mobile = use_context::<Signal<bool>>();

    rsx! {
        // Mobile: compact clickable card
        if *is_mobile.read() {
            button {
                class: "flex items-center gap-3 bg-background-neutral-primary rounded-lg p-4 w-full text-left",
                onclick: move |_| show_modal.set(true),
                Icon { icon: LdQrCode, class: "text-foreground-neutral-primary" }
                span { class: "text-foreground-neutral-primary font-medium", "Check-in QR code" }
            }
        } else {
            // Desktop: full tile with QR code visible
            div { class: "flex flex-col gap-4 bg-background-neutral-primary rounded-lg p-6 aspect-square",
                div { class: "flex items-center gap-2",
                    Icon {
                        icon: LdQrCode,
                        class: "text-foreground-neutral-primary",
                    }
                    "Check-in QR Code"
                    button {
                        class: "ml-auto text-black font-semibold text-sm leading-5 rounded-full pl-4 py-2.5",
                        onclick: move |_| show_modal.set(true),
                        Icon {
                            width: 16,
                            height: 16,
                            icon: LdExpand,
                            class: "text-black",
                        }
                    }
                }
                // QR code itself
                QRDisplay { qr_svg: qr_svg.clone(), user_id }
            }
        }

        // Fullscreen QR modal
        if show_modal() {
            QRModal {
                qr_svg: qr_svg.clone(),
                user_id,
                on_close: move |_| show_modal.set(false),
            }
        }
    }
}

/// Reusable QR code display component
#[component]
fn QRDisplay(qr_svg: String, user_id: i32) -> Element {
    rsx! {
        div { class: "w-full h-full flex-col gap-1 flex items-center justify-center p-3",
            div {
                class: "w-full h-full bg-background-neutral-secondary rounded-xl",
                dangerous_inner_html: "{qr_svg}",
            }
            // user id for backup
            div { class: "text-black font-semibold text-sm leading-5", "User ID: {user_id}" }
        }
    }
}

/// Fullscreen QR code modal
#[component]
pub fn QRModal(qr_svg: String, user_id: i32, on_close: EventHandler<()>) -> Element {
    rsx! {
        // Backdrop - covers entire screen with semi-transparent grey
        div {
            class: "fixed inset-0 flex items-center justify-center z-50",
            style: "background-color: rgba(0, 0, 0, 0.7);",
            onclick: move |_| on_close.call(()),

            // Modal content - centered QR code
            div {
                class: "relative flex flex-col items-center justify-center",
                onclick: move |e| e.stop_propagation(),

                // Large QR code display
                div { class: "w-[95vmin] h-[95vmin] max-w-[500px] max-h-[500px] flex flex-col items-center justify-center gap-4",
                    div {
                        class: "w-full h-full bg-background-neutral-primary rounded-2xl",
                        dangerous_inner_html: "{qr_svg}",
                    }
                    div { class: "text-white font-semibold text-lg", "User ID: {user_id}" }
                }
            }
        }
    }
}
