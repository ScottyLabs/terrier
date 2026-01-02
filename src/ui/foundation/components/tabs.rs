use dioxus::prelude::*;

#[component]
pub fn TabSwitcher<T: Clone + PartialEq + 'static>(
    active_tab: Signal<T>,
    tabs: Vec<(T, String)>,
) -> Element {
    rsx! {
        div { class: "flex md:inline-flex flex-col md:flex-row items-center md:justify-center gap-2 p-1 bg-background-neutral-subtle-pressed rounded-xl md:rounded-full",
            for (value , label) in tabs {
                button {
                    key: "{label}",
                    class: if active_tab() == value { "px-4 py-2 rounded-full bg-background-neutral-primary text-foreground-neutral-primary font-semibold text-sm w-full md:w-fit" } else { "px-4 py-2 rounded-full bg-transparent text-foreground-neutral-primary font-semibold text-sm w-full md:w-fit" },
                    onclick: {
                        let v = value.clone();
                        move |_| active_tab.set(v.clone())
                    },
                    "{label}"
                }
            }
        }
    }
}
