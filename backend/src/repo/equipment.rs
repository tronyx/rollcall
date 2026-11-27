use super::{contains_pattern, lookups::upsert_brand};
use crate::{
    error::{AppError, AppResult},
    models::{
        AcquisitionKind, Equipment, EquipmentCategory, EquipmentFilter, SaveEquipment, trim_opt,
    },
};
use sqlx::PgPool;
use uuid::Uuid;

async fn query(pool: &PgPool, id: Option<Uuid>, f: &EquipmentFilter) -> AppResult<Vec<Equipment>> {
    // "none" asks for items with no printer; anything else must be an id.
    let (printer_id, no_printer) = match f.printer.as_deref().map(str::trim) {
        None | Some("") => (None, false),
        Some("none") => (None, true),
        Some(p) => (
            Some(
                Uuid::parse_str(p)
                    .map_err(|_| AppError::BadRequest("printer must be an id or none".into()))?,
            ),
            false,
        ),
    };
    let q = f.q.as_deref().and_then(contains_pattern);
    Ok(sqlx::query_as!(
        Equipment,
        r#"
        SELECT e.id, e.category AS "category: EquipmentCategory", e.kind, e.name,
               e.brand_id, b.name AS "brand_name?", e.printer_id, p.name AS "printer_name?",
               e.model, e.quantity, e.retired,
               e.acquisition AS "acquisition: AcquisitionKind", e.acquired_on, e.acquired_from,
               e.price, e.currency, e.discounted, e.discount_note, e.notes,
               e.created_at, e.updated_at
        FROM equipment e
        LEFT JOIN brands b   ON b.id = e.brand_id
        LEFT JOIN printers p ON p.id = e.printer_id
        WHERE ($1::uuid IS NULL OR e.id = $1)
          AND ($2::equipment_category IS NULL OR e.category = $2)
          AND ($3::uuid IS NULL OR e.brand_id = $3)
          AND ($4::uuid IS NULL OR e.printer_id = $4)
          AND (NOT $5 OR e.printer_id IS NULL)
          AND ($6 OR $1::uuid IS NOT NULL OR NOT e.retired)
          AND ($7::text IS NULL
               OR e.name ILIKE $7 OR coalesce(e.kind, '') ILIKE $7
               OR coalesce(e.model, '') ILIKE $7 OR coalesce(b.name, '') ILIKE $7
               OR coalesce(e.notes, '') ILIKE $7)
        ORDER BY e.retired, e.category, lower(coalesce(e.kind, '')), lower(e.name), e.created_at
        "#,
        id,
        f.category as Option<EquipmentCategory>,
        f.brand_id,
        printer_id,
        no_printer,
        f.include_retired,
        q,
    )
    .fetch_all(pool)
    .await?)
}

pub async fn list(pool: &PgPool, filter: &EquipmentFilter) -> AppResult<Vec<Equipment>> {
    query(pool, None, filter).await
}

pub async fn get(pool: &PgPool, id: Uuid) -> AppResult<Equipment> {
    query(pool, Some(id), &EquipmentFilter::default())
        .await?
        .into_iter()
        .next()
        .ok_or(AppError::NotFound("equipment"))
}

/// Inserts (`id` is None) or replaces an item.
pub async fn save(pool: &PgPool, id: Option<Uuid>, input: SaveEquipment) -> AppResult<Equipment> {
    let name = input.name.trim().to_string();
    if name.is_empty() {
        return Err(AppError::BadRequest("name is required".into()));
    }
    let currency = input
        .currency
        .map(|c| c.trim().to_ascii_uppercase())
        .filter(|c| !c.is_empty());
    let mut tx = pool.begin().await?;
    if let Some(id) = id {
        sqlx::query_scalar!("SELECT id FROM equipment WHERE id = $1 FOR UPDATE", id)
            .fetch_optional(&mut *tx)
            .await?
            .ok_or(AppError::NotFound("equipment"))?;
    }
    let brand_id = match trim_opt(input.brand_name) {
        Some(b) => Some(upsert_brand(&mut tx, &b).await?),
        None => None,
    };
    let discount_note = if input.discounted {
        trim_opt(input.discount_note)
    } else {
        None
    };
    let saved = sqlx::query_scalar!(
        r#"
        INSERT INTO equipment (
            id, category, kind, name, brand_id, printer_id, model, quantity, retired,
            acquisition, acquired_on, acquired_from, price, currency, discounted, discount_note,
            notes
        ) VALUES (
            coalesce($1, uuidv7()), $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13,
            coalesce($14, (SELECT default_currency FROM app_settings)), $15, $16, $17
        )
        ON CONFLICT (id) DO UPDATE SET
            category = EXCLUDED.category, kind = EXCLUDED.kind, name = EXCLUDED.name,
            brand_id = EXCLUDED.brand_id, printer_id = EXCLUDED.printer_id,
            model = EXCLUDED.model, quantity = EXCLUDED.quantity, retired = EXCLUDED.retired,
            acquisition = EXCLUDED.acquisition, acquired_on = EXCLUDED.acquired_on,
            acquired_from = EXCLUDED.acquired_from, price = EXCLUDED.price,
            currency = EXCLUDED.currency, discounted = EXCLUDED.discounted,
            discount_note = EXCLUDED.discount_note, notes = EXCLUDED.notes
        RETURNING id
        "#,
        id,
        input.category as EquipmentCategory,
        trim_opt(input.kind),
        name,
        brand_id,
        input.printer_id,
        trim_opt(input.model),
        input.quantity,
        input.retired,
        input.acquisition as AcquisitionKind,
        input.acquired_on,
        trim_opt(input.acquired_from),
        input.price,
        currency,
        input.discounted,
        discount_note,
        trim_opt(input.notes),
    )
    .fetch_one(&mut *tx)
    .await?;
    tx.commit().await?;
    get(pool, saved).await
}

pub async fn delete(pool: &PgPool, id: Uuid) -> AppResult<()> {
    let res = sqlx::query!("DELETE FROM equipment WHERE id = $1", id)
        .execute(pool)
        .await?;
    if res.rows_affected() == 0 {
        return Err(AppError::NotFound("equipment"));
    }
    Ok(())
}
