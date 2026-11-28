//! Accounts, browser sessions and API tokens.

use crate::{
    error::{AppError, AppResult},
    models::{ApiToken, User},
};
use chrono::{DateTime, Utc};
use sqlx::PgPool;
use uuid::Uuid;

/// Serializes the account changes that must keep at least one admin.
const ACCOUNTS_LOCK: i64 = 0x726f_6c6c_6361_6c6c; // "rollcall"

pub async fn user_count(pool: &PgPool) -> AppResult<i64> {
    Ok(sqlx::query_scalar!(r#"SELECT count(*) AS "n!" FROM users"#)
        .fetch_one(pool)
        .await?)
}

pub async fn registration_enabled(pool: &PgPool) -> AppResult<bool> {
    Ok(
        sqlx::query_scalar!("SELECT registration_enabled FROM app_settings")
            .fetch_one(pool)
            .await?,
    )
}

pub async fn set_registration_enabled(pool: &PgPool, enabled: bool) -> AppResult<bool> {
    Ok(sqlx::query_scalar!(
        "UPDATE app_settings SET registration_enabled = $1 RETURNING registration_enabled",
        enabled
    )
    .fetch_one(pool)
    .await?)
}

/// Creates an account. The first account is the admin and can always be
/// created; later ones only while registration is on, unless `by_admin`.
pub async fn create_user(
    pool: &PgPool,
    username: &str,
    password_hash: &str,
    by_admin: bool,
) -> AppResult<User> {
    let mut tx = pool.begin().await?;
    sqlx::query!("SELECT pg_advisory_xact_lock($1)", ACCOUNTS_LOCK)
        .execute(&mut *tx)
        .await?;
    let count = sqlx::query_scalar!(r#"SELECT count(*) AS "n!" FROM users"#)
        .fetch_one(&mut *tx)
        .await?;
    let first = count == 0;
    if !first && !by_admin {
        let open = sqlx::query_scalar!("SELECT registration_enabled FROM app_settings")
            .fetch_one(&mut *tx)
            .await?;
        if !open {
            return Err(AppError::Forbidden(
                "Registration is turned off. Ask the admin for an account.".into(),
            ));
        }
    }
    let taken = sqlx::query_scalar!(
        r#"SELECT EXISTS (SELECT 1 FROM users WHERE lower(username) = lower($1)) AS "taken!""#,
        username
    )
    .fetch_one(&mut *tx)
    .await?;
    if taken {
        return Err(AppError::Conflict("That username is taken.".into()));
    }
    let user = sqlx::query_as!(
        User,
        r#"
        INSERT INTO users (username, password_hash, is_admin)
        VALUES ($1, $2, $3)
        RETURNING id, username, is_admin, created_at
        "#,
        username,
        password_hash,
        first
    )
    .fetch_one(&mut *tx)
    .await?;
    sqlx::query!(
        r#"
        INSERT INTO user_settings (user_id, low_stock_threshold_g, default_currency)
        SELECT $1, low_stock_threshold_g, default_currency FROM app_settings
        "#,
        user.id
    )
    .execute(&mut *tx)
    .await?;
    if first {
        // Data from before accounts existed belongs to the first one.
        sqlx::query!("SELECT adopt_unowned($1)", user.id)
            .execute(&mut *tx)
            .await?;
    }
    tx.commit().await?;
    Ok(user)
}

/// The account and its password hash, for signing in.
pub async fn find_for_login(pool: &PgPool, username: &str) -> AppResult<Option<(User, String)>> {
    let row = sqlx::query!(
        r#"
        SELECT id, username, is_admin, created_at, password_hash
        FROM users WHERE lower(username) = lower($1)
        "#,
        username
    )
    .fetch_optional(pool)
    .await?;
    Ok(row.map(|r| {
        (
            User {
                id: r.id,
                username: r.username,
                is_admin: r.is_admin,
                created_at: r.created_at,
            },
            r.password_hash,
        )
    }))
}

pub async fn password_hash(pool: &PgPool, id: Uuid) -> AppResult<String> {
    sqlx::query_scalar!("SELECT password_hash FROM users WHERE id = $1", id)
        .fetch_optional(pool)
        .await?
        .ok_or(AppError::NotFound("user"))
}

pub async fn find_by_username(pool: &PgPool, username: &str) -> AppResult<Option<User>> {
    Ok(sqlx::query_as!(
        User,
        r#"SELECT id, username, is_admin, created_at FROM users WHERE lower(username) = lower($1)"#,
        username
    )
    .fetch_optional(pool)
    .await?)
}

pub async fn list_users(pool: &PgPool) -> AppResult<Vec<User>> {
    Ok(sqlx::query_as!(
        User,
        "SELECT id, username, is_admin, created_at FROM users ORDER BY lower(username)"
    )
    .fetch_all(pool)
    .await?)
}

/// Fails when the change would leave no admin.
async fn keep_an_admin(tx: &mut sqlx::PgConnection, without: Uuid) -> AppResult<()> {
    let others = sqlx::query_scalar!(
        r#"SELECT count(*) AS "n!" FROM users WHERE is_admin AND id <> $1"#,
        without
    )
    .fetch_one(&mut *tx)
    .await?;
    if others == 0 {
        return Err(AppError::Conflict(
            "Rollcall needs at least one admin. Make another account an admin first.".into(),
        ));
    }
    Ok(())
}

pub async fn set_admin(pool: &PgPool, id: Uuid, is_admin: bool) -> AppResult<User> {
    let mut tx = pool.begin().await?;
    sqlx::query!("SELECT pg_advisory_xact_lock($1)", ACCOUNTS_LOCK)
        .execute(&mut *tx)
        .await?;
    if !is_admin {
        keep_an_admin(&mut tx, id).await?;
    }
    let user = sqlx::query_as!(
        User,
        r#"
        UPDATE users SET is_admin = $2 WHERE id = $1
        RETURNING id, username, is_admin, created_at
        "#,
        id,
        is_admin
    )
    .fetch_optional(&mut *tx)
    .await?
    .ok_or(AppError::NotFound("user"))?;
    tx.commit().await?;
    Ok(user)
}

pub async fn delete_user(pool: &PgPool, id: Uuid) -> AppResult<()> {
    let mut tx = pool.begin().await?;
    sqlx::query!("SELECT pg_advisory_xact_lock($1)", ACCOUNTS_LOCK)
        .execute(&mut *tx)
        .await?;
    keep_an_admin(&mut tx, id).await?;
    let res = sqlx::query!("DELETE FROM users WHERE id = $1", id)
        .execute(&mut *tx)
        .await?;
    if res.rows_affected() == 0 {
        return Err(AppError::NotFound("user"));
    }
    tx.commit().await?;
    Ok(())
}

/// Replaces the password and signs out every session except `keep`.
pub async fn set_password(
    pool: &PgPool,
    id: Uuid,
    password_hash: &str,
    keep: Option<&[u8]>,
) -> AppResult<()> {
    let mut tx = pool.begin().await?;
    let res = sqlx::query!(
        "UPDATE users SET password_hash = $2 WHERE id = $1",
        id,
        password_hash
    )
    .execute(&mut *tx)
    .await?;
    if res.rows_affected() == 0 {
        return Err(AppError::NotFound("user"));
    }
    sqlx::query!(
        "DELETE FROM sessions WHERE user_id = $1 AND token_hash IS DISTINCT FROM $2",
        id,
        keep
    )
    .execute(&mut *tx)
    .await?;
    tx.commit().await?;
    Ok(())
}

pub async fn create_session(
    pool: &PgPool,
    user_id: Uuid,
    token_hash: &[u8],
    expires_at: DateTime<Utc>,
) -> AppResult<()> {
    sqlx::query!("DELETE FROM sessions WHERE expires_at < now()")
        .execute(pool)
        .await?;
    sqlx::query!(
        "INSERT INTO sessions (token_hash, user_id, expires_at) VALUES ($1, $2, $3)",
        token_hash,
        user_id,
        expires_at
    )
    .execute(pool)
    .await?;
    Ok(())
}

/// The account behind a session, sliding its expiry forward.
pub async fn session_user(
    pool: &PgPool,
    token_hash: &[u8],
    expires_at: DateTime<Utc>,
) -> AppResult<Option<User>> {
    Ok(sqlx::query_as!(
        User,
        r#"
        WITH s AS (
            UPDATE sessions SET expires_at = $2
            WHERE token_hash = $1 AND expires_at > now()
            RETURNING user_id
        )
        SELECT u.id, u.username, u.is_admin, u.created_at
        FROM s JOIN users u ON u.id = s.user_id
        "#,
        token_hash,
        expires_at
    )
    .fetch_optional(pool)
    .await?)
}

pub async fn delete_session(pool: &PgPool, token_hash: &[u8]) -> AppResult<()> {
    sqlx::query!("DELETE FROM sessions WHERE token_hash = $1", token_hash)
        .execute(pool)
        .await?;
    Ok(())
}

pub async fn create_token(
    pool: &PgPool,
    user_id: Uuid,
    name: &str,
    token_hash: &[u8],
) -> AppResult<ApiToken> {
    Ok(sqlx::query_as!(
        ApiToken,
        r#"
        INSERT INTO api_tokens (user_id, name, token_hash) VALUES ($1, $2, $3)
        RETURNING id, name, created_at, last_used_at
        "#,
        user_id,
        name,
        token_hash
    )
    .fetch_one(pool)
    .await?)
}

pub async fn list_tokens(pool: &PgPool, user_id: Uuid) -> AppResult<Vec<ApiToken>> {
    Ok(sqlx::query_as!(
        ApiToken,
        r#"
        SELECT id, name, created_at, last_used_at FROM api_tokens
        WHERE user_id = $1 ORDER BY created_at
        "#,
        user_id
    )
    .fetch_all(pool)
    .await?)
}

pub async fn delete_token(pool: &PgPool, user_id: Uuid, id: Uuid) -> AppResult<()> {
    let res = sqlx::query!(
        "DELETE FROM api_tokens WHERE id = $1 AND user_id = $2",
        id,
        user_id
    )
    .execute(pool)
    .await?;
    if res.rows_affected() == 0 {
        return Err(AppError::NotFound("token"));
    }
    Ok(())
}

/// The account behind an API token, noting when it was used.
pub async fn token_user(pool: &PgPool, token_hash: &[u8]) -> AppResult<Option<User>> {
    Ok(sqlx::query_as!(
        User,
        r#"
        WITH t AS (
            UPDATE api_tokens SET last_used_at = now()
            WHERE token_hash = $1
            RETURNING user_id
        )
        SELECT u.id, u.username, u.is_admin, u.created_at
        FROM t JOIN users u ON u.id = t.user_id
        "#,
        token_hash
    )
    .fetch_optional(pool)
    .await?)
}
