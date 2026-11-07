use std::{future::Future, pin::Pin};

use anyhow::Context;
use axum::{
    extract::{FromRequestParts, Request, State},
    http::request::Parts,
    middleware::Next,
    response::Response,
};
use sea_orm::{
    ConnectionTrait, DatabaseConnection, DatabaseTransaction, DbBackend, Statement,
    TransactionTrait,
};
use uuid::Uuid;

use crate::{auth::current_user::CurrentUser, error::ApiError, state::AppState};

type WriteFuture<'a, T> = Pin<Box<dyn Future<Output = Result<T, ApiError>> + Send + 'a>>;

/// Parent lock acquired before the callback. The exclusive lock also permits checked restoration.
#[derive(Clone, Copy)]
pub enum Scope {
    Global,
    Hackathon(Uuid),
    HackathonExclusive(Uuid),
}

impl Scope {
    fn id(self) -> Option<Uuid> {
        match self {
            Self::Global => None,
            Self::Hackathon(id) | Self::HackathonExclusive(id) => Some(id),
        }
    }
}

/// An authenticated actor for a domain write. Policies still authorize each resource.
#[derive(Clone)]
pub struct Audit {
    actor: Uuid,
    db: DatabaseConnection,
}

pub async fn context(
    user: CurrentUser,
    State(state): State<AppState>,
    mut request: Request,
    next: Next,
) -> Response {
    if !request.method().is_safe() {
        request.extensions_mut().insert(Audit {
            actor: user.user().id,
            db: state.db,
        });
    }
    next.run(request).await
}

impl<S: Send + Sync> FromRequestParts<S> for Audit {
    type Rejection = ApiError;

    async fn from_request_parts(parts: &mut Parts, _: &S) -> Result<Self, Self::Rejection> {
        parts.extensions.get::<Self>().cloned().ok_or_else(|| {
            ApiError::Internal(anyhow::anyhow!(
                "Audit requires authenticated write middleware"
            ))
        })
    }
}

impl Audit {
    /// Commits the callback's changes and trigger-generated audit rows together.
    /// Lock application and team rows in RFC 0008 order before dependent writes.
    #[doc(hidden)]
    pub async fn write<T: Send>(
        &self,
        scope: Scope,
        reason: Option<&str>,
        callback: impl for<'a> FnOnce(&'a DatabaseTransaction) -> WriteFuture<'a, T> + Send,
    ) -> Result<T, ApiError> {
        transaction(&self.db, Some(self.actor), scope, reason, callback).await
    }
}

/// Runs an automatic domain change with a required reason and no human actor.
#[doc(hidden)]
pub async fn automatic<T: Send>(
    db: &DatabaseConnection,
    scope: Scope,
    reason: &str,
    callback: impl for<'a> FnOnce(&'a DatabaseTransaction) -> WriteFuture<'a, T> + Send,
) -> Result<T, ApiError> {
    if reason.trim().is_empty() {
        return Err(ApiError::Internal(anyhow::anyhow!(
            "Automatic audit reason is empty"
        )));
    }
    transaction(db, None, scope, Some(reason), callback).await
}

async fn transaction<T: Send>(
    db: &DatabaseConnection,
    actor: Option<Uuid>,
    scope: Scope,
    reason: Option<&str>,
    callback: impl for<'a> FnOnce(&'a DatabaseTransaction) -> WriteFuture<'a, T> + Send,
) -> Result<T, ApiError> {
    let tx = db
        .begin()
        .await
        .context("begin audited transaction")
        .map_err(ApiError::Internal)?;
    tx.execute(Statement::from_sql_and_values(
        DbBackend::Postgres,
        "SELECT set_config('terrier.actor_user_id', $1, true),
                set_config('terrier.hackathon_id', $2, true),
                set_config('terrier.audit_reason', $3, true),
                set_config('terrier.automatic', $4, true),
                set_config('terrier.management', $5, true)",
        [
            actor.map(|id| id.to_string()).unwrap_or_default().into(),
            scope
                .id()
                .map(|id| id.to_string())
                .unwrap_or_default()
                .into(),
            reason.unwrap_or_default().to_owned().into(),
            (if actor.is_none() { "true" } else { "false" }).into(),
            (if matches!(scope, Scope::HackathonExclusive(_)) {
                "true"
            } else {
                "false"
            })
            .into(),
        ],
    ))
    .await
    .context("set audit transaction context")
    .map_err(ApiError::Internal)?;
    if let Some(id) = scope.id() {
        let parent = tx
            .query_one(Statement::from_sql_and_values(
                DbBackend::Postgres,
                match scope {
                    Scope::HackathonExclusive(_) => {
                        "SELECT id FROM hackathon WHERE id=$1 FOR UPDATE"
                    }
                    _ => "SELECT id FROM hackathon WHERE id=$1 AND is_active FOR SHARE",
                },
                [id.into()],
            ))
            .await
            .context("lock audited hackathon")
            .map_err(ApiError::Internal)?;
        if parent.is_none() {
            return Err(ApiError::NotFound);
        }
    }
    let result = match callback(&tx).await {
        Ok(result) => result,
        Err(error) => {
            if let Err(rollback) = tx.rollback().await {
                return Err(ApiError::Internal(anyhow::anyhow!(
                    "Rollback failed after {error:?}: {rollback}"
                )));
            }
            return Err(error);
        }
    };
    tx.commit()
        .await
        .context("commit audited transaction")
        .map_err(ApiError::Internal)?;
    Ok(result)
}

/// Executes writes and their audit records in one transaction.
#[macro_export]
macro_rules! audited {
    ($audit:expr, $scope:expr, |$db:ident| $body:block) => {
        ($audit).write($scope, None, |$db| ::std::boxed::Box::pin(async move $body))
    };
    ($audit:expr, $scope:expr, reason = $reason:expr, |$db:ident| $body:block) => {
        ($audit).write($scope, Some($reason), |$db| ::std::boxed::Box::pin(async move $body))
    };
}

/// Executes an automatic change with a required explanation.
#[macro_export]
macro_rules! audited_automatic {
    ($db:expr, $scope:expr, $reason:expr, |$transaction:ident| $body:block) => {
        $crate::audit::automatic($db, $scope, $reason, |$transaction| {
            ::std::boxed::Box::pin(async move $body)
        })
    };
}
