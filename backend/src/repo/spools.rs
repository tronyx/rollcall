use super::{contains_pattern, filaments};
use crate::{
    error::{AppError, AppResult},
    models::{
        AcquisitionKind, CreateSpool, LocationKind, SpoolFilter, SpoolStatus, SpoolView,
        UpdateSpool, WeightAdjustment, WeightEvent, WeightEventKind, trim_opt,
    },
};
use rust_decimal::Decimal;
use sqlx::PgPool;
use uuid::Uuid;

/// Weigh-ins may read a little over the label weight; matches the DB check.
const MAX_OVERFILL: Decimal = Decimal::from_parts(110, 0, 0, false, 2); // 1.10

async fn query_spools(
    pool: &PgPool,
    id: Option<Uuid>,
    f: &SpoolFilter,
) -> AppResult<Vec<SpoolView>> {
    let q = f.q.as_deref().and_then(contains_pattern);
    let color = f.color.as_deref().and_then(contains_pattern);
    let rows = sqlx::query_as!(
        SpoolView,
        r#"
        SELECT
            s.id, s.label_code, s.status AS "status: SpoolStatus",
            s.initial_weight_g, s.remaining_weight_g, s.tare_weight_g,
            round(100 * s.remaining_weight_g / s.initial_weight_g, 1) AS "remaining_pct!",
            (s.status NOT IN ('empty', 'archived')
                AND s.remaining_weight_g < st.low_stock_threshold_g) AS "is_low_stock!",
            s.purchase_price, s.currency, s.purchase_date, s.vendor,
            s.acquisition AS "acquisition: AcquisitionKind", s.discounted, s.discount_note,
            s.lot_number,
            s.opened_at, s.last_dried_at, s.emptied_at, s.notes, s.created_at, s.updated_at,
            f.id AS filament_id, f.product_name, f.color_name, f.color_hex, f.color_hex_secondary,
            f.diameter_mm, f.image_url,
            b.id AS brand_id, b.name AS brand_name,
            m.id AS material_id, m.code AS material_code,
            l.id AS "location_id?", l.name AS "location_name?",
            l.kind AS "location_kind?: LocationKind"
        FROM spools s
        JOIN filaments f      ON f.id = s.filament_id
        JOIN brands b         ON b.id = f.brand_id
        JOIN materials m      ON m.id = f.material_id
        LEFT JOIN locations l ON l.id = s.location_id
        CROSS JOIN app_settings st
        WHERE ($1::uuid IS NULL OR s.id = $1)
          AND ($2::text IS NULL OR m.code = upper($2))
          AND ($3::text IS NULL OR m.family = upper($3) OR m.code = upper($3))
          AND ($4::uuid IS NULL OR f.brand_id = $4)
          AND ($5::uuid IS NULL OR s.location_id = $5)
          AND ($6::spool_status IS NULL OR s.status = $6)
          AND ($7::text IS NULL OR f.color_name ILIKE $7)
          AND (NOT $8 OR (s.status NOT IN ('empty', 'archived')
                          AND s.remaining_weight_g < st.low_stock_threshold_g))
          AND ($9 OR $1::uuid IS NOT NULL OR $6::spool_status IS NOT NULL
               OR s.status NOT IN ('empty', 'archived'))
          AND ($10::text IS NULL
               OR b.name ILIKE $10
               OR f.color_name ILIKE $10
               OR coalesce(f.product_name, '') ILIKE $10
               OR m.code ILIKE $10
               OR coalesce(s.label_code, '') ILIKE $10
               OR coalesce(s.notes, '') ILIKE $10)
        ORDER BY
            CASE s.status WHEN 'active' THEN 0 WHEN 'drying' THEN 1 WHEN 'sealed' THEN 2
                          WHEN 'empty' THEN 3 ELSE 4 END,
            lower(b.name), m.code, lower(f.color_name), s.created_at
        "#,
        id,
        f.material.as_deref(),
        f.material_family.as_deref(),
        f.brand_id,
        f.location_id,
        f.status as Option<SpoolStatus>,
        color,
        f.low_stock,
        f.include_empty,
        q,
    )
    .fetch_all(pool)
    .await?;
    Ok(rows)
}

