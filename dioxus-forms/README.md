# Dioxus Forms

A form management library for Dioxus applications that provides:

- Field-level state management with dirty/touched tracking
- Synchronous and asynchronous validation
- Form-level state aggregation
- Dynamic field arrays for repeatable field groups

## Quick Start

```rust,ignore
use dioxus::prelude::*;
use dioxus_forms::*;

#[component]
fn MyForm() -> Element {
    let mut form = use_form();
    let name = use_form_field(String::new())
        .with_validator(validators::min_length(3, "Name too short"));

    form.register_field(&name);

    let (value, on_input, on_blur) = use_field_bind(&name);

    rsx! {
        input { value, oninput: on_input, onblur: on_blur }
        if name.touched() && name.error().is_some() {
            span { "{name.error().unwrap()}" }
        }
    }
}
```
