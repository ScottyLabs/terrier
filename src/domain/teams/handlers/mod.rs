// Handler modules will be populated as we split teams.rs
pub mod crud;
pub mod invitations;
pub mod join_requests;
pub mod membership;
pub mod query;

// Re-export all public items from each module
pub use crud::*;
pub use invitations::*;
pub use join_requests::*;
pub use membership::*;
pub use query::*;
