use crate::{
    AppState,
    auth::{self, Admin, CurrentUser},
    error::{AppError, AppResult},
    models::{
        AdminSettings, ApiToken, AuthStatus, ChangePassword, CreateApiToken, Credentials,
        NewApiToken, SetPassword, UpdateUser, User,
    },
    repo::accounts,
};
use axum::{
    Json, Router,
    extract::{Path, State},
    http::{HeaderMap, StatusCode, header},
    response::{IntoResponse, Response},
    routing::{delete, get, post, put},
};
use uuid::Uuid;

/// Routes anyone can call: signing in, registering, and what the login page shows.
pub fn public_routes() -> Router<AppState> {
    Router::new()
        .route("/auth/status", get(status))
        .route("/auth/register", post(register))
        .route("/auth/login", post(login))
        .route("/auth/logout", post(logout))
}

/// Routes for a signed-in account.
pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/account/password", put(change_password))
        .route("/account/tokens", get(tokens).post(create_token))
        .route("/account/tokens/{id}", delete(delete_token))
        .route("/users", get(users).post(add_user))
        .route("/users/{id}", put(update_user).delete(delete_user))
        .route("/users/{id}/password", put(reset_password))
        .route(
            "/admin/settings",
            get(admin_settings).put(update_admin_settings),
        )
}

async fn status(State(s): State<AppState>, headers: HeaderMap) -> AppResult<Json<AuthStatus>> {
    Ok(Json(AuthStatus {
        setup_needed: accounts::user_count(&s.pool).await? == 0,
        registration_enabled: accounts::registration_enabled(&s.pool).await?,
        user: auth::user_from_headers(&s, &headers).await?,
    }))
}

/// Starts a session and returns the account with its cookie.
async fn sign_in(
    s: &AppState,
    headers: &HeaderMap,
    user: User,
    status: StatusCode,
) -> AppResult<Response> {
    let token = auth::new_secret();
    accounts::create_session(
        &s.pool,
        user.id,
        &auth::hash_secret(&token),
        auth::session_expiry(),
    )
    .await?;
    Ok((
        status,
        [(header::SET_COOKIE, auth::session_cookie(&token, headers))],
        Json(user),
    )
        .into_response())
}

async fn register(
    State(s): State<AppState>,
    headers: HeaderMap,
    Json(body): Json<Credentials>,
) -> AppResult<Response> {
    let username = auth::check_username(&body.username)?;
    auth::check_password(&body.password)?;
    let hash = auth::hash_password(&body.password).await?;
    let user = accounts::create_user(&s.pool, &username, &hash, false).await?;
    tracing::info!(username = %user.username, admin = user.is_admin, "account registered");
    sign_in(&s, &headers, user, StatusCode::CREATED).await
}

async fn login(
    State(s): State<AppState>,
    headers: HeaderMap,
    Json(body): Json<Credentials>,
) -> AppResult<Response> {
    let found = accounts::find_for_login(&s.pool, body.username.trim()).await?;
    let (user, hash) = match found {
        Some((u, h)) => (Some(u), Some(h)),
        None => (None, None),
    };
    if !auth::verify_password(&body.password, hash).await {
        tracing::warn!(username = %body.username.trim(), "failed sign-in");
        return Err(AppError::Unauthorized(
            "That username and password don't match.".into(),
        ));
    }
    let user = user.expect("verified against a stored hash");
    sign_in(&s, &headers, user, StatusCode::OK).await
}

async fn logout(State(s): State<AppState>, headers: HeaderMap) -> AppResult<Response> {
    if let Some(token) = auth::session_token(&headers) {
        accounts::delete_session(&s.pool, &auth::hash_secret(token)).await?;
    }
    Ok((
        StatusCode::NO_CONTENT,
        [(header::SET_COOKIE, auth::clear_cookie(&headers))],
    )
        .into_response())
}

