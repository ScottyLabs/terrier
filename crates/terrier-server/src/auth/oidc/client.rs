use std::sync::Arc;

use anyhow::Context;
use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};
use openidconnect::{
    AccessTokenHash, AuthorizationCode, ClaimsVerificationError, ClientId, ClientSecret, CsrfToken,
    EndpointMaybeSet, EndpointNotSet, EndpointSet, IssuerUrl, Nonce, OAuth2TokenResponse,
    PkceCodeChallenge, PkceCodeVerifier, RedirectUrl, RefreshToken, RequestTokenError, Scope,
    SubjectIdentifier, TokenResponse,
    core::{
        CoreAuthenticationFlow, CoreClient, CoreErrorResponseType, CoreIdToken, CoreIdTokenClaims,
        CoreProviderMetadata, CoreTokenResponse,
    },
    reqwest,
};
use serde::{Deserialize, Serialize};
use tokio::sync::RwLock;
use tower_sessions::{Session, cookie::time::OffsetDateTime};
use url::Url;

use super::{CSRF, internal, session_error};
use crate::{
    auth::{config::AuthConfig, session::OIDC_KEY},
    error::ApiError,
};

type ProviderClient = CoreClient<
    EndpointSet,
    EndpointNotSet,
    EndpointNotSet,
    EndpointNotSet,
    EndpointMaybeSet,
    EndpointMaybeSet,
>;
pub(super) const PENDING: &str = "oidc_pending";

#[derive(Clone)]
pub struct OidcClient {
    provider: Arc<RwLock<ProviderClient>>,
    http: reqwest::Client,
    config: Arc<AuthConfig>,
}

#[derive(Serialize, Deserialize)]
struct PendingLogin {
    state: CsrfToken,
    nonce: Nonce,
    verifier: PkceCodeVerifier,
    created_at: i64,
}

#[derive(Serialize, Deserialize)]
struct Credentials {
    id_token: CoreIdToken,
    refresh_token: Option<RefreshToken>,
    nonce: Nonce,
    subject: SubjectIdentifier,
    auth_time: Option<i64>,
}

#[derive(Serialize)]
struct RelayState<'a> {
    return_to: &'a str,
    csrf: &'a str,
}

impl OidcClient {
    pub async fn discover(config: &AuthConfig) -> anyhow::Result<Self> {
        let http = reqwest::Client::builder()
            .redirect(reqwest::redirect::Policy::none())
            .timeout(std::time::Duration::from_secs(10))
            .build()?;
        let provider = discover(config, &http, IssuerUrl::new(config.issuer.to_string())?).await?;
        Ok(Self {
            provider: Arc::new(RwLock::new(provider)),
            http,
            config: Arc::new(config.clone()),
        })
    }

    pub async fn rediscover(&self, issuer: IssuerUrl) -> anyhow::Result<()> {
        let provider = discover(&self.config, &self.http, issuer).await?;
        *self.provider.write().await = provider;
        Ok(())
    }

    pub(super) async fn begin(&self, session: &Session) -> Result<Url, ApiError> {
        let callback = self
            .config
            .app_url
            .join("api/auth/callback")
            .map_err(|_| internal("invalid OIDC callback URL"))?;
        let random = CsrfToken::new_random();
        let state = if self.config.relay_url.is_some() {
            CsrfToken::new(
                URL_SAFE_NO_PAD.encode(
                    serde_json::to_vec(&RelayState {
                        return_to: callback.as_str(),
                        csrf: random.secret(),
                    })
                    .map_err(|_| internal("failed to encode relay state"))?,
                ),
            )
        } else {
            random
        };
        let (challenge, verifier) = PkceCodeChallenge::new_random_sha256();
        let (url, state, nonce) = self
            .provider
            .read()
            .await
            .authorize_url(
                CoreAuthenticationFlow::AuthorizationCode,
                || state,
                Nonce::new_random,
            )
            .add_scope(Scope::new("email".into()))
            .add_scope(Scope::new("profile".into()))
            .set_pkce_challenge(challenge)
            .url();
        session
            .insert(
                PENDING,
                PendingLogin {
                    state,
                    nonce,
                    verifier,
                    created_at: OffsetDateTime::now_utc().unix_timestamp(),
                },
            )
            .await
            .map_err(session_error)?;
        session.save().await.map_err(session_error)?;
        Ok(url)
    }

