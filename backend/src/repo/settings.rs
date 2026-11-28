use crate::{
    error::{AppError, AppResult},
    models::{AppSettings, NewPrinter, Printer, trim_opt},
};
use sqlx::PgPool;
use uuid::Uuid;

pub async fn get_settings(pool: &PgPool, me: Uuid) -> AppResult<AppSettings> {
    Ok(sqlx::query_as!(
        AppSettings,
        r#"
        SELECT low_stock_threshold_g, default_currency AS "default_currency!"
        FROM user_settings WHERE user_id = $1
        "#,
        me
    )
    .fetch_one(pool)
    .await?)
}

pub async fn update_settings(
    pool: &PgPool,
    me: Uuid,
    input: AppSettings,
) -> AppResult<AppSettings> {
    Ok(sqlx::query_as!(
        AppSettings,
        r#"
        UPDATE user_settings SET low_stock_threshold_g = $2, default_currency = upper(btrim($3))
        WHERE user_id = $1
        RETURNING low_stock_threshold_g, default_currency AS "default_currency!"
        "#,
        me,
        input.low_stock_threshold_g,
        input.default_currency
    )
    .fetch_one(pool)
    .await?)
}

pub async fn list_printers(pool: &PgPool, me: Uuid) -> AppResult<Vec<Printer>> {
    Ok(sqlx::query_as!(
        Printer,
        r#"
        SELECT id, name, model, nozzle_diameter_mm, is_default, notes
        FROM printers
        WHERE owner_id = $1
        ORDER BY is_default DESC, lower(name)
        "#,
        me
    )
    .fetch_all(pool)
    .await?)
}

/// Inserts (`id` is None) or replaces a printer. Making it the default
/// clears the flag on the others in the same transaction.
pub async fn save_printer(
    pool: &PgPool,
    me: Uuid,
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
            r#"
            UPDATE printers SET is_default = false
            WHERE is_default AND owner_id = $2 AND id IS DISTINCT FROM $1
            "#,
            id,
            me
        )
        .execute(&mut *tx)
        .await?;
    }
    let printer = match id {
        None => {
            sqlx::query_as!(
                Printer,
                r#"
                INSERT INTO printers (owner_id, name, model, nozzle_diameter_mm, is_default, notes)
                VALUES ($6, $1, $2, $3, $4, $5)
                RETURNING id, name, model, nozzle_diameter_mm, is_default, notes
                "#,
                name,
                model,
                input.nozzle_diameter_mm,
                input.is_default,
                notes,
                me
            )
            .fetch_one(&mut *tx)
            .await?
        }
        Some(id) => sqlx::query_as!(
            Printer,
            r#"
            UPDATE printers
            SET name = $2, model = $3, nozzle_diameter_mm = $4, is_default = $5, notes = $6
            WHERE id = $1 AND owner_id = $7
            RETURNING id, name, model, nozzle_diameter_mm, is_default, notes
            "#,
            id,
            name,
            model,
            input.nozzle_diameter_mm,
            input.is_default,
            notes,
            me
        )
        .fetch_optional(&mut *tx)
        .await?
        .ok_or(AppError::NotFound("printer"))?,
    };
    tx.commit().await?;
    Ok(printer)
}

pub async fn delete_printer(pool: &PgPool, me: Uuid, id: Uuid) -> AppResult<()> {
    let res = sqlx::query!(
        "DELETE FROM printers WHERE id = $1 AND owner_id = $2",
        id,
        me
    )
    .execute(pool)
    .await?;
    if res.rows_affected() == 0 {
        return Err(AppError::NotFound("printer"));
    }
    Ok(())
}
