use crate::{
    error::{AppError, AppResult},
    gcode::SlicedPrint,
    models::{
        AssignSlot, Print, PrintFilter, PrintSlot, PrintSlotStatus, PrintSpool, SlicerProfile,
        SpoolStatus, WeightAdjustment, WeightEventKind,
    },
    repo::spools,
};
use rust_decimal::Decimal;
use sqlx::PgPool;
use uuid::Uuid;

/// The same upload within this many minutes is the same print.
const DUPLICATE_MINUTES: i32 = 10;

/// Records a print and logs each slot whose spool is known.
/// Returns the print and whether it is new (false for a repeated upload).
pub async fn create(
    pool: &PgPool,
    me: Uuid,
    name: &str,
    sliced: &SlicedPrint,
) -> AppResult<(Print, bool)> {
    if sliced.slots.is_empty() {
        return Err(AppError::BadRequest(
            "no filament use found; send G-code from OrcaSlicer or Bambu Studio".into(),
        ));
    }
    let fingerprint = fingerprint(name, sliced);
    let existing = sqlx::query_scalar!(
        r#"
        SELECT id FROM prints
        WHERE owner_id = $3 AND fingerprint = $1
          AND created_at > now() - make_interval(mins => $2)
        ORDER BY created_at DESC LIMIT 1
        "#,
        fingerprint,
        DUPLICATE_MINUTES,
        me
    )
    .fetch_optional(pool)
    .await?;
    if let Some(id) = existing {
        return Ok((get(pool, me, id).await?, false));
    }

    let mut tx = pool.begin().await?;
    let id = sqlx::query_scalar!(
        r#"
        INSERT INTO prints (name, slicer, printer_preset, fingerprint, owner_id)
        VALUES ($1, $2, $3, $4, $5) RETURNING id
        "#,
        name,
        sliced.slicer,
        sliced.printer,
        fingerprint,
        me
    )
    .fetch_one(&mut *tx)
    .await?;
    for s in &sliced.slots {
        sqlx::query!(
            r#"
            INSERT INTO print_slots (print_id, slot, grams, profile_name, filament_type, color_hex)
            VALUES ($1, $2, $3, $4, $5, $6)
            "#,
            id,
            s.slot,
            s.grams,
            s.profile,
            s.filament_type,
            s.color_hex
        )
        .execute(&mut *tx)
        .await?;
    }
    tx.commit().await?;

    for s in &sliced.slots {
        let Some(profile) = &s.profile else { continue };
        if let Some(spool_id) = match_spool(pool, me, profile).await? {
            log_slot(pool, me, id, s.slot, spool_id).await?;
        }
    }
    Ok((get(pool, me, id).await?, true))
}

fn fingerprint(name: &str, p: &SlicedPrint) -> String {
    let slots: Vec<String> = p
        .slots
        .iter()
        .map(|s| {
            format!(
                "{}:{}:{}",
                s.slot,
                s.grams,
                s.profile.as_deref().unwrap_or("")
            )
        })
        .collect();
    format!(
        "{name}|{}|{}|{}",
        p.slicer.as_deref().unwrap_or(""),
        p.printer.as_deref().unwrap_or(""),
        slots.join(",")
    )
}

/// The spool a slicer preset prints from: the one active (or drying) spool of
/// its filament, or failing that its one sealed spool. None when it's unclear.
/// Only your presets, and spools you can change, count.
pub async fn match_spool(pool: &PgPool, me: Uuid, profile: &str) -> AppResult<Option<Uuid>> {
    let rows = sqlx::query!(
        r#"
        SELECT s.id, s.status AS "status: SpoolStatus"
        FROM slicer_profiles sp
        JOIN spools s ON s.filament_id = sp.filament_id
        WHERE sp.owner_id = $2 AND sp.name = $1 AND s.status IN ('active', 'drying', 'sealed')
          AND spool_access(s.owner_id, s.location_id, $2) IN ('owner', 'edit')
        "#,
        profile,
        me
    )
    .fetch_all(pool)
    .await?;
    let open: Vec<Uuid> = rows
        .iter()
        .filter(|r| r.status != SpoolStatus::Sealed)
        .map(|r| r.id)
        .collect();
    let sealed: Vec<Uuid> = rows
        .iter()
        .filter(|r| r.status == SpoolStatus::Sealed)
        .map(|r| r.id)
        .collect();
    Ok(match (open.as_slice(), sealed.as_slice()) {
        ([one], _) => Some(*one),
        ([], [one]) => Some(*one),
        _ => None,
    })
}

