use crate::{
    AppState,
    auth::CurrentUser,
    error::AppResult,
    gcode,
    models::{AssignSlot, Print, PrintFilter, SlicerProfile},
    repo::prints,
};
use axum::{
    Json, Router,
    body::Bytes,
    extract::{DefaultBodyLimit, Path, Query, State},
    http::StatusCode,
    routing::{get, post, put},
};
use serde::Deserialize;
use uuid::Uuid;

/// Enough for a whole G-code file, for `curl --data-binary @print.gcode`. The
/// script and the web page send only the comment lines, a few kilobytes.
const MAX_GCODE_BYTES: usize = 256 * 1024 * 1024;

pub fn routes() -> Router<AppState> {
    Router::new()
        .route(
            "/prints",
            get(list)
                .post(create)
                .layer(DefaultBodyLimit::max(MAX_GCODE_BYTES)),
        )
        .route("/prints/{id}", get(show).delete(remove))
        .route("/prints/{id}/slots/{slot}", put(assign))
        .route("/prints/{id}/slots/{slot}/undo", post(undo))
        .route("/slicer-profiles", get(profiles).put(save_profile))
        .route(
            "/slicer-profiles/{name}",
            axum::routing::delete(delete_profile),
        )
}

#[derive(Deserialize)]
struct CreateQuery {
    /// The file name the slicer saves; the extension is dropped.
    name: Option<String>,
}

/// The print's name from a file name: "Benchy_PLA_1h2m.gcode" → "Benchy_PLA_1h2m".
fn print_name(raw: Option<&str>) -> String {
    let base = raw
        .unwrap_or("")
        .rsplit(['/', '\\'])
        .next()
        .unwrap_or("")
        .trim();
    let lower = base.to_ascii_lowercase();
    let cut = [".gcode.3mf", ".gcode", ".3mf", ".bgcode"]
        .iter()
        .find(|ext| lower.ends_with(*ext))
        .map_or(base.len(), |ext| base.len() - ext.len());
    let name = base[..cut].trim();
    if name.is_empty() {
        "Untitled print".into()
    } else {
        name.chars().take(200).collect()
    }
}

/// `POST /api/prints?name=<file name>` with G-code (or just its comment lines)
/// as the body. 201 for a new print, 200 when the same upload came moments ago.
async fn create(
    State(s): State<AppState>,
    CurrentUser(me): CurrentUser,
    Query(q): Query<CreateQuery>,
    body: Bytes,
) -> AppResult<(StatusCode, Json<Print>)> {
    let text = String::from_utf8_lossy(&body);
    let sliced = gcode::parse(&text);
    let (print, new) =
        prints::create(&s.pool, me.id, &print_name(q.name.as_deref()), &sliced).await?;
    Ok((
        if new {
            StatusCode::CREATED
        } else {
            StatusCode::OK
        },
        Json(print),
    ))
}

async fn list(
    State(s): State<AppState>,
    CurrentUser(me): CurrentUser,
    Query(f): Query<PrintFilter>,
) -> AppResult<Json<Vec<Print>>> {
    Ok(Json(prints::list(&s.pool, me.id, &f).await?))
}

async fn show(
    State(s): State<AppState>,
    CurrentUser(me): CurrentUser,
    Path(id): Path<Uuid>,
) -> AppResult<Json<Print>> {
    Ok(Json(prints::get(&s.pool, me.id, id).await?))
}

async fn remove(
    State(s): State<AppState>,
    CurrentUser(me): CurrentUser,
    Path(id): Path<Uuid>,
) -> AppResult<StatusCode> {
    prints::delete(&s.pool, me.id, id).await?;
    Ok(StatusCode::NO_CONTENT)
}

async fn assign(
    State(s): State<AppState>,
    CurrentUser(me): CurrentUser,
    Path((id, slot)): Path<(Uuid, i16)>,
    Json(body): Json<AssignSlot>,
) -> AppResult<Json<Print>> {
    Ok(Json(prints::assign(&s.pool, me.id, id, slot, body).await?))
}

async fn undo(
    State(s): State<AppState>,
    CurrentUser(me): CurrentUser,
    Path((id, slot)): Path<(Uuid, i16)>,
) -> AppResult<Json<Print>> {
    Ok(Json(prints::undo(&s.pool, me.id, id, slot).await?))
}

#[derive(Deserialize)]
struct ProfileQuery {
    filament_id: Option<Uuid>,
}

async fn profiles(
    State(s): State<AppState>,
    CurrentUser(me): CurrentUser,
    Query(q): Query<ProfileQuery>,
) -> AppResult<Json<Vec<SlicerProfile>>> {
    Ok(Json(prints::profiles(&s.pool, me.id, q.filament_id).await?))
}

async fn save_profile(
    State(s): State<AppState>,
    CurrentUser(me): CurrentUser,
    Json(body): Json<SlicerProfile>,
) -> AppResult<Json<SlicerProfile>> {
    Ok(Json(prints::save_profile(&s.pool, me.id, &body).await?))
}

async fn delete_profile(
    State(s): State<AppState>,
    CurrentUser(me): CurrentUser,
    Path(name): Path<String>,
) -> AppResult<StatusCode> {
    prints::delete_profile(&s.pool, me.id, &name).await?;
    Ok(StatusCode::NO_CONTENT)
}

#[cfg(test)]
mod tests {
    use super::print_name;

    #[test]
    fn names_from_files() {
        assert_eq!(print_name(Some("Benchy_PLA_1h2m.gcode")), "Benchy_PLA_1h2m");
        assert_eq!(print_name(Some("C:\\out\\Plate 1.gcode.3mf")), "Plate 1");
        assert_eq!(print_name(Some("/tmp/x/Cube.GCODE")), "Cube");
        assert_eq!(print_name(Some("  ")), "Untitled print");
        assert_eq!(print_name(None), "Untitled print");
    }
}
