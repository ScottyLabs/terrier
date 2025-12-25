use dioxus::prelude::*;

/// State for an async action
#[derive(Clone, Default)]
pub struct AsyncActionState {
    pub is_loading: bool,
    pub error: Option<String>,
}

/// Hook for managing async API call state with loading and error handling
///
/// # Example
/// ```rust
/// let action = use_async_action(cx, move |data: MyData| async move {
///     my_server_fn(data).await
/// });
///
/// // In your component:
/// if action.is_loading() {
///     rsx! { "Loading..." }
/// }
///
/// if let Some(error) = action.error() {
///     rsx! { "Error: {error}" }
/// }
///
/// // Execute the action:
/// action.execute(my_data);
/// ```
#[derive(Clone, Copy)]
pub struct AsyncAction {
    state: Signal<AsyncActionState>,
}

impl AsyncAction {
    pub fn new() -> Self {
        Self {
            state: use_signal(AsyncActionState::default),
        }
    }

    pub fn is_loading(&self) -> bool {
        self.state.read().is_loading
    }

    pub fn error(&self) -> Option<String> {
        self.state.read().error.clone()
    }

    pub fn clear_error(&mut self) {
        self.state.write().error = None;
    }

    pub fn set_loading(&mut self, loading: bool) {
        self.state.write().is_loading = loading;
    }

    pub fn set_error(&mut self, error: Option<String>) {
        self.state.write().error = error;
    }
}

/// Create a new async action hook
pub fn use_async_action() -> AsyncAction {
    AsyncAction::new()
}
