use crate::{
    error::{AppError, AppResult},
    models::{HumidityReading, NewHumidityReading, trim_opt},
};
use sqlx::PgPool;
use uuid::Uuid;

/// A spool's readings, newest first. A missing spool is a 404.
pub async fn list(pool: &PgPool, me: Uuid, spool_id: Uuid) -> AppResult<Vec<HumidityReading>> {
    super::spools::access(pool, me, spool_id).await?;
    Ok(sqlx::query_as!(
        HumidityReading,
        r#"
        SELECT id, spool_id, measured_at, humidity_pct, temperature_c, note
        FROM humidity_readings
        WHERE spool_id = $1
        ORDER BY measured_at DESC, id DESC
        "#,
        spool_id
    )
    .fetch_all(pool)
    .await?)
}

pub async fn add(
    pool: &PgPool,
    me: Uuid,
    spool_id: Uuid,
    input: NewHumidityReading,
) -> AppResult<HumidityReading> {
    super::spools::require_edit(pool, me, spool_id).await?;
    Ok(sqlx::query_as!(
        HumidityReading,
        r#"
        INSERT INTO humidity_readings (spool_id, measured_at, humidity_pct, temperature_c, note)
        VALUES ($1, coalesce($2, now()), $3, $4, $5)
        RETURNING id, spool_id, measured_at, humidity_pct, temperature_c, note
        "#,
        spool_id,
        input.measured_at,
        input.humidity_pct,
        input.temperature_c,
        trim_opt(input.note)
    )
    .fetch_one(pool)
    .await?)
}

pub async fn delete(pool: &PgPool, me: Uuid, id: Uuid) -> AppResult<()> {
    let spool_id = sqlx::query_scalar!("SELECT spool_id FROM humidity_readings WHERE id = $1", id)
        .fetch_optional(pool)
        .await?
        .ok_or(AppError::NotFound("humidity reading"))?;
    super::spools::require_edit(pool, me, spool_id).await?;
    sqlx::query!("DELETE FROM humidity_readings WHERE id = $1", id)
        .execute(pool)
        .await?;
    Ok(())
}