    pub(super) async fn complete(
        &self,
        session: &Session,
        state: String,
        code: Option<String>,
        error: Option<String>,
    ) -> Result<(), ApiError> {
        let pending: PendingLogin = session
            .get(PENDING)
            .await
            .map_err(session_error)?
            .ok_or_else(invalid_callback)?;
        let age = OffsetDateTime::now_utc().unix_timestamp() - pending.created_at;
        if pending.state != CsrfToken::new(state) || !(0..600).contains(&age) {
            return Err(invalid_callback());
        }
        // Consume the attempt before contacting the provider, including denied logins.
        session.remove_value(PENDING).await.map_err(session_error)?;
        session.save().await.map_err(session_error)?;
        if error.is_some() || code.as_ref().is_none_or(|code| code.is_empty()) {
            return Err(invalid_callback());
        }
        let provider = self.provider.read().await.clone();
        let response = provider
            .exchange_code(AuthorizationCode::new(code.ok_or_else(invalid_callback)?))
            .map_err(|_| internal("OIDC token endpoint is missing"))?
            .set_pkce_verifier(pending.verifier)
            .request_async(&self.http)
            .await
            .map_err(|error| match error {
                RequestTokenError::ServerResponse(ref response)
                    if *response.error() == CoreErrorResponseType::InvalidGrant =>
                {
                    invalid_callback()
                }
                _ => internal("OIDC authorization code exchange failed"),
            })?;
        let id_token = response
            .id_token()
            .ok_or_else(|| internal("OIDC ID token is missing"))?;
        let claims = id_token
            .claims(&provider.id_token_verifier(), &pending.nonce)
            .map_err(|_| invalid_callback())?;
        validate_access_token(&provider, &response).map_err(|_| invalid_callback())?;
        session.cycle_id().await.map_err(session_error)?;
        session
            .insert(
                OIDC_KEY,
                Credentials {
                    id_token: id_token.clone(),
                    refresh_token: response.refresh_token().cloned(),
                    nonce: pending.nonce,
                    subject: claims.subject().clone(),
                    auth_time: claims.auth_time().map(|time| time.timestamp()),
                },
            )
            .await
            .map_err(session_error)?;
        session
            .insert(CSRF, uuid::Uuid::new_v4().to_string())
            .await
            .map_err(session_error)?;
        session.save().await.map_err(session_error)?;
        Ok(())
    }

