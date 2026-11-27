use crate::{
    error::{AppError, AppResult},
    models::{Brand, Location, LocationKind, Material, NewLocation},
};
use sqlx::{PgPool, Postgres, Transaction};
use uuid::Uuid;

pub async fn list_brands(pool: &PgPool) -> AppResult<Vec<Brand>> {
    Ok(sqlx::query_as!(
        Brand,
        "SELECT id, name, website FROM brands ORDER BY lower(name)"
    )
    .fetch_all(pool)
    .await?)
}

/// Finds a brand by case-insensitive name, creating it if needed.
pub async fn upsert_brand(tx: &mut Transaction<'_, Postgres>, name: &str) -> AppResult<Uuid> {
    let name = name.trim();
    if name.is_empty() {
        return Err(AppError::BadRequest("brand_name is required".into()));
    }
    let id = sqlx::query_scalar!(
        r#"
        INSERT INTO brands (name) VALUES ($1)
        ON CONFLICT ((lower(name))) DO UPDATE SET name = brands.name
        RETURNING id
        "#,
        name
    )
    .fetch_one(&mut **tx)
    .await?;
    Ok(id)
}

pub async fn list_materials(pool: &PgPool) -> AppResult<Vec<Material>> {
    Ok(sqlx::query_as!(
        Material,
        r#"
        SELECT id, code, display_name, family, density_g_cm3,
               default_drying_temp_c, default_drying_time_min, is_abrasive
        FROM materials
        ORDER BY family, code
        "#
    )
    .fetch_all(pool)
    .await?)
}

pub async fn material_id_by_code(
    tx: &mut Transaction<'_, Postgres>,
    code: &str,
) -> AppResult<Uuid> {
    sqlx::query_scalar!(
        "SELECT id FROM materials WHERE code = upper($1)",
        code.trim()
    )
    .fetch_optional(&mut **tx)
    .await?
    .ok_or_else(|| AppError::BadRequest(format!("unknown material code '{code}'")))
}

pub async fn list_locations(pool: &PgPool) -> AppResult<Vec<Location>> {
    Ok(sqlx::query_as!(
        Location,
        r#"
        SELECT id, name, kind AS "kind: LocationKind", description, sort_order
        FROM locations
        ORDER BY sort_order, lower(name)
        "#
    )
    .fetch_all(pool)
    .await?)
}

pub async fn create_location(pool: &PgPool, input: NewLocation) -> AppResult<Location> {
    let name = input.name.trim();
    if name.is_empty() {
        return Err(AppError::BadRequest("name is required".into()));
    }
    Ok(sqlx::query_as!(
        Location,
        r#"
        INSERT INTO locations (name, kind, description, sort_order)
        VALUES ($1, $2, $3, $4)
        RETURNING id, name, kind AS "kind: LocationKind", description, sort_order
        "#,
        name,
        input.kind as _,
        input.description,
        input.sort_order
    )
    .fetch_one(pool)
    .await?)
}

pub async fn delete_location(pool: &PgPool, id: Uuid) -> AppResult<()> {
    let res = sqlx::query!("DELETE FROM locations WHERE id = $1", id)
        .execute(pool)
        .await?;
    if res.rows_affected() == 0 {
        return Err(AppError::NotFound("location"));
    }
    Ok(())
}
