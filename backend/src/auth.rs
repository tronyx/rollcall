//! Sign-in: password hashing, session cookies, API tokens, and the
//! middleware that requires one of them on every protected API route.

use crate::{
    AppState,
    error::{AppError, AppResult},
    models::User,
    repo::accounts,
};
use argon2::{Argon2, PasswordHash, PasswordHasher, PasswordVerifier, password_hash::SaltString};
use axum::{
    extract::{FromRequestParts, Request, State},
    http::{HeaderMap, HeaderValue, header, request::Parts},
    middleware::Next,
    response::Response,
};
use chrono::{DateTime, Duration, Utc};
use rand_core::{OsRng, RngCore};
use sha2::{Digest, Sha256};
use std::sync::OnceLock;

pub const SESSION_COOKIE: &str = "rollcall_session";
/// A session lasts this long after it was last used.
pub const SESSION_DAYS: i64 = 30;
/// API tokens start with this, so they're easy to recognize.
pub const TOKEN_PREFIX: &str = "rc_";

pub fn session_expiry() -> DateTime<Utc> {
    Utc::now() + Duration::days(SESSION_DAYS)
}

/// 32 random bytes as hex.
pub fn new_secret() -> String {
    let mut bytes = [0u8; 32];
    OsRng.fill_bytes(&mut bytes);
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

/// Sessions and tokens are stored as the SHA-256 of their secret.
pub fn hash_secret(secret: &str) -> Vec<u8> {
    Sha256::digest(secret.as_bytes()).to_vec()
}

pub fn check_username(username: &str) -> AppResult<String> {
    let u = username.trim();
    let ok = (1..=40).contains(&u.len())
        && u.chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '_' | '-'));
    if !ok {
        return Err(AppError::BadRequest(
            "Use 1 to 40 letters, numbers, dots, dashes or underscores for the username.".into(),
        ));
    }
    Ok(u.to_string())
}

pub fn check_password(password: &str) -> AppResult<()> {
    let n = password.chars().count();
    if !(8..=200).contains(&n) {
        return Err(AppError::BadRequest(
            "Use a password of at least 8 characters.".into(),
        ));
    }
    Ok(())
}

/// Argon2id with a random salt. Runs off the async threads, since it's slow on purpose.
pub async fn hash_password(password: &str) -> AppResult<String> {
    let password = password.to_owned();
    tokio::task::spawn_blocking(move || {
        let mut salt = [0u8; 16];
        OsRng.fill_bytes(&mut salt);
        let salt = SaltString::encode_b64(&salt).map_err(|e| anyhow::anyhow!("salt: {e}"))?;
        Argon2::default()
            .hash_password(password.as_bytes(), &salt)
            .map(|h| h.to_string())
            .map_err(|e| AppError::Internal(anyhow::anyhow!("hashing a password: {e}")))
    })
    .await
    .map_err(|e| AppError::Internal(e.into()))?
}

/// Checks a password against a stored hash. With no hash (an unknown
/// username) it checks against a throwaway one, so both cases take as long.
pub async fn verify_password(password: &str, hash: Option<String>) -> bool {
    let password = password.to_owned();
    tokio::task::spawn_blocking(move || {
        let found = hash.is_some();
        let hash = hash.unwrap_or_else(|| dummy_hash().to_owned());
        let Ok(parsed) = PasswordHash::new(&hash) else {
            return false;
        };
        let ok = Argon2::default()
            .verify_password(password.as_bytes(), &parsed)
            .is_ok();
        found && ok
    })
    .await
    .unwrap_or(false)
}

fn dummy_hash() -> &'static str {
    static HASH: OnceLock<String> = OnceLock::new();
    HASH.get_or_init(|| {
        let salt = SaltString::encode_b64(&[7u8; 16]).expect("static salt");
        Argon2::default()
            .hash_password(b"not a real password", &salt)
            .expect("hashing the dummy password")
            .to_string()
    })
}

/// The session cookie's token, if the request has one.
pub fn session_token(headers: &HeaderMap) -> Option<&str> {
    headers
        .get_all(header::COOKIE)
        .iter()
        .filter_map(|v| v.to_str().ok())
        .flat_map(|v| v.split(';'))
        .filter_map(|pair| pair.trim().split_once('='))
        .find(|(k, _)| *k == SESSION_COOKIE)
        .map(|(_, v)| v)
}

fn bearer_token(headers: &HeaderMap) -> Option<&str> {
    headers
        .get(header::AUTHORIZATION)?
        .to_str()
        .ok()?
        .strip_prefix("Bearer ")
        .map(str::trim)
}

/// Behind an HTTPS reverse proxy the cookie is marked Secure.
fn is_https(headers: &HeaderMap) -> bool {
    headers
        .get("x-forwarded-proto")
        .and_then(|v| v.to_str().ok())
        .is_some_and(|v| v.eq_ignore_ascii_case("https"))
}

pub fn session_cookie(token: &str, headers: &HeaderMap) -> HeaderValue {
    let secure = if is_https(headers) { "; Secure" } else { "" };
    let max_age = SESSION_DAYS * 24 * 60 * 60;
    HeaderValue::from_str(&format!(
        "{SESSION_COOKIE}={token}; Path=/; HttpOnly; SameSite=Lax; Max-Age={max_age}{secure}"
    ))
    .expect("cookie is ASCII")
}

pub fn clear_cookie(headers: &HeaderMap) -> HeaderValue {
    let secure = if is_https(headers) { "; Secure" } else { "" };
    HeaderValue::from_str(&format!(
        "{SESSION_COOKIE}=; Path=/; HttpOnly; SameSite=Lax; Max-Age=0{secure}"
    ))
    .expect("cookie is ASCII")
}

/// The signed-in account from a session cookie or an API token.
pub async fn user_from_headers(state: &AppState, headers: &HeaderMap) -> AppResult<Option<User>> {
    if let Some(token) = bearer_token(headers) {
        return accounts::token_user(&state.pool, &hash_secret(token)).await;
    }
    if let Some(token) = session_token(headers) {
        return accounts::session_user(&state.pool, &hash_secret(token), session_expiry()).await;
    }
    Ok(None)
}

/// Middleware: lets the request through only for a signed-in account.
pub async fn require_user(
    State(state): State<AppState>,
    mut req: Request,
    next: Next,
) -> Result<Response, AppError> {
    let user = user_from_headers(&state, req.headers())
        .await?
        .ok_or_else(|| AppError::Unauthorized("Sign in to use Rollcall.".into()))?;
    req.extensions_mut().insert(CurrentUser(user));
    Ok(next.run(req).await)
}

/// The signed-in account, for handlers behind `require_user`.
#[derive(Debug, Clone)]
pub struct CurrentUser(pub User);

impl<S: Send + Sync> FromRequestParts<S> for CurrentUser {
    type Rejection = AppError;

    async fn from_request_parts(parts: &mut Parts, _: &S) -> Result<Self, Self::Rejection> {
        parts
            .extensions
            .get::<CurrentUser>()
            .cloned()
            .ok_or_else(|| AppError::Unauthorized("Sign in to use Rollcall.".into()))
    }
}

/// The signed-in account, when it is an admin.
#[derive(Debug, Clone)]
pub struct Admin(pub User);

impl<S: Send + Sync> FromRequestParts<S> for Admin {
    type Rejection = AppError;

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        let CurrentUser(user) = CurrentUser::from_request_parts(parts, state).await?;
        if !user.is_admin {
            return Err(AppError::Forbidden("Only an admin can do that.".into()));
        }
        Ok(Admin(user))
    }
}
