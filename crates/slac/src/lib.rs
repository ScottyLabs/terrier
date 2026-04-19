//! Type-level authorization for axum handlers.
//!
//! See `rfcs/0009-slac.md` for the design and `tests/integration.rs` for
//! end-to-end usage.

#![forbid(unsafe_code)]
// Trait impls in this crate must spell out `+ Send` on the returned future to
// satisfy the trait bound; rewriting them as `async fn` drops the bound.
#![allow(clippy::manual_async_fn)]

mod combinators;
mod macros;
mod policy;

pub use combinators::{All, Any, Either};
pub use policy::{Authorized, Policy};

#[doc(hidden)]
pub mod __private {
    pub use axum::http::request::Parts;
}
