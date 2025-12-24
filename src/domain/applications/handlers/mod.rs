// Handler modules will be populated as we split applications.rs
pub mod attendance;
pub mod files;
pub mod query;
pub mod review;
pub mod submission;

// Re-export all public items from each module
pub use attendance::*;
pub use files::*;
pub use query::*;
pub use review::*;
pub use submission::*;
