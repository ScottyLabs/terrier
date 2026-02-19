use anyhow::{Context, Result};

pub struct Config {
    pub base_url: String,
    pub entity_id: String,
    pub idp_cert_path: String,
    pub idp_key_path: String,
    pub mdq_signing_cert_path: String,
    pub host: String,
    pub port: u16,
}

impl Config {
    pub fn from_env() -> Result<Self> {
        let base_url =
            std::env::var("SAML_PROXY_BASE_URL").context("SAML_PROXY_BASE_URL must be set")?;
        let entity_id =
            std::env::var("SAML_PROXY_ENTITY_ID").context("SAML_PROXY_ENTITY_ID must be set")?;
        let idp_cert_path = std::env::var("SAML_PROXY_IDP_CERT_PATH")
            .context("SAML_PROXY_IDP_CERT_PATH must be set")?;
        let idp_key_path = std::env::var("SAML_PROXY_IDP_KEY_PATH")
            .context("SAML_PROXY_IDP_KEY_PATH must be set")?;

        let mdq_signing_cert_path = std::env::var("SAML_PROXY_MDQ_SIGNING_CERT_PATH")
            .unwrap_or_else(|_| "certs/incommon-mdq.pem".into());

        let host = std::env::var("SAML_PROXY_HOST").unwrap_or_else(|_| "0.0.0.0".into());
        let port = std::env::var("SAML_PROXY_PORT")
            .unwrap_or_else(|_| "8443".into())
            .parse::<u16>()
            .context("SAML_PROXY_PORT must be a valid u16")?;

        Ok(Self {
            base_url,
            entity_id,
            idp_cert_path,
            idp_key_path,
            mdq_signing_cert_path,
            host,
            port,
        })
    }
}
