use core::future::Future;
use core::marker::PhantomData;

use axum::http::request::Parts;

use crate::Policy;

/// Try `A` first; on rejection, try `B`. Both branches must share the same
/// `Error` type.
pub struct Any<A, B>(PhantomData<fn() -> (A, B)>);

/// Require both `A` and `B` to pass; the proof is the pair of their outputs.
pub struct All<A, B>(PhantomData<fn() -> (A, B)>);

/// Result of an [`Any<A, B>`] check.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Either<L, R> {
    Left(L),
    Right(R),
}

impl<S, A, B> Policy<S> for Any<A, B>
where
    S: Send + Sync + 'static,
    A: Policy<S>,
    B: Policy<S, Error = A::Error>,
{
    type Output = Either<A::Output, B::Output>;
    type Error = A::Error;

    fn check(
        parts: &mut Parts,
        state: &S,
    ) -> impl Future<Output = Result<Self::Output, Self::Error>> + Send {
        async move {
            match A::check(parts, state).await {
                Ok(o) => Ok(Either::Left(o)),
                Err(_) => B::check(parts, state).await.map(Either::Right),
            }
        }
    }
}

impl<S, A, B> Policy<S> for All<A, B>
where
    S: Send + Sync + 'static,
    A: Policy<S>,
    B: Policy<S, Error = A::Error>,
{
    type Output = (A::Output, B::Output);
    type Error = A::Error;

    fn check(
        parts: &mut Parts,
        state: &S,
    ) -> impl Future<Output = Result<Self::Output, Self::Error>> + Send {
        async move {
            let a = A::check(parts, state).await?;
            let b = B::check(parts, state).await?;
            Ok((a, b))
        }
    }
}
