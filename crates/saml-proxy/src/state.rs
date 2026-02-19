use crate::config::Config;
use crate::discovery::federation_index::FederationIndex;
use crate::session::SessionStore;
use anyhow::{Context, Result};
use saml_mdq::{MdqCache, MdqClient};
use std::time::Duration;

const MDQ_BASE_URL: &str = "https://mdq.incommon.org";
const MDQ_SIGNING_CERT_PEM: &[u8] = include_bytes!("../certs/inc-md-cert-mdq.pem");

pub struct AppState {
    pub config: Config,
    pub sessions: SessionStore,
    pub mdq_client: MdqClient,
    pub federation_index: FederationIndex,
    pub idp_key_der: Vec<u8>,
    pub idp_cert_der: Vec<u8>,
}

impl AppState {
    pub fn new(config: Config) -> Result<Self> {
        let cert_pem =
            std::fs::read(&config.idp_cert_path).context("failed to read IDP certificate")?;
        let key_pem =
            std::fs::read(&config.idp_key_path).context("failed to read IDP private key")?;

        let idp_cert =
            openssl::x509::X509::from_pem(&cert_pem).context("failed to parse IDP certificate")?;
        let idp_key = openssl::pkey::PKey::private_key_from_pem(&key_pem)
            .context("failed to parse IDP private key")?;
        let idp_cert_der = idp_cert
            .to_der()
            .context("failed to encode certificate as DER")?;
        let idp_key_der = idp_key
            .rsa()
            .context("IDP key must be RSA")?
            .private_key_to_der()
            .context("failed to encode private key as DER")?;

        let mdq_signing_cert = openssl::x509::X509::from_pem(MDQ_SIGNING_CERT_PEM)
            .context("failed to parse MDQ signing certificate")?;
        let mdq_signing_cert_der = mdq_signing_cert
            .to_der()
            .context("failed to encode MDQ signing certificate as DER")?;

        let cache = MdqCache::new(1000, Duration::from_secs(3600));
        let mdq_client = MdqClient::builder(MDQ_BASE_URL)
            .cache(cache)
            .signing_cert(mdq_signing_cert_der)
            .build()
            .map_err(|e| anyhow::anyhow!(e))?;

        Ok(Self {
            config,
            sessions: SessionStore::new(),
            mdq_client,
            federation_index: FederationIndex::new(),
            idp_key_der,
            idp_cert_der,
        })
    }
}
