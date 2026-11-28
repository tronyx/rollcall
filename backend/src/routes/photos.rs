use crate::{
    AppState,
    auth::CurrentUser,
    error::{AppError, AppResult},
    models::{Photo, PhotoOwner, sniff_image},
    repo::photos,
};
use axum::{
    Json, Router,
    body::Body,
    extract::{DefaultBodyLimit, Multipart, Path, Query, State},
    http::{StatusCode, header},
    response::Response,
    routing::get,
};
use serde::Deserialize;
use uuid::Uuid;

/// The largest photo accepted. Phone photos are usually 2-8 MB.
pub const MAX_PHOTO_BYTES: usize = 20 * 1024 * 1024;

pub fn routes() -> Router<AppState> {
    Router::new()
        .route(
            "/photos",
            get(list)
                .post(upload)
                // Room for the form fields around the file.
                .layer(DefaultBodyLimit::max(MAX_PHOTO_BYTES + 64 * 1024)),
        )
        .route("/photos/{id}", get(show).patch(caption).delete(remove))
        .route("/photos/{id}/file", get(file))
}

async fn list(
    State(s): State<AppState>,
    CurrentUser(me): CurrentUser,
    Query(owner): Query<PhotoOwner>,
) -> AppResult<Json<Vec<Photo>>> {
    Ok(Json(photos::list(&s.pool, me.id, &owner).await?))
}

async fn show(
    State(s): State<AppState>,
    CurrentUser(me): CurrentUser,
    Path(id): Path<Uuid>,
) -> AppResult<Json<Photo>> {
    Ok(Json(photos::get(&s.pool, me.id, id).await?.0))
}

fn bad(msg: impl Into<String>) -> AppError {
    AppError::BadRequest(msg.into())
}

/// `POST /api/photos` as `multipart/form-data`: one of `spool_id`,
/// `filament_id` or `equipment_id`, a `file`, and an optional `caption`.
async fn upload(
    State(s): State<AppState>,
    CurrentUser(me): CurrentUser,
    mut form: Multipart,
) -> AppResult<(StatusCode, Json<Photo>)> {
    let mut owner = PhotoOwner::default();
    let mut caption = None;
    let mut file: Option<(Vec<u8>, Option<String>)> = None;
    while let Some(field) = form
        .next_field()
        .await
        .map_err(|e| bad(format!("could not read the upload: {e}")))?
    {
        let name = field.name().unwrap_or_default().to_string();
        let id =
            |v: String| Uuid::parse_str(v.trim()).map_err(|_| bad(format!("{name} must be an id")));
        match name.as_str() {
            "file" => {
                let original = field.file_name().map(str::to_string);
                let bytes = field
                    .bytes()
                    .await
                    .map_err(|e| bad(format!("could not read the file: {e}")))?;
                file = Some((bytes.to_vec(), original));
            }
            "spool_id" | "filament_id" | "equipment_id" => {
                let value = field.text().await.map_err(|e| bad(e.to_string()))?;
                let parsed = Some(id(value)?);
                match name.as_str() {
                    "spool_id" => owner.spool_id = parsed,
                    "filament_id" => owner.filament_id = parsed,
                    _ => owner.equipment_id = parsed,
                }
            }
            "caption" => caption = Some(field.text().await.map_err(|e| bad(e.to_string()))?),
            _ => {}
        }
    }
    let (bytes, original) = file.ok_or_else(|| bad("no file was uploaded"))?;
    if bytes.is_empty() {
        return Err(bad("the file is empty"));
    }
    if bytes.len() > MAX_PHOTO_BYTES {
        return Err(bad("photos can be up to 20 MB"));
    }
    let (content_type, ext) = sniff_image(&bytes)
        .ok_or_else(|| bad("that isn't a JPEG, PNG, WebP, GIF or AVIF image"))?;
    let photo = photos::add(
        &s.pool,
        me.id,
        &s.uploads,
        &owner,
        &bytes,
        content_type,
        ext,
        original,
        caption,
    )
    .await?;
    Ok((StatusCode::CREATED, Json(photo)))
}

#[derive(Deserialize)]
struct CaptionBody {
    caption: Option<String>,
}

async fn caption(
    State(s): State<AppState>,
    CurrentUser(me): CurrentUser,
    Path(id): Path<Uuid>,
    Json(body): Json<CaptionBody>,
) -> AppResult<Json<Photo>> {
    Ok(Json(
        photos::set_caption(&s.pool, me.id, id, body.caption).await?,
    ))
}

async fn remove(
    State(s): State<AppState>,
    CurrentUser(me): CurrentUser,
    Path(id): Path<Uuid>,
) -> AppResult<StatusCode> {
    photos::delete(&s.pool, me.id, &s.uploads, id).await?;
    Ok(StatusCode::NO_CONTENT)
}

/// The image itself. A photo's file never changes, so browsers may cache it for good.
async fn file(
    State(s): State<AppState>,
    CurrentUser(me): CurrentUser,
    Path(id): Path<Uuid>,
) -> AppResult<Response> {
    let (photo, file_name) = photos::get(&s.pool, me.id, id).await?;
    let bytes = tokio::fs::read(s.uploads.join(&file_name))
        .await
        .map_err(|_| AppError::NotFound("photo file"))?;
    Response::builder()
        .header(header::CONTENT_TYPE, photo.content_type)
        .header(
            header::CACHE_CONTROL,
            "private, max-age=31536000, immutable",
        )
        .header(header::X_CONTENT_TYPE_OPTIONS, "nosniff")
        .header(
            header::CONTENT_SECURITY_POLICY,
            "default-src 'none'; sandbox",
        )
        .body(Body::from(bytes))
        .map_err(|e| AppError::Internal(e.into()))
}
