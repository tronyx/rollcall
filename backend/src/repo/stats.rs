use crate::{error::AppResult, models::InventoryStats};
use sqlx::PgPool;
use uuid::Uuid;

/// Totals over the spools you can see: yours and those in locations shared
/// with you. Each spool is low by its owner's threshold.
pub async fn inventory_stats(pool: &PgPool, me: Uuid) -> AppResult<InventoryStats> {
    let row = sqlx::query_as!(
        InventoryStats,
        r#"
        WITH v AS (
            SELECT s.status, s.remaining_weight_g,
                   (s.status NOT IN ('empty', 'archived')
                    AND s.remaining_weight_g < st.low_stock_threshold_g) AS is_low_stock
            FROM spools s
            JOIN user_settings st ON st.user_id = s.owner_id
            WHERE spool_access(s.owner_id, s.location_id, $1) IS NOT NULL
        )
        SELECT
            count(*) FILTER (WHERE status NOT IN ('empty', 'archived')) AS "spools_in_stock!",
            count(*) FILTER (WHERE status = 'sealed')                   AS "spools_sealed!",
            count(*) FILTER (WHERE status = 'active')                   AS "spools_active!",
            count(*) FILTER (WHERE status = 'drying')                   AS "spools_drying!",
            count(*) FILTER (WHERE status = 'empty')                    AS "spools_empty!",
            coalesce(round(sum(remaining_weight_g)
                FILTER (WHERE status NOT IN ('empty', 'archived')) / 1000, 2), 0) AS "remaining_kg!",
            count(*) FILTER (WHERE is_low_stock)                        AS "low_stock_count!",
            (SELECT low_stock_threshold_g FROM user_settings WHERE user_id = $1)
                                                                        AS "low_stock_threshold_g!"
        FROM v
        "#,
        me
    )
    .fetch_one(pool)
    .await?;
    Ok(row)
}
