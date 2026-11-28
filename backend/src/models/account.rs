use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// An account. Every account sees the same inventory; admins also manage
/// accounts and whether new ones can register.
#[derive(Debug, Clone, Serialize)]
pub struct User {
    pub id: Uuid,
    pub username: String,
    pub is_admin: bool,
    pub created_at: DateTime<Utc>,
}

/// What the login page needs before anyone is signed in.
#[derive(Debug, Serialize)]
pub struct AuthStatus {
    /// No accounts yet: the next registration creates the admin.
    pub setup_needed: bool,
    pub registration_enabled: bool,
    /// The signed-in account, if any.
    pub user: Option<User>,
}

#[derive(Debug, Deserialize)]
pub struct Credentials {
    pub username: String,
    pub password: String,
}

#[derive(Debug, Deserialize)]
pub struct ChangePassword {
    pub current_password: String,
    pub new_password: String,
}

#[derive(Debug, Deserialize)]
pub struct SetPassword {
    pub password: String,
}

#[derive(Debug, Deserialize)]
pub struct UpdateUser {
    pub is_admin: bool,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct AdminSettings {
    pub registration_enabled: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct ApiToken {
    pub id: Uuid,
    pub name: String,
    pub created_at: DateTime<Utc>,
    pub last_used_at: Option<DateTime<Utc>>,
}

/// A new token, with its secret. The secret is only ever returned here.
#[derive(Debug, Serialize)]
pub struct NewApiToken {
    #[serde(flatten)]
    pub token: ApiToken,
    pub secret: String,
}

#[derive(Debug, Deserialize)]
pub struct CreateApiToken {
    pub name: String,
}