    pub(super) async fn claims(
        &self,
        session: &Session,
    ) -> Result<Option<CoreIdTokenClaims>, ApiError> {
        let Some(mut credentials): Option<Credentials> =
            session.get(OIDC_KEY).await.map_err(session_error)?
        else {
            return Ok(None);
        };
        let provider = self.provider.read().await.clone();
        match credentials
            .id_token
            .claims(&provider.id_token_verifier(), |nonce: Option<&Nonce>| {
                verify_refresh_nonce(nonce, &credentials.nonce)
            }) {
            Ok(claims) => return Ok(Some(claims.clone())),
            Err(ClaimsVerificationError::Expired(_)) => {}
            Err(_) => return Err(internal("stored OIDC ID token validation failed")),
        }
        let Some(refresh_token) = credentials.refresh_token.as_ref() else {
            clear_credentials(session).await?;
            return Ok(None);
        };
        let response = match provider
            .exchange_refresh_token(refresh_token)
            .map_err(|_| internal("OIDC token endpoint is missing"))?
            .request_async(&self.http)
            .await
        {
            Ok(response) => response,
            Err(RequestTokenError::ServerResponse(error))
                if *error.error() == CoreErrorResponseType::InvalidGrant
                    || error.error().as_ref() == "JwtToken" =>
            {
                clear_credentials(session).await?;
                return Ok(None);
            }
            Err(_) => return Err(internal("OIDC token refresh failed")),
        };
        let id_token = response
            .id_token()
            .ok_or_else(|| internal("refreshed OIDC ID token is missing"))?;
        let claims = id_token
            .claims(&provider.id_token_verifier(), |nonce: Option<&Nonce>| {
                verify_refresh_nonce(nonce, &credentials.nonce)
            })
            .map_err(|_| internal("refreshed OIDC ID token validation failed"))?
            .clone();
        if claims.subject() != &credentials.subject
            || claims.auth_time().map(|time| time.timestamp()) != credentials.auth_time
        {
            return Err(internal("OIDC refresh changed the authenticated identity"));
        }
        validate_access_token(&provider, &response)
            .map_err(|_| internal("refreshed OIDC access token hash is invalid"))?;
        credentials.id_token = id_token.clone();
        if let Some(refresh_token) = response.refresh_token() {
            credentials.refresh_token = Some(refresh_token.clone());
        }
        // Persist rotated tokens before an authenticated handler runs.
        session
            .insert(OIDC_KEY, credentials)
            .await
            .map_err(session_error)?;
        session.save().await.map_err(session_error)?;
        Ok(Some(claims))
    }
}

async fn discover(
    config: &AuthConfig,
    http: &reqwest::Client,
    issuer: IssuerUrl,
) -> anyhow::Result<ProviderClient> {
    let metadata = CoreProviderMetadata::discover_async(issuer, http)
        .await
        .context("failed to discover Keycloak OIDC configuration")?;
    anyhow::ensure!(
        metadata.token_endpoint().is_some(),
        "OIDC token endpoint is missing"
    );
    let redirect = config
        .relay_url
        .clone()
        .unwrap_or(config.app_url.join("api/auth/callback")?);
    Ok(CoreClient::from_provider_metadata(
        metadata,
        ClientId::new(config.client_id.clone()),
        Some(ClientSecret::new(config.client_secret.clone())),
    )
    .set_redirect_uri(RedirectUrl::new(redirect.to_string())?))
}

fn validate_access_token(
    provider: &ProviderClient,
    response: &CoreTokenResponse,
) -> Result<(), ()> {
    let token = response.id_token().ok_or(())?;
    let verifier = provider.id_token_verifier();
    let claims = token
        .claims(&verifier, |_: Option<&Nonce>| Ok(()))
        .map_err(|_| ())?;
    if let Some(expected) = claims.access_token_hash() {
        let actual = AccessTokenHash::from_token(
            response.access_token(),
            token.signing_alg().map_err(|_| ())?,
            token.signing_key(&verifier).map_err(|_| ())?,
        )
        .map_err(|_| ())?;
        if actual != *expected {
            return Err(());
        }
    }
    Ok(())
}

pub(super) async fn clear_credentials(session: &Session) -> Result<(), ApiError> {
    session
        .remove_value(OIDC_KEY)
        .await
        .map_err(session_error)?;
    session.remove_value(CSRF).await.map_err(session_error)?;
    session.save().await.map_err(session_error)
}

fn invalid_callback() -> ApiError {
    ApiError::BadRequest("Invalid or expired login response. Start sign-in again.".into())
}

fn verify_refresh_nonce(nonce: Option<&Nonce>, expected: &Nonce) -> Result<(), String> {
    // A refresh may omit the nonce; if supplied it must match the original login.
    if nonce.is_none_or(|nonce| nonce == expected) {
        Ok(())
    } else {
        Err("refreshed nonce differs from the original login".into())
    }
}
