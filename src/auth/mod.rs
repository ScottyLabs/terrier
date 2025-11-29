#[cfg(feature = "server")]
pub mod extractors;
#[cfg(feature = "server")]
pub mod handlers;
#[cfg(feature = "server")]
pub mod middleware;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct UserInfo {
    pub id: String,
    pub email: String,
    pub name: Option<String>,
    pub picture: Option<String>,
    pub is_admin: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoginQuery {
    pub redirect_uri: Option<String>,
}
