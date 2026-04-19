use core::future::Future;
use core::marker::PhantomData;

use axum::extract::FromRequestParts;
use axum::http::request::Parts;
use axum::response::IntoResponse;

/// A unit of authorization. `check` runs against the request parts and app
/// state, producing the data an authorized caller is entitled to or rejecting
/// with an HTTP-shaped error.
///
/// `check` is an associated function (no `&self`), so a policy is identified
/// purely by its type. This is what gives the type parameter on `Authorized<P>`
/// its meaning.
pub trait Policy<S>: Sized + Send + Sync + 'static {
    type Output: Send + Sync + 'static;
    type Error: IntoResponse + Send + 'static;

    fn check(
        parts: &mut Parts,
        state: &S,
    ) -> impl Future<Output = Result<Self::Output, Self::Error>> + Send;
}

/// Witness that policy `P` ran successfully against the current request.
///
/// Constructible only via the [`FromRequestParts`] impl, which calls
/// `P::check`. The `S = ()` default keeps stateless examples short; stateful
/// consumers should declare a project-local alias next to their `AppState`
/// (e.g. `pub type Auth<P> = slac::Authorized<P, AppState>;`) so handler
/// signatures stay readable.
pub struct Authorized<P, S = ()>
where
    P: Policy<S>,
{
    pub data: P::Output,
    _proof: PhantomData<fn() -> (P, S)>,
}

impl<P, S> FromRequestParts<S> for Authorized<P, S>
where
    P: Policy<S>,
    S: Send + Sync,
{
    type Rejection = P::Error;

    fn from_request_parts(
        parts: &mut Parts,
        state: &S,
    ) -> impl Future<Output = Result<Self, Self::Rejection>> + Send {
        async move {
            let data = P::check(parts, state).await?;
            Ok(Self {
                data,
                _proof: PhantomData,
            })
        }
    }
}