async fn change_password(
    State(s): State<AppState>,
    CurrentUser(user): CurrentUser,
    headers: HeaderMap,
    Json(body): Json<ChangePassword>,
) -> AppResult<StatusCode> {
    let current = accounts::password_hash(&s.pool, user.id).await?;
    if !auth::verify_password(&body.current_password, Some(current)).await {
        return Err(AppError::BadRequest(
            "Your current password isn't right.".into(),
        ));
    }
    auth::check_password(&body.new_password)?;
    let hash = auth::hash_password(&body.new_password).await?;
    // Stay signed in here; sign out everywhere else.
    let keep = auth::session_token(&headers).map(auth::hash_secret);
    accounts::set_password(&s.pool, user.id, &hash, keep.as_deref()).await?;
    Ok(StatusCode::NO_CONTENT)
}

async fn tokens(
    State(s): State<AppState>,
    CurrentUser(user): CurrentUser,
) -> AppResult<Json<Vec<ApiToken>>> {
    Ok(Json(accounts::list_tokens(&s.pool, user.id).await?))
}

async fn create_token(
    State(s): State<AppState>,
    CurrentUser(user): CurrentUser,
    Json(body): Json<CreateApiToken>,
) -> AppResult<(StatusCode, Json<NewApiToken>)> {
    let name = body.name.trim();
    if name.is_empty() {
        return Err(AppError::BadRequest("Give the token a name.".into()));
    }
    let secret = format!("{}{}", auth::TOKEN_PREFIX, auth::new_secret());
    let token = accounts::create_token(&s.pool, user.id, name, &auth::hash_secret(&secret)).await?;
    Ok((StatusCode::CREATED, Json(NewApiToken { token, secret })))
}

async fn delete_token(
    State(s): State<AppState>,
    CurrentUser(user): CurrentUser,
    Path(id): Path<Uuid>,
) -> AppResult<StatusCode> {
    accounts::delete_token(&s.pool, user.id, id).await?;
    Ok(StatusCode::NO_CONTENT)
}

async fn users(State(s): State<AppState>, _: Admin) -> AppResult<Json<Vec<User>>> {
    Ok(Json(accounts::list_users(&s.pool).await?))
}

/// An admin can add an account even while registration is off.
async fn add_user(
    State(s): State<AppState>,
    _: Admin,
    Json(body): Json<Credentials>,
) -> AppResult<(StatusCode, Json<User>)> {
    let username = auth::check_username(&body.username)?;
    auth::check_password(&body.password)?;
    let hash = auth::hash_password(&body.password).await?;
    let user = accounts::create_user(&s.pool, &username, &hash, true).await?;
    Ok((StatusCode::CREATED, Json(user)))
}

async fn update_user(
    State(s): State<AppState>,
    _: Admin,
    Path(id): Path<Uuid>,
    Json(body): Json<UpdateUser>,
) -> AppResult<Json<User>> {
    Ok(Json(accounts::set_admin(&s.pool, id, body.is_admin).await?))
}

async fn delete_user(
    State(s): State<AppState>,
    Admin(me): Admin,
    Path(id): Path<Uuid>,
) -> AppResult<StatusCode> {
    if id == me.id {
        return Err(AppError::BadRequest(
            "You can't remove your own account.".into(),
        ));
    }
    accounts::delete_user(&s.pool, id).await?;
    Ok(StatusCode::NO_CONTENT)
}

async fn reset_password(
    State(s): State<AppState>,
    _: Admin,
    Path(id): Path<Uuid>,
    Json(body): Json<SetPassword>,
) -> AppResult<StatusCode> {
    auth::check_password(&body.password)?;
    let hash = auth::hash_password(&body.password).await?;
    accounts::set_password(&s.pool, id, &hash, None).await?;
    Ok(StatusCode::NO_CONTENT)
}

async fn admin_settings(State(s): State<AppState>, _: Admin) -> AppResult<Json<AdminSettings>> {
    Ok(Json(AdminSettings {
        registration_enabled: accounts::registration_enabled(&s.pool).await?,
    }))
}

async fn update_admin_settings(
    State(s): State<AppState>,
    _: Admin,
    Json(body): Json<AdminSettings>,
) -> AppResult<Json<AdminSettings>> {
    Ok(Json(AdminSettings {
        registration_enabled: accounts::set_registration_enabled(
            &s.pool,
            body.registration_enabled,
        )
        .await?,
    }))
}
