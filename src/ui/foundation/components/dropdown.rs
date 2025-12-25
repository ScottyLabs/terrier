use dioxus::prelude::*;
use dioxus_free_icons::{Icon, icons::ld_icons::LdCheck};

#[derive(Clone, PartialEq)]
pub struct DropdownOption {
    pub label: String,
    pub value: String,
    pub selected: bool,
}

#[component]
pub fn Dropdown(options: Vec<DropdownOption>, on_change: EventHandler<Vec<String>>) -> Element {
    rsx! {
        div { class: "bg-background-neutral-primary border-[0.5px] border-stroke-neutral-2 rounded-lg py-2 w-[332px] shadow-lg",
            for option in &options {
                {
                    let option_value = option.value.clone();
                    let option_label = option.label.clone();
                    let option_selected = option.selected;
                    let options_clone = options.clone();

                    rsx! {
                        div {
                            key: "{option_value}",
                            class: if option_selected { "bg-background-neutral-subtle-pressed" } else { "bg-transparent hover:bg-background-neutral-secondary-enabled" },
                            onclick: move |_| {
                                let mut selected_values: Vec<String> = options_clone
                                    .iter()
                                    .filter(|o| o.selected && o.value != option_value)
                                    .map(|o| o.value.clone())
                                    .collect();

                                if !option_selected {
                                    selected_values.push(option_value.clone());
                                }

                                on_change.call(selected_values);
                            },
                            div { class: "flex gap-3 items-center px-3.5 py-2 h-9",
                                div { class: "flex items-center justify-center p-2",
                                    if option_selected {
                                        div { class: "w-4 h-4 bg-foreground-neutral-primary rounded flex items-center justify-center",
                                            Icon {
                                                width: 12,
                                                height: 12,
                                                icon: LdCheck,
                                                class: "text-white",
                                            }
                                        }
                                    } else {
                                        div { class: "w-4 h-4 border border-foreground-neutral-primary rounded" }
                                    }
                                }
                                p { class: "text-sm leading-5 text-foreground-neutral-primary flex-1", "{option_label}" }
                            }
                        }
                    }
                }
            }
        }
    }
}
