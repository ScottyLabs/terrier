//! Type-level authorization for axum handlers.
//!
//! See `rfcs/0009-slac.md` for the design and `tests/integration.rs` for
//! end-to-end usage.

#![forbid(unsafe_code)]

mod combinators;
mod macros;
mod policy;

pub use combinators::{All, Any, Either};
pub use policy::{Authorized, Policy};

#[doc(hidden)]
pub mod __private {
    pub use axum::http::request::Parts;
}
