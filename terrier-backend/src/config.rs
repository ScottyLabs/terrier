#[derive(Clone, Debug)]
pub struct Config {
    pub app_url: String,
    pub api_url: String,
    pub database_url: String,
    pub oidc_issuer: String,
    pub oidc_client_id: String,
    pub oidc_client_secret: String,
    pub admin_emails: Vec<String>,
    pub s3_endpoint: String,
    pub s3_public_endpoint: String,
    pub s3_access_key: String,
    pub s3_secret_key: String,
    pub s3_bucket_name: String,
    pub s3_region: String,
}

impl Config {
    pub fn from_env() -> Result<Self, Box<dyn std::error::Error>> {
        dotenvy::dotenv().ok();

        let admin_emails = dotenvy::var("ADMIN_EMAILS")
            .unwrap_or_default()
            .split(',')
            .map(|s| s.trim().to_lowercase().to_string())
            .filter(|s| !s.is_empty())
            .collect();

        let s3_endpoint =
            dotenvy::var("S3_ENDPOINT").unwrap_or_else(|_| "http://localhost:9000".to_string());
        let s3_public_endpoint =
            dotenvy::var("S3_PUBLIC_ENDPOINT").unwrap_or_else(|_| s3_endpoint.clone());

        Ok(Config {
            app_url: dotenvy::var("APP_URL")?,
            api_url: dotenvy::var("API_URL")?,
            database_url: dotenvy::var("DATABASE_URL")?,
            oidc_issuer: dotenvy::var("OIDC_ISSUER")?,
            oidc_client_id: dotenvy::var("OIDC_CLIENT_ID")?,
            oidc_client_secret: dotenvy::var("OIDC_CLIENT_SECRET")?,
            admin_emails,
            s3_endpoint,
            s3_public_endpoint,
            s3_access_key: dotenvy::var("S3_ACCESS_KEY")
                .unwrap_or_else(|_| "minioadmin".to_string()),
            s3_secret_key: dotenvy::var("S3_SECRET_KEY")
                .unwrap_or_else(|_| "minioadmin".to_string()),
            s3_bucket_name: dotenvy::var("S3_BUCKET_NAME")
                .unwrap_or_else(|_| "terrier-files".to_string()),
            s3_region: dotenvy::var("S3_REGION").unwrap_or_else(|_| "us-east-1".to_string()),
        })
    }
}
