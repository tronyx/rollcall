use crate::{
    error::{AppError, AppResult},
    models::{AppSettings, NewPrinter, Printer, trim_opt},
};
use sqlx::PgPool;
use uuid::Uuid;

pub async fn get_settings(pool: &PgPool) -> AppResult<AppSettings> {
    Ok(sqlx::query_as!(
        AppSettings,
        r#"SELECT low_stock_threshold_g, default_currency AS "default_currency!" FROM app_settings"#
    )
    .fetch_one(pool)
    .await?)
}

pub async fn update_settings(pool: &PgPool, input: AppSettings) -> AppResult<AppSettings> {
    Ok(sqlx::query_as!(
        AppSettings,
        r#"
        UPDATE app_settings SET low_stock_threshold_g = $1, default_currency = upper(btrim($2))
        RETURNING low_stock_threshold_g, default_currency AS "default_currency!"
        "#,
        input.low_stock_threshold_g,
        input.default_currency
    )
    .fetch_one(pool)
    .await?)
}

pub async fn list_printers(pool: &PgPool) -> AppResult<Vec<Printer>> {
    Ok(sqlx::query_as!(
        Printer,
        r#"
        SELECT id, name, model, nozzle_diameter_mm, is_default, notes
        FROM printers
        ORDER BY is_default DESC, lower(name)
        "#
    )
    .fetch_all(pool)
    .await?)
}

/// Inserts (`id` is None) or replaces a printer. Making it the default
/// clears the flag on the others in the same transaction.
pub async fn save_printer(
    pool: &PgPool,
    id: Option<Uuid>,
    input: NewPrinter,
) -> AppResult<Printer> {
    let name = input.name.trim().to_string();
    if name.is_empty() {
        return Err(AppError::BadRequest("name is required".into()));
    }
    let model = trim_opt(input.model);
    let notes = trim_opt(input.notes);
    let mut tx = pool.begin().await?;
    if input.is_default {
        sqlx::query!(
            "UPDATE printers SET is_default = false WHERE is_default AND id IS DISTINCT FROM $1",
            id
        )
        .execute(&mut *tx)
        .await?;
    }
    let printer = match id {
        None => {
            sqlx::query_as!(
                Printer,
                r#"
                INSERT INTO printers (name, model, nozzle_diameter_mm, is_default, notes)
                VALUES ($1, $2, $3, $4, $5)
                RETURNING id, name, model, nozzle_diameter_mm, is_default, notes
                "#,
                name,
                model,
                input.nozzle_diameter_mm,
                input.is_default,
                notes
            )
            .fetch_one(&mut *tx)
            .await?
        }
        Some(id) => sqlx::query_as!(
            Printer,
            r#"
            UPDATE printers
            SET name = $2, model = $3, nozzle_diameter_mm = $4, is_default = $5, notes = $6
            WHERE id = $1
            RETURNING id, name, model, nozzle_diameter_mm, is_default, notes
            "#,
            id,
            name,
            model,
            input.nozzle_diameter_mm,
            input.is_default,
            notes
        )
        .fetch_optional(&mut *tx)
        .await?
        .ok_or(AppError::NotFound("printer"))?,
    };
    tx.commit().await?;
    Ok(printer)
}

pub async fn delete_printer(pool: &PgPool, id: Uuid) -> AppResult<()> {
    let res = sqlx::query!("DELETE FROM printers WHERE id = $1", id)
        .execute(pool)
        .await?;
    if res.rows_affected() == 0 {
        return Err(AppError::NotFound("printer"));
    }
    Ok(())
}