pub async fn list_spools(pool: &PgPool, filter: &SpoolFilter) -> AppResult<Vec<SpoolView>> {
    query_spools(pool, None, filter).await
}

pub async fn get_spool(pool: &PgPool, id: Uuid) -> AppResult<SpoolView> {
    query_spools(pool, Some(id), &SpoolFilter::default())
        .await?
        .into_iter()
        .next()
        .ok_or(AppError::NotFound("spool"))
}

pub async fn create_spool(pool: &PgPool, input: CreateSpool) -> AppResult<SpoolView> {
    if input.initial_weight_g <= Decimal::ZERO {
        return Err(AppError::BadRequest(
            "initial_weight_g must be positive".into(),
        ));
    }
    let remaining = input.remaining_weight_g.unwrap_or(input.initial_weight_g);
    let currency = input
        .currency
        .map(|c| c.trim().to_ascii_uppercase())
        .filter(|c| !c.is_empty());

    let mut tx = pool.begin().await?;
    let filament_id = match (input.filament_id, input.filament) {
        (Some(id), None) => id,
        (None, Some(new)) => filaments::find_or_create(&mut tx, new).await?,
        _ => {
            return Err(AppError::BadRequest(
                "provide exactly one of filament_id or filament".into(),
            ));
        }
    };

    let spool_id = sqlx::query_scalar!(
        r#"
        INSERT INTO spools (
            filament_id, label_code, location_id, status, initial_weight_g, remaining_weight_g,
            tare_weight_g, purchase_price, currency, purchase_date, vendor, lot_number, notes,
            acquisition, discounted, discount_note, opened_at, emptied_at
        ) VALUES (
            $1, $2, $3, $4::spool_status, $5, $6, $7, $8,
            coalesce($9, (SELECT default_currency FROM app_settings)),
            $10, $11, $12, $13, $14::acquisition_kind, $15, CASE WHEN $15 THEN $16 END,
            CASE WHEN $4::spool_status IN ('active', 'drying') THEN now() END,
            CASE WHEN $4::spool_status = 'empty' THEN now() END
        )
        RETURNING id
        "#,
        filament_id,
        trim_opt(input.label_code),
        input.location_id,
        input.status as SpoolStatus,
        input.initial_weight_g,
        remaining,
        input.tare_weight_g,
        input.purchase_price,
        currency,
        input.purchase_date,
        trim_opt(input.vendor),
        trim_opt(input.lot_number),
        trim_opt(input.notes),
        input.acquisition as AcquisitionKind,
        input.discounted,
        trim_opt(input.discount_note),
    )
    .fetch_one(&mut *tx)
    .await?;

    sqlx::query!(
        r#"
        INSERT INTO spool_weight_events (spool_id, kind, delta_g, weight_after_g)
        VALUES ($1, 'initial', $2, $2)
        "#,
        spool_id,
        remaining
    )
    .execute(&mut *tx)
    .await?;

    tx.commit().await?;
    get_spool(pool, spool_id).await
}

pub async fn update_spool(pool: &PgPool, id: Uuid, input: UpdateSpool) -> AppResult<SpoolView> {
    let res = sqlx::query!(
        r#"
        UPDATE spools SET
            filament_id      = $2,
            label_code       = $3,
            location_id      = $4,
            status           = $5::spool_status,
            initial_weight_g = $6,
            tare_weight_g    = $7,
            purchase_price   = $8,
            currency         = upper($9),
            purchase_date    = $10,
            vendor           = $11,
            lot_number       = $12,
            last_dried_at    = $13,
            notes            = $14,
            acquisition      = $15::acquisition_kind,
            discounted       = $16,
            discount_note    = CASE WHEN $16 THEN $17 END,
            opened_at  = CASE WHEN $5::spool_status IN ('active', 'drying', 'empty')
                              THEN coalesce(opened_at, now()) ELSE opened_at END,
            emptied_at = CASE WHEN $5::spool_status = 'empty' THEN coalesce(emptied_at, now()) END
        WHERE id = $1
        "#,
        id,
        input.filament_id,
        trim_opt(input.label_code),
        input.location_id,
        input.status as SpoolStatus,
        input.initial_weight_g,
        input.tare_weight_g,
        input.purchase_price,
        input.currency.trim(),
        input.purchase_date,
        trim_opt(input.vendor),
        trim_opt(input.lot_number),
        input.last_dried_at,
        trim_opt(input.notes),
        input.acquisition as AcquisitionKind,
        input.discounted,
        trim_opt(input.discount_note),
    )
    .execute(pool)
    .await?;
    if res.rows_affected() == 0 {
        return Err(AppError::NotFound("spool"));
    }
    get_spool(pool, id).await
}

