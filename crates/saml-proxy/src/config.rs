use anyhow::{Context, Result};

pub struct Config {
    pub base_url: String,
    pub entity_id: String,
    pub idp_cert_path: String,
    pub idp_key_path: String,
    pub host: String,
    pub port: u16,
}

impl Config {
    pub fn from_env() -> Result<Self> {
        let base_url = std::env::var("BASE_URL").context("BASE_URL must be set")?;
        let entity_id = std::env::var("ENTITY_ID").context("ENTITY_ID must be set")?;
        let idp_cert_path = std::env::var("IDP_CERT_PATH").context("IDP_CERT_PATH must be set")?;
        let idp_key_path = std::env::var("IDP_KEY_PATH").context("IDP_KEY_PATH must be set")?;

        let host = std::env::var("HOST").unwrap_or_else(|_| "0.0.0.0".into());
        let port = std::env::var("PORT")
            .unwrap_or_else(|_| "8443".into())
            .parse::<u16>()
            .context("PORT must be a valid u16")?;

        Ok(Self {
            base_url,
            entity_id,
            idp_cert_path,
            idp_key_path,
            host,
            port,
        })
    }
}
