use std::{collections::HashSet, env};

use anyhow::{Context, ensure};
use url::Url;

pub struct AuthConfig {
    pub app_url: Url,
    pub issuer: Url,
    pub relay_url: Url,
    pub client_id: String,
    pub client_secret: String,
    pub instance_admin_subjects: HashSet<String>,
}

impl AuthConfig {
    pub fn from_env() -> anyhow::Result<Self> {
        let keycloak = required("KEYCLOAK_URL")?;
        let realm = required("KEYCLOAK_REALM")?;
        ensure!(
            !realm.contains(['/', '?', '#', '%']),
            "KEYCLOAK_REALM must be a realm name"
        );
        let app_url = endpoint(&required("APP_URL")?, "APP_URL")?;
        ensure!(
            app_url.path() == "/",
            "APP_URL must be an origin without a path"
        );
        let issuer = endpoint(
            &format!("{}/realms/{realm}", keycloak.trim_end_matches('/')),
            "Keycloak issuer",
        )?;
        let relay_url = endpoint(&required("OAUTH_RELAY_URL")?, "OAUTH_RELAY_URL")?;
        let instance_admin_subjects: HashSet<String> =
            serde_json::from_str(&required("INSTANCE_ADMIN_SUBJECTS")?)
                .context("INSTANCE_ADMIN_SUBJECTS must be a JSON string array")?;
        ensure!(
            instance_admin_subjects.iter().all(|s| !s.trim().is_empty()),
            "INSTANCE_ADMIN_SUBJECTS contains an empty subject"
        );
        Ok(Self {
            app_url,
            issuer,
            relay_url,
            client_id: required("OIDC_CLIENT_ID")?,
            client_secret: required("OIDC_CLIENT_SECRET")?,
            instance_admin_subjects,
        })
    }

    pub fn secure_cookies(&self) -> bool {
        self.app_url.scheme() == "https"
    }
}

fn required(name: &str) -> anyhow::Result<String> {
    let value = env::var(name).with_context(|| format!("{name} must be set"))?;
    ensure!(!value.trim().is_empty(), "{name} must not be empty");
    Ok(value)
}

fn endpoint(value: &str, name: &str) -> anyhow::Result<Url> {
    let url = Url::parse(value).with_context(|| format!("{name} must be an absolute URL"))?;
    let local = matches!(url.host_str(), Some("localhost" | "127.0.0.1" | "[::1]"));
    ensure!(
        url.scheme() == "https" || (url.scheme() == "http" && local),
        "{name} requires HTTPS except on loopback hosts"
    );
    ensure!(
        url.host_str().is_some()
            && url.username().is_empty()
            && url.password().is_none()
            && url.query().is_none()
            && url.fragment().is_none(),
        "{name} must not contain credentials, a query, or a fragment"
    );
    Ok(url)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn deployment_urls_require_https_except_on_loopback() {
        for url in [
            "http://terrier.example",
            "https://user:secret@terrier.example",
            "https://terrier.example?query=value",
            "https://terrier.example#fragment",
        ] {
            assert!(endpoint(url, "test URL").is_err(), "{url}");
        }
        for url in [
            "https://terrier.example",
            "http://localhost:5173",
            "http://127.0.0.1:8080",
            "http://[::1]:3000",
        ] {
            assert!(endpoint(url, "test URL").is_ok(), "{url}");
        }
    }
}