pub async fn delete_spool(pool: &PgPool, id: Uuid) -> AppResult<()> {
    let res = sqlx::query!("DELETE FROM spools WHERE id = $1", id)
        .execute(pool)
        .await?;
    if res.rows_affected() == 0 {
        return Err(AppError::NotFound("spool"));
    }
    Ok(())
}

/// Result of a weight change: the updated spool and the logged event.
#[derive(Debug, serde::Serialize)]
pub struct WeightChange {
    pub spool: SpoolView,
    pub event: WeightEvent,
}

/// Applies a deduction, correction or scale reading in one transaction,
/// logging the event and moving the status along (sealed → active → empty).
pub async fn adjust_weight(
    pool: &PgPool,
    spool_id: Uuid,
    input: WeightAdjustment,
) -> AppResult<WeightChange> {
    let mut tx = pool.begin().await?;
    let cur = sqlx::query!(
        r#"
        SELECT remaining_weight_g, initial_weight_g, tare_weight_g,
               status AS "status: SpoolStatus"
        FROM spools WHERE id = $1 FOR UPDATE
        "#,
        spool_id
    )
    .fetch_optional(&mut *tx)
    .await?
    .ok_or(AppError::NotFound("spool"))?;

    let target = compute_new_weight(
        input.kind,
        cur.remaining_weight_g,
        cur.tare_weight_g,
        input.delta_g,
        input.gross_reading_g,
    )?;
    let max = (cur.initial_weight_g * MAX_OVERFILL).round_dp(1);
    if target > max {
        return Err(AppError::BadRequest(format!(
            "remaining weight {target} g would exceed {max} g (initial weight + 10%); \
             fix the initial weight first"
        )));
    }
    let delta = target - cur.remaining_weight_g;
    let new_status = next_status(cur.status, target, delta);

    sqlx::query!(
        r#"
        UPDATE spools SET
            remaining_weight_g = $2,
            status     = $3::spool_status,
            opened_at  = CASE WHEN $3::spool_status IN ('active', 'drying', 'empty')
                              THEN coalesce(opened_at, now()) ELSE opened_at END,
            emptied_at = CASE WHEN $3::spool_status = 'empty' THEN coalesce(emptied_at, now()) END
        WHERE id = $1
        "#,
        spool_id,
        target,
        new_status as SpoolStatus,
    )
    .execute(&mut *tx)
    .await?;

    let event = sqlx::query_as!(
        WeightEvent,
        r#"
        INSERT INTO spool_weight_events
            (spool_id, kind, delta_g, weight_after_g, gross_reading_g, print_name, note)
        VALUES ($1, $2, $3, $4, $5, $6, $7)
        RETURNING id, spool_id, kind AS "kind: WeightEventKind", delta_g, weight_after_g,
                  gross_reading_g, print_name, note, created_at
        "#,
        spool_id,
        input.kind as WeightEventKind,
        delta,
        target,
        input.gross_reading_g,
        trim_opt(input.print_name),
        trim_opt(input.note),
    )
    .fetch_one(&mut *tx)
    .await?;

    tx.commit().await?;
    Ok(WeightChange {
        spool: get_spool(pool, spool_id).await?,
        event,
    })
}

