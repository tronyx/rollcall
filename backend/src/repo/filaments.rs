use super::lookups::{material_id_by_code, upsert_brand};
use crate::{
    error::{AppError, AppResult},
    models::{Filament, FilamentSpecs, NewFilament, SpecSource},
};
use sqlx::{PgPool, Postgres, Transaction};
use uuid::Uuid;

/// Lists filaments, or fetches one when `id` is given.
async fn query_filaments(pool: &PgPool, id: Option<Uuid>) -> AppResult<Vec<Filament>> {
    let rows = sqlx::query!(
        r#"
        SELECT f.id, f.brand_id, b.name AS brand_name, f.material_id, m.code AS material_code,
               f.product_name, f.color_name, f.color_hex, f.color_hex_secondary, f.diameter_mm,
               f.nominal_weight_g, f.typical_tare_g,
               f.nozzle_temp_min_c, f.nozzle_temp_max_c, f.bed_temp_min_c, f.bed_temp_max_c,
               f.print_speed_min_mm_s, f.print_speed_max_mm_s, f.drying_temp_c, f.drying_time_min,
               f.density_g_cm3, f.td_value, f.source_url, f.image_url,
               f.spec_source AS "spec_source: SpecSource", f.scraped_at, f.created_at, f.updated_at
        FROM filaments f
        JOIN brands b    ON b.id = f.brand_id
        JOIN materials m ON m.id = f.material_id
        WHERE ($1::uuid IS NULL OR f.id = $1)
        ORDER BY lower(b.name), m.code, lower(f.color_name)
        "#,
        id
    )
    .fetch_all(pool)
    .await?;

    Ok(rows
        .into_iter()
        .map(|r| Filament {
            id: r.id,
            brand_id: r.brand_id,
            brand_name: r.brand_name,
            material_id: r.material_id,
            material_code: r.material_code,
            product_name: r.product_name,
            color_name: r.color_name,
            color_hex: r.color_hex,
            color_hex_secondary: r.color_hex_secondary,
            diameter_mm: r.diameter_mm,
            nominal_weight_g: r.nominal_weight_g,
            typical_tare_g: r.typical_tare_g,
            specs: FilamentSpecs {
                nozzle_temp_min_c: r.nozzle_temp_min_c,
                nozzle_temp_max_c: r.nozzle_temp_max_c,
                bed_temp_min_c: r.bed_temp_min_c,
                bed_temp_max_c: r.bed_temp_max_c,
                print_speed_min_mm_s: r.print_speed_min_mm_s,
                print_speed_max_mm_s: r.print_speed_max_mm_s,
                drying_temp_c: r.drying_temp_c,
                drying_time_min: r.drying_time_min,
                density_g_cm3: r.density_g_cm3,
                td_value: r.td_value,
            },
            source_url: r.source_url,
            image_url: r.image_url,
            spec_source: r.spec_source,
            scraped_at: r.scraped_at,
            created_at: r.created_at,
            updated_at: r.updated_at,
        })
        .collect())
}

pub async fn list_filaments(pool: &PgPool) -> AppResult<Vec<Filament>> {
    query_filaments(pool, None).await
}

pub async fn get_filament(pool: &PgPool, id: Uuid) -> AppResult<Filament> {
    query_filaments(pool, Some(id))
        .await?
        .into_iter()
        .next()
        .ok_or(AppError::NotFound("filament"))
}

