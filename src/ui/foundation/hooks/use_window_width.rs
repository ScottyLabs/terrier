use dioxus::prelude::*;

pub fn use_window_width() -> Signal<f64> {
    let mut width = use_signal(|| 0.0);

    #[cfg(target_arch = "wasm32")]
    {
        let mut _listener = use_signal(|| None as Option<gloo_events::EventListener>);

        use_effect(move || {
            let window = match web_sys::window() {
                Some(w) => w,
                None => return,
            };

            let get_width = || {
                window
                    .inner_width()
                    .ok()
                    .and_then(|w| w.as_f64())
                    .unwrap_or(0.0)
            };

            // Initial set
            width.set(get_width());

            let l = gloo_events::EventListener::new(&window, "resize", move |_| {
                if let Some(w) = web_sys::window() {
                    if let Ok(v) = w.inner_width() {
                        if let Some(f) = v.as_f64() {
                            width.set(f);
                        }
                    }
                }
            });

            _listener.set(Some(l));
        });
    }

    width
}
