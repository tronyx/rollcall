use crate::{
    error::{AppError, AppResult},
    models::{Photo, PhotoOwner, trim_opt},
};
use sqlx::PgPool;
use std::{collections::HashSet, path::Path};
use uuid::Uuid;

/// Photos of one spool, filament or piece of equipment, oldest first.
pub async fn list(pool: &PgPool, owner: &PhotoOwner) -> AppResult<Vec<Photo>> {
    if owner.count() != 1 {
        return Err(AppError::BadRequest(
            "give exactly one of spool_id, filament_id or equipment_id".into(),
        ));
    }
    Ok(sqlx::query_as!(
        Photo,
        r#"
        SELECT id, spool_id, filament_id, equipment_id, content_type, size_bytes,
               original_name, caption, created_at,
               '/api/photos/' || id || '/file' AS "url!"
        FROM photos
        WHERE spool_id = $1 OR filament_id = $2 OR equipment_id = $3
        ORDER BY created_at, id
        "#,
        owner.spool_id,
        owner.filament_id,
        owner.equipment_id
    )
    .fetch_all(pool)
    .await?)
}

pub async fn get(pool: &PgPool, id: Uuid) -> AppResult<(Photo, String)> {
    let row = sqlx::query!(
        r#"
        SELECT id, spool_id, filament_id, equipment_id, content_type, size_bytes,
               original_name, caption, created_at, file_name
        FROM photos WHERE id = $1
        "#,
        id
    )
    .fetch_optional(pool)
    .await?
    .ok_or(AppError::NotFound("photo"))?;
    Ok((
        Photo {
            id: row.id,
            spool_id: row.spool_id,
            filament_id: row.filament_id,
            equipment_id: row.equipment_id,
            content_type: row.content_type,
            size_bytes: row.size_bytes,
            original_name: row.original_name,
            caption: row.caption,
            created_at: row.created_at,
            url: format!("/api/photos/{}/file", row.id),
        },
        row.file_name,
    ))
}

/// Saves the image as `<id>.<ext>` in `dir`, then records it. The owner must
/// exist (404 otherwise); the file is removed again if the insert fails.
#[allow(clippy::too_many_arguments)]
pub async fn add(
    pool: &PgPool,
    dir: &Path,
    owner: &PhotoOwner,
    bytes: &[u8],
    content_type: &str,
    ext: &str,
    original_name: Option<String>,
    caption: Option<String>,
) -> AppResult<Photo> {
    if owner.count() != 1 {
        return Err(AppError::BadRequest(
            "give exactly one of spool_id, filament_id or equipment_id".into(),
        ));
    }
    let exists = sqlx::query_scalar!(
        r#"
        SELECT EXISTS (SELECT 1 FROM spools WHERE id = $1)
            OR EXISTS (SELECT 1 FROM filaments WHERE id = $2)
            OR EXISTS (SELECT 1 FROM equipment WHERE id = $3) AS "exists!"
        "#,
        owner.spool_id,
        owner.filament_id,
        owner.equipment_id
    )
    .fetch_one(pool)
    .await?;
    if !exists {
        return Err(AppError::NotFound(if owner.spool_id.is_some() {
            "spool"
        } else if owner.filament_id.is_some() {
            "filament"
        } else {
            "equipment"
        }));
    }

    let id = Uuid::now_v7();
    let file_name = format!("{id}.{ext}");
    let path = dir.join(&file_name);
    tokio::fs::write(&path, bytes)
        .await
        .map_err(|e| AppError::Internal(anyhow::anyhow!("saving {}: {e}", path.display())))?;
    let size = i32::try_from(bytes.len()).unwrap_or(i32::MAX);
    let inserted = sqlx::query!(
        r#"
        INSERT INTO photos (id, spool_id, filament_id, equipment_id, file_name, content_type,
                            size_bytes, original_name, caption)
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)
        "#,
        id,
        owner.spool_id,
        owner.filament_id,
        owner.equipment_id,
        file_name,
        content_type,
        size,
        trim_opt(original_name),
        trim_opt(caption)
    )
    .execute(pool)
    .await;
    if let Err(e) = inserted {
        let _ = tokio::fs::remove_file(&path).await;
        return Err(e.into());
    }
    Ok(get(pool, id).await?.0)
}

pub async fn set_caption(pool: &PgPool, id: Uuid, caption: Option<String>) -> AppResult<Photo> {
    let res = sqlx::query!(
        "UPDATE photos SET caption = $2 WHERE id = $1",
        id,
        trim_opt(caption)
    )
    .execute(pool)
    .await?;
    if res.rows_affected() == 0 {
        return Err(AppError::NotFound("photo"));
    }
    Ok(get(pool, id).await?.0)
}

/// Deletes the row, then the file. A file that's already gone is fine.
pub async fn delete(pool: &PgPool, dir: &Path, id: Uuid) -> AppResult<()> {
    let file_name = sqlx::query_scalar!("DELETE FROM photos WHERE id = $1 RETURNING file_name", id)
        .fetch_optional(pool)
        .await?
        .ok_or(AppError::NotFound("photo"))?;
    let _ = tokio::fs::remove_file(dir.join(file_name)).await;
    Ok(())
}

/// Removes files in `dir` that no photo row names: left behind when a spool or
/// piece of equipment was deleted (the rows go with it). Returns how many.
pub async fn remove_orphan_files(pool: &PgPool, dir: &Path) -> anyhow::Result<usize> {
    let known: HashSet<String> = sqlx::query_scalar!("SELECT file_name FROM photos")
        .fetch_all(pool)
        .await?
        .into_iter()
        .collect();
    let mut removed = 0;
    let mut entries = tokio::fs::read_dir(dir).await?;
    while let Some(entry) = entries.next_entry().await? {
        let name = entry.file_name().to_string_lossy().into_owned();
        // Only files this app could have written: "<uuid>.<ext>".
        let ours = name
            .split_once('.')
            .is_some_and(|(stem, _)| Uuid::parse_str(stem).is_ok());
        if ours && !known.contains(&name) && entry.file_type().await?.is_file() {
            tokio::fs::remove_file(entry.path()).await?;
            removed += 1;
        }
    }
    Ok(removed)
}

/// The files of one owner's photos, read before deleting the owner so they
/// can be removed after the rows cascade away.
pub async fn file_names(pool: &PgPool, owner: &PhotoOwner) -> AppResult<Vec<String>> {
    Ok(sqlx::query_scalar!(
        "SELECT file_name FROM photos WHERE spool_id = $1 OR filament_id = $2 OR equipment_id = $3",
        owner.spool_id,
        owner.filament_id,
        owner.equipment_id
    )
    .fetch_all(pool)
    .await?)
}

/// Removes these files, ignoring any that are already gone.
pub async fn remove_files(dir: &Path, names: &[String]) {
    for name in names {
        let _ = tokio::fs::remove_file(dir.join(name)).await;
    }
}