struct SlotRow {
    grams: Decimal,
    status: PrintSlotStatus,
    print_name: String,
    weight_event_id: Option<i64>,
}

async fn slot_row(pool: &PgPool, print_id: Uuid, slot: i16) -> AppResult<SlotRow> {
    sqlx::query_as!(
        SlotRow,
        r#"
        SELECT ps.grams, ps.status AS "status: PrintSlotStatus", p.name AS print_name,
               ps.weight_event_id
        FROM print_slots ps JOIN prints p ON p.id = ps.print_id
        WHERE ps.print_id = $1 AND ps.slot = $2
        "#,
        print_id,
        slot
    )
    .fetch_optional(pool)
    .await?
    .ok_or(AppError::NotFound("print slot"))
}

/// Deducts a pending slot's grams from the spool.
async fn log_slot(
    pool: &PgPool,
    me: Uuid,
    print_id: Uuid,
    slot: i16,
    spool_id: Uuid,
) -> AppResult<()> {
    let row = slot_row(pool, print_id, slot).await?;
    if row.status != PrintSlotStatus::Pending {
        return Err(AppError::Conflict(
            "this filament is already logged or skipped; undo it first".into(),
        ));
    }
    // Weights are kept to 0.1 g; never round a real use down to nothing.
    let grams = row.grams.round_dp(1).max(Decimal::new(1, 1));
    let change = spools::adjust_weight(
        pool,
        me,
        spool_id,
        WeightAdjustment {
            kind: WeightEventKind::Print,
            delta_g: Some(-grams),
            gross_reading_g: None,
            print_name: Some(row.print_name),
            note: Some("Logged from the slicer".into()),
        },
    )
    .await?;
    sqlx::query!(
        r#"
        UPDATE print_slots SET status = 'logged', spool_id = $3, weight_event_id = $4
        WHERE print_id = $1 AND slot = $2
        "#,
        print_id,
        slot,
        spool_id,
        change.event.id
    )
    .execute(pool)
    .await?;
    Ok(())
}

/// Picks the spool for a pending slot, or skips it.
pub async fn assign(
    pool: &PgPool,
    me: Uuid,
    print_id: Uuid,
    slot: i16,
    body: AssignSlot,
) -> AppResult<Print> {
    get(pool, me, print_id).await?;
    match (body.spool_id, body.skip) {
        (Some(spool_id), false) => {
            log_slot(pool, me, print_id, slot, spool_id).await?;
            if body.remember.unwrap_or(true) {
                sqlx::query!(
                    r#"
                    INSERT INTO slicer_profiles (name, filament_id, owner_id)
                    SELECT ps.profile_name, s.filament_id, $4
                    FROM print_slots ps, spools s
                    WHERE ps.print_id = $1 AND ps.slot = $2 AND s.id = $3
                      AND ps.profile_name IS NOT NULL
                    ON CONFLICT (owner_id, name) DO UPDATE
                        SET filament_id = EXCLUDED.filament_id, updated_at = now()
                    "#,
                    print_id,
                    slot,
                    spool_id,
                    me
                )
                .execute(pool)
                .await?;
            }
        }
        (None, true) => {
            let row = slot_row(pool, print_id, slot).await?;
            if row.status != PrintSlotStatus::Pending {
                return Err(AppError::Conflict(
                    "this filament is already logged or skipped; undo it first".into(),
                ));
            }
            sqlx::query!(
                "UPDATE print_slots SET status = 'skipped' WHERE print_id = $1 AND slot = $2",
                print_id,
                slot
            )
            .execute(pool)
            .await?;
        }
        _ => {
            return Err(AppError::BadRequest(
                "give either spool_id or skip: true".into(),
            ));
        }
    }
    get(pool, me, print_id).await
}