/// Reuses an existing filament when the same product page or the same
/// brand/material/product/color/diameter already exists; otherwise inserts it.
pub async fn find_or_create(
    tx: &mut Transaction<'_, Postgres>,
    mut input: NewFilament,
) -> AppResult<Uuid> {
    input.normalize();
    if input.color_name.is_empty() {
        return Err(AppError::BadRequest("color_name is required".into()));
    }
    let brand_id = upsert_brand(tx, &input.brand_name).await?;
    let material_id = material_id_by_code(tx, &input.material_code).await?;

    if let Some(existing) = sqlx::query_scalar!(
        r#"
        SELECT id FROM filaments
        WHERE ($1::text IS NOT NULL AND source_url = $1)
           OR (brand_id = $2 AND material_id = $3
               AND lower(coalesce(product_name, '')) = lower(coalesce($4, ''))
               AND lower(color_name) = lower($5)
               AND diameter_mm = $6)
        LIMIT 1
        "#,
        input.source_url,
        brand_id,
        material_id,
        input.product_name,
        input.color_name,
        input.diameter_mm
    )
    .fetch_optional(&mut **tx)
    .await?
    {
        return Ok(existing);
    }

    let s = &input.specs;
    let scraped_at = (input.spec_source != SpecSource::Manual).then(chrono::Utc::now);
    let id = sqlx::query_scalar!(
        r#"
        INSERT INTO filaments (
            brand_id, material_id, product_name, color_name, color_hex, color_hex_secondary,
            diameter_mm, nominal_weight_g, typical_tare_g,
            nozzle_temp_min_c, nozzle_temp_max_c, bed_temp_min_c, bed_temp_max_c,
            print_speed_min_mm_s, print_speed_max_mm_s, drying_temp_c, drying_time_min, density_g_cm3, td_value,
            source_url, image_url, spec_source, scraped_at, scrape_raw
        ) VALUES (
            $1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16, $17, $18,
            $19, $20, $21, $22, $23, $24
        )
        RETURNING id
        "#,
        brand_id,
        material_id,
        input.product_name,
        input.color_name,
        input.color_hex,
        input.color_hex_secondary,
        input.diameter_mm,
        input.nominal_weight_g,
        input.typical_tare_g,
        s.nozzle_temp_min_c,
        s.nozzle_temp_max_c,
        s.bed_temp_min_c,
        s.bed_temp_max_c,
        s.print_speed_min_mm_s,
        s.print_speed_max_mm_s,
        s.drying_temp_c,
        s.drying_time_min,
        s.density_g_cm3,
        s.td_value,
        input.source_url,
        input.image_url,
        input.spec_source as _,
        scraped_at,
        input.scrape_raw
    )
    .fetch_one(&mut **tx)
    .await?;
    Ok(id)
}

/// Replaces a filament's details. Every spool of this filament sees the
/// change. The brand is created if it's new; the material must exist.
pub async fn update_filament(
    pool: &PgPool,
    id: Uuid,
    mut input: NewFilament,
) -> AppResult<Filament> {
    input.normalize();
    if input.color_name.is_empty() {
        return Err(AppError::BadRequest("color_name is required".into()));
    }
    let mut tx = pool.begin().await?;
    let brand_id = upsert_brand(&mut tx, &input.brand_name).await?;
    let material_id = material_id_by_code(&mut tx, &input.material_code).await?;
    let s = &input.specs;
    let res = sqlx::query!(
        r#"
        UPDATE filaments SET
            brand_id = $2, material_id = $3, product_name = $4, color_name = $5,
            color_hex = $6, color_hex_secondary = $7, diameter_mm = $8,
            nominal_weight_g = $9, typical_tare_g = $10,
            nozzle_temp_min_c = $11, nozzle_temp_max_c = $12,
            bed_temp_min_c = $13, bed_temp_max_c = $14,
            print_speed_min_mm_s = $15, print_speed_max_mm_s = $16,
            drying_temp_c = $17, drying_time_min = $18,
            density_g_cm3 = $19, td_value = $20,
            source_url = $21, image_url = $22
        WHERE id = $1
        "#,
        id,
        brand_id,
        material_id,
        input.product_name,
        input.color_name,
        input.color_hex,
        input.color_hex_secondary,
        input.diameter_mm,
        input.nominal_weight_g,
        input.typical_tare_g,
        s.nozzle_temp_min_c,
        s.nozzle_temp_max_c,
        s.bed_temp_min_c,
        s.bed_temp_max_c,
        s.print_speed_min_mm_s,
        s.print_speed_max_mm_s,
        s.drying_temp_c,
        s.drying_time_min,
        s.density_g_cm3,
        s.td_value,
        input.source_url,
        input.image_url,
    )
    .execute(&mut *tx)
    .await?;
    if res.rows_affected() == 0 {
        return Err(AppError::NotFound("filament"));
    }
    tx.commit().await?;
    get_filament(pool, id).await
}