pub async fn weight_history(
    pool: &PgPool,
    spool_id: Uuid,
    limit: i64,
) -> AppResult<Vec<WeightEvent>> {
    Ok(sqlx::query_as!(
        WeightEvent,
        r#"
        SELECT id, spool_id, kind AS "kind: WeightEventKind", delta_g, weight_after_g,
               gross_reading_g, print_name, note, created_at
        FROM spool_weight_events
        WHERE spool_id = $1
        ORDER BY created_at DESC, id DESC
        LIMIT $2
        "#,
        spool_id,
        limit.clamp(1, 500)
    )
    .fetch_all(pool)
    .await?)
}

/// Pure weight math, kept separate so it is unit-testable without a database.
pub fn compute_new_weight(
    kind: WeightEventKind,
    current: Decimal,
    tare: Option<Decimal>,
    delta: Option<Decimal>,
    gross: Option<Decimal>,
) -> AppResult<Decimal> {
    let raw = match kind {
        WeightEventKind::WeighIn => {
            let gross = gross.ok_or_else(|| {
                AppError::BadRequest("gross_reading_g is required for weigh_in".into())
            })?;
            let tare = tare.ok_or_else(|| {
                AppError::BadRequest("set the spool's tare weight before weighing in".into())
            })?;
            gross - tare
        }
        WeightEventKind::Initial => {
            return Err(AppError::BadRequest(
                "initial events are created automatically".into(),
            ));
        }
        WeightEventKind::Print | WeightEventKind::QuickAdjust | WeightEventKind::Correction => {
            let delta = delta
                .filter(|d| !d.is_zero())
                .ok_or_else(|| AppError::BadRequest("delta_g must be a non-zero number".into()))?;
            if kind != WeightEventKind::Correction && delta > Decimal::ZERO {
                return Err(AppError::BadRequest(
                    "print and quick_adjust deduct weight; send a negative delta_g".into(),
                ));
            }
            current + delta
        }
    };
    Ok(raw.max(Decimal::ZERO).round_dp(1))
}

fn next_status(current: SpoolStatus, new_weight: Decimal, delta: Decimal) -> SpoolStatus {
    if new_weight.is_zero() {
        return SpoolStatus::Empty;
    }
    match current {
        SpoolStatus::Sealed if delta < Decimal::ZERO => SpoolStatus::Active,
        SpoolStatus::Empty => SpoolStatus::Active,
        other => other,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal_macros::dec;

    #[test]
    fn deduction_clamps_at_zero() {
        let w = compute_new_weight(
            WeightEventKind::QuickAdjust,
            dec!(30),
            None,
            Some(dec!(-50)),
            None,
        )
        .unwrap();
        assert_eq!(w, dec!(0));
    }

    #[test]
    fn weigh_in_subtracts_tare() {
        let w = compute_new_weight(
            WeightEventKind::WeighIn,
            dec!(900),
            Some(dec!(250)),
            None,
            Some(dec!(1012.4)),
        )
        .unwrap();
        assert_eq!(w, dec!(762.4));
    }

    #[test]
    fn weigh_in_needs_tare() {
        assert!(
            compute_new_weight(
                WeightEventKind::WeighIn,
                dec!(900),
                None,
                None,
                Some(dec!(1000))
            )
            .is_err()
        );
    }

    #[test]
    fn print_rejects_positive_delta() {
        assert!(
            compute_new_weight(
                WeightEventKind::Print,
                dec!(900),
                None,
                Some(dec!(10)),
                None
            )
            .is_err()
        );
        assert!(
            compute_new_weight(
                WeightEventKind::Correction,
                dec!(900),
                None,
                Some(dec!(10)),
                None
            )
            .is_ok()
        );
    }

    #[test]
    fn status_transitions() {
        assert_eq!(
            next_status(SpoolStatus::Sealed, dec!(990), dec!(-10)),
            SpoolStatus::Active
        );
        assert_eq!(
            next_status(SpoolStatus::Active, dec!(0), dec!(-5)),
            SpoolStatus::Empty
        );
        assert_eq!(
            next_status(SpoolStatus::Empty, dec!(40), dec!(40)),
            SpoolStatus::Active
        );
        assert_eq!(
            next_status(SpoolStatus::Drying, dec!(500), dec!(-10)),
            SpoolStatus::Drying
        );
    }
}
