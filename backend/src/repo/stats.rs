use crate::{error::AppResult, models::InventoryStats};
use sqlx::PgPool;

pub async fn inventory_stats(pool: &PgPool) -> AppResult<InventoryStats> {
    let row = sqlx::query_as!(
        InventoryStats,
        r#"
        SELECT
            v.spools_in_stock   AS "spools_in_stock!",
            v.spools_sealed     AS "spools_sealed!",
            v.spools_active     AS "spools_active!",
            v.spools_drying     AS "spools_drying!",
            v.spools_empty      AS "spools_empty!",
            v.remaining_kg      AS "remaining_kg!",
            v.low_stock_count   AS "low_stock_count!",
            st.low_stock_threshold_g
        FROM inventory_stats v
        CROSS JOIN app_settings st
        "#
    )
    .fetch_one(pool)
    .await?;
    Ok(row)
}
