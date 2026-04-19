/// Builds a sum-type [`Policy`](crate::Policy) from several existing policies.
///
/// Variants are tried in source order and the first to succeed is taken; if
/// all reject, the last error is returned. All variants must share the same
/// `Error` associated type.
///
/// # Syntax
///
/// ```ignore
/// policy! {
///     pub enum SettingsAccess for AppState {
///         GlobalAdmin    = IsGlobalAdmin,
///         HackathonAdmin = IsHackathonAdmin,
///     }
/// }
/// ```
#[macro_export]
macro_rules! policy {
    (
        $(#[$attr:meta])*
        $vis:vis enum $name:ident for $state:ty {
            $first_variant:ident = $first_policy:ty
            $(, $variant:ident = $policy:ty)* $(,)?
        }
    ) => {
        $(#[$attr])*
        $vis enum $name {
            $first_variant(<$first_policy as $crate::Policy<$state>>::Output),
            $(
                $variant(<$policy as $crate::Policy<$state>>::Output),
            )*
        }

        impl $crate::Policy<$state> for $name
        where
            $(
                $policy: $crate::Policy<
                    $state,
                    Error = <$first_policy as $crate::Policy<$state>>::Error,
                >,
            )*
        {
            type Output = Self;
            type Error = <$first_policy as $crate::Policy<$state>>::Error;

            fn check(
                parts: &mut $crate::__private::Parts,
                state: &$state,
            ) -> impl ::core::future::Future<
                Output = ::core::result::Result<Self::Output, Self::Error>,
            > + ::core::marker::Send {
                async move {
                    let mut last_err: ::core::option::Option<Self::Error> =
                        ::core::option::Option::None;

                    match <$first_policy as $crate::Policy<$state>>::check(parts, state).await {
                        ::core::result::Result::Ok(o) => {
                            return ::core::result::Result::Ok(Self::$first_variant(o));
                        }
                        ::core::result::Result::Err(e) => {
                            last_err = ::core::option::Option::Some(e);
                        }
                    }

                    $(
                        match <$policy as $crate::Policy<$state>>::check(parts, state).await {
                            ::core::result::Result::Ok(o) => {
                                return ::core::result::Result::Ok(Self::$variant(o));
                            }
                            ::core::result::Result::Err(e) => {
                                last_err = ::core::option::Option::Some(e);
                            }
                        }
                    )*

                    ::core::result::Result::Err(last_err.unwrap())
                }
            }
        }
    };
}
