#[cfg(feature = "server")]
pub mod auth;
pub mod errors;
#[cfg(feature = "server")]
pub mod types;

#[cfg(feature = "server")]
pub mod database;

// Re-export commonly used items
pub use errors::{OptionExt, ResultExt};
#[cfg(feature = "server")]
pub use types::*;