/// Puts a slot back to pending, adding its grams back to the spool if they
/// were deducted.
pub async fn undo(pool: &PgPool, me: Uuid, print_id: Uuid, slot: i16) -> AppResult<Print> {
    get(pool, me, print_id).await?;
    undo_slot(pool, me, print_id, slot).await?;
    get(pool, me, print_id).await
}

async fn undo_slot(pool: &PgPool, me: Uuid, print_id: Uuid, slot: i16) -> AppResult<()> {
    let row = slot_row(pool, print_id, slot).await?;
    if row.status == PrintSlotStatus::Logged
        && let Some(event_id) = row.weight_event_id
    {
        // The event is gone if its spool was deleted; nothing to give back then.
        let event = sqlx::query!(
            "SELECT spool_id, delta_g FROM spool_weight_events WHERE id = $1",
            event_id
        )
        .fetch_optional(pool)
        .await?;
        if let Some(e) = event {
            spools::adjust_weight(
                pool,
                me,
                e.spool_id,
                WeightAdjustment {
                    kind: WeightEventKind::Correction,
                    delta_g: Some(-e.delta_g),
                    gross_reading_g: None,
                    print_name: None,
                    note: Some(format!("Undo print: {}", row.print_name)),
                },
            )
            .await?;
        }
    }
    sqlx::query!(
        r#"
        UPDATE print_slots SET status = 'pending', spool_id = NULL, weight_event_id = NULL
        WHERE print_id = $1 AND slot = $2
        "#,
        print_id,
        slot
    )
    .execute(pool)
    .await?;
    Ok(())
}

/// Removes a print, giving back the filament it took.
pub async fn delete(pool: &PgPool, me: Uuid, id: Uuid) -> AppResult<()> {
    get(pool, me, id).await?;
    let slots = sqlx::query_scalar!(
        "SELECT slot FROM print_slots WHERE print_id = $1 AND status = 'logged'",
        id
    )
    .fetch_all(pool)
    .await?;
    for slot in slots {
        undo_slot(pool, me, id, slot).await?;
    }
    let res = sqlx::query!("DELETE FROM prints WHERE id = $1", id)
        .execute(pool)
        .await?;
    if res.rows_affected() == 0 {
        return Err(AppError::NotFound("print"));
    }
    Ok(())
}

pub async fn get(pool: &PgPool, me: Uuid, id: Uuid) -> AppResult<Print> {
    let mut found = load(pool, me, Some(id), false, 1).await?;
    found.pop().ok_or(AppError::NotFound("print"))
}

pub async fn list(pool: &PgPool, me: Uuid, f: &PrintFilter) -> AppResult<Vec<Print>> {
    load(
        pool,
        me,
        None,
        f.pending,
        f.limit.unwrap_or(50).clamp(1, 500),
    )
    .await
}

