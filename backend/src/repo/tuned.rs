use crate::{
    error::{AppError, AppResult},
    models::{SaveTunedSettings, TunedSettings, trim_opt},
};
use sqlx::PgPool;
use uuid::Uuid;

/// The settings for one filament: the any-printer row first, then by printer name.
pub async fn list_for_filament(pool: &PgPool, filament_id: Uuid) -> AppResult<Vec<TunedSettings>> {
    // A missing filament is a 404, not an empty list.
    sqlx::query_scalar!("SELECT id FROM filaments WHERE id = $1", filament_id)
        .fetch_optional(pool)
        .await?
        .ok_or(AppError::NotFound("filament"))?;
    Ok(sqlx::query_as!(
        TunedSettings,
        r#"
        SELECT t.id, t.filament_id, t.printer_id, p.name AS "printer_name?",
               t.nozzle_temp_c, t.first_layer_nozzle_temp_c, t.bed_temp_c, t.print_speed_mm_s,
               t.max_volumetric_speed_mm3_s, t.flow_ratio, t.pressure_advance,
               t.retraction_length_mm, t.retraction_speed_mm_s, t.fan_speed_pct,
               t.drying_temp_c, t.drying_time_min, t.notes, t.verified_at, t.updated_at
        FROM tuned_settings t
        LEFT JOIN printers p ON p.id = t.printer_id
        WHERE t.filament_id = $1
        ORDER BY t.printer_id IS NOT NULL, p.is_default DESC, lower(p.name)
        "#,
        filament_id
    )
    .fetch_all(pool)
    .await?)
}

/// Creates or replaces the row for (filament, printer). An unknown filament
/// is a 404; an unknown printer fails its foreign key (422).
pub async fn save(
    pool: &PgPool,
    filament_id: Uuid,
    input: SaveTunedSettings,
) -> AppResult<TunedSettings> {
    sqlx::query_scalar!("SELECT id FROM filaments WHERE id = $1", filament_id)
        .fetch_optional(pool)
        .await?
        .ok_or(AppError::NotFound("filament"))?;
    let notes = trim_opt(input.notes);
    let id = sqlx::query_scalar!(
        r#"
        INSERT INTO tuned_settings (
            filament_id, printer_id, nozzle_temp_c, first_layer_nozzle_temp_c, bed_temp_c,
            print_speed_mm_s, max_volumetric_speed_mm3_s, flow_ratio, pressure_advance,
            retraction_length_mm, retraction_speed_mm_s, fan_speed_pct, drying_temp_c,
            drying_time_min, notes, verified_at
        )
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15,
                CASE WHEN $16 THEN now() END)
        ON CONFLICT ON CONSTRAINT tuned_settings_filament_printer_uq DO UPDATE SET
            nozzle_temp_c = EXCLUDED.nozzle_temp_c,
            first_layer_nozzle_temp_c = EXCLUDED.first_layer_nozzle_temp_c,
            bed_temp_c = EXCLUDED.bed_temp_c,
            print_speed_mm_s = EXCLUDED.print_speed_mm_s,
            max_volumetric_speed_mm3_s = EXCLUDED.max_volumetric_speed_mm3_s,
            flow_ratio = EXCLUDED.flow_ratio,
            pressure_advance = EXCLUDED.pressure_advance,
            retraction_length_mm = EXCLUDED.retraction_length_mm,
            retraction_speed_mm_s = EXCLUDED.retraction_speed_mm_s,
            fan_speed_pct = EXCLUDED.fan_speed_pct,
            drying_temp_c = EXCLUDED.drying_temp_c,
            drying_time_min = EXCLUDED.drying_time_min,
            notes = EXCLUDED.notes,
            verified_at = CASE WHEN $16 THEN coalesce(tuned_settings.verified_at, now()) END
        RETURNING id
        "#,
        filament_id,
        input.printer_id,
        input.nozzle_temp_c,
        input.first_layer_nozzle_temp_c,
        input.bed_temp_c,
        input.print_speed_mm_s,
        input.max_volumetric_speed_mm3_s,
        input.flow_ratio,
        input.pressure_advance,
        input.retraction_length_mm,
        input.retraction_speed_mm_s,
        input.fan_speed_pct,
        input.drying_temp_c,
        input.drying_time_min,
        notes,
        input.verified
    )
    .fetch_one(pool)
    .await?;
    list_for_filament(pool, filament_id)
        .await?
        .into_iter()
        .find(|t| t.id == id)
        .ok_or(AppError::NotFound("tuned settings"))
}

pub async fn delete(pool: &PgPool, id: Uuid) -> AppResult<()> {
    let res = sqlx::query!("DELETE FROM tuned_settings WHERE id = $1", id)
        .execute(pool)
        .await?;
    if res.rows_affected() == 0 {
        return Err(AppError::NotFound("tuned settings"));
    }
    Ok(())
}