async fn load(
    pool: &PgPool,
    me: Uuid,
    id: Option<Uuid>,
    pending: bool,
    limit: i64,
) -> AppResult<Vec<Print>> {
    let prints = sqlx::query!(
        r#"
        SELECT p.id, p.name, p.slicer, p.printer_preset, p.created_at
        FROM prints p
        WHERE p.owner_id = $4 AND ($1::uuid IS NULL OR p.id = $1)
          AND (NOT $2 OR EXISTS (SELECT 1 FROM print_slots ps
                                 WHERE ps.print_id = p.id AND ps.status = 'pending'))
        ORDER BY p.created_at DESC, p.id DESC
        LIMIT $3
        "#,
        id,
        pending,
        limit,
        me
    )
    .fetch_all(pool)
    .await?;
    let ids: Vec<Uuid> = prints.iter().map(|p| p.id).collect();
    let rows = sqlx::query!(
        r#"
        SELECT ps.print_id, ps.slot, ps.grams, ps.profile_name, ps.filament_type, ps.color_hex,
               ps.status AS "status: PrintSlotStatus", ps.spool_id, ps.weight_event_id,
               sp.filament_id AS "profile_filament_id?",
               s.label_code, b.name AS "brand_name?", m.code AS "material_code?",
               f.product_name, f.color_name AS "color_name?", f.color_hex AS spool_color_hex,
               s.remaining_weight_g AS "remaining_weight_g?"
        FROM print_slots ps
        LEFT JOIN slicer_profiles sp ON sp.name = ps.profile_name AND sp.owner_id = $2
        LEFT JOIN spools s    ON s.id = ps.spool_id
        LEFT JOIN filaments f ON f.id = s.filament_id
        LEFT JOIN brands b    ON b.id = f.brand_id
        LEFT JOIN materials m ON m.id = f.material_id
        WHERE ps.print_id = ANY($1)
        ORDER BY ps.slot
        "#,
        &ids,
        me
    )
    .fetch_all(pool)
    .await?;
    let slots: Vec<PrintSlot> = rows
        .into_iter()
        .map(|r| PrintSlot {
            print_id: r.print_id,
            slot: r.slot,
            grams: r.grams,
            profile_name: r.profile_name,
            filament_type: r.filament_type,
            color_hex: r.color_hex,
            status: r.status,
            spool_id: r.spool_id,
            weight_event_id: r.weight_event_id,
            profile_filament_id: r.profile_filament_id,
            spool: match (
                r.brand_name,
                r.material_code,
                r.color_name,
                r.remaining_weight_g,
            ) {
                (Some(brand_name), Some(material_code), Some(color_name), Some(remaining)) => {
                    Some(PrintSpool {
                        label_code: r.label_code,
                        brand_name,
                        material_code,
                        product_name: r.product_name,
                        color_name,
                        color_hex: r.spool_color_hex,
                        remaining_weight_g: remaining,
                    })
                }
                _ => None,
            },
        })
        .collect();
    Ok(prints
        .into_iter()
        .map(|p| Print {
            slots: slots
                .iter()
                .filter(|s| s.print_id == p.id)
                .cloned()
                .collect(),
            id: p.id,
            name: p.name,
            slicer: p.slicer,
            printer_preset: p.printer_preset,
            created_at: p.created_at,
        })
        .collect())
}

/// Records that a slicer preset is this filament (on export from Rollcall).
pub async fn save_profile(pool: &PgPool, me: Uuid, p: &SlicerProfile) -> AppResult<SlicerProfile> {
    let name = p.name.trim();
    if name.is_empty() {
        return Err(AppError::BadRequest("name must not be empty".into()));
    }
    super::filaments::access(pool, me, p.filament_id).await?;
    sqlx::query!(
        r#"
        INSERT INTO slicer_profiles (name, filament_id, owner_id) VALUES ($1, $2, $3)
        ON CONFLICT (owner_id, name) DO UPDATE
            SET filament_id = EXCLUDED.filament_id, updated_at = now()
        "#,
        name,
        p.filament_id,
        me
    )
    .execute(pool)
    .await?;
    Ok(SlicerProfile {
        name: name.to_string(),
        filament_id: p.filament_id,
    })
}

pub async fn profiles(
    pool: &PgPool,
    me: Uuid,
    filament_id: Option<Uuid>,
) -> AppResult<Vec<SlicerProfile>> {
    Ok(sqlx::query_as!(
        SlicerProfile,
        r#"
        SELECT name, filament_id FROM slicer_profiles
        WHERE owner_id = $2 AND ($1::uuid IS NULL OR filament_id = $1)
        ORDER BY name
        "#,
        filament_id,
        me
    )
    .fetch_all(pool)
    .await?)
}

pub async fn delete_profile(pool: &PgPool, me: Uuid, name: &str) -> AppResult<()> {
    let res = sqlx::query!(
        "DELETE FROM slicer_profiles WHERE name = $1 AND owner_id = $2",
        name,
        me
    )
    .execute(pool)
    .await?;
    if res.rows_affected() == 0 {
        return Err(AppError::NotFound("slicer profile"));
    }
    Ok(())
}
