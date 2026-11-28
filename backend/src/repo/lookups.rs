use crate::{
    error::{AppError, AppResult},
    models::{
        Access, Brand, Location, LocationKind, LocationShare, Material, NewLocation, ShareAccess,
        ShareLocation,
    },
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

/// Your locations, then the ones shared with you.
pub async fn list_locations(pool: &PgPool, me: Uuid) -> AppResult<Vec<Location>> {
    let rows = sqlx::query!(
        r#"
        SELECT l.id, l.name, l.kind AS "kind: LocationKind", l.description, l.sort_order,
               CASE WHEN l.owner_id = $1 THEN 'owner' ELSE ls.access::text END AS "access!",
               CASE WHEN l.owner_id = $1 THEN NULL ELSE u.username END AS owner_username,
               (SELECT count(*) FROM location_shares x WHERE x.location_id = l.id
                  AND l.owner_id = $1) AS "share_count!"
        FROM locations l
        LEFT JOIN location_shares ls ON ls.location_id = l.id AND ls.user_id = $1
        LEFT JOIN users u ON u.id = l.owner_id
        WHERE l.owner_id = $1 OR ls.user_id IS NOT NULL
        ORDER BY l.owner_id <> $1, u.username, l.sort_order, lower(l.name)
        "#,
        me
    )
    .fetch_all(pool)
    .await?;
    Ok(rows
        .into_iter()
        .map(|r| Location {
            id: r.id,
            name: r.name,
            kind: r.kind,
            description: r.description,
            sort_order: r.sort_order,
            access: Access::parse(Some(&r.access)).unwrap_or(Access::View),
            owner_username: r.owner_username,
            share_count: r.share_count,
        })
        .collect())
}

pub async fn get_location(pool: &PgPool, me: Uuid, id: Uuid) -> AppResult<Location> {
    list_locations(pool, me)
        .await?
        .into_iter()
        .find(|l| l.id == id)
        .ok_or(AppError::NotFound("location"))
}

/// The owner of a location you can add spools to: your own, or one shared
/// with you to edit.
pub async fn editable_location_owner(pool: &PgPool, me: Uuid, id: Uuid) -> AppResult<Uuid> {
    sqlx::query_scalar!(
        r#"
        SELECT l.owner_id AS "owner_id!" FROM locations l
        WHERE l.id = $1 AND l.owner_id IS NOT NULL AND (l.owner_id = $2 OR EXISTS (
            SELECT 1 FROM location_shares s
            WHERE s.location_id = l.id AND s.user_id = $2 AND s.access = 'edit'))
        "#,
        id,
        me
    )
    .fetch_optional(pool)
    .await?
    .ok_or_else(|| AppError::BadRequest("You can't put spools in that location.".into()))
}

pub async fn create_location(pool: &PgPool, me: Uuid, input: NewLocation) -> AppResult<Location> {
    let name = input.name.trim();
    if name.is_empty() {
        return Err(AppError::BadRequest("name is required".into()));
    }
    let id = sqlx::query_scalar!(
        r#"
        INSERT INTO locations (owner_id, name, kind, description, sort_order)
        VALUES ($1, $2, $3, $4, $5)
        RETURNING id
        "#,
        me,
        name,
        input.kind as _,
        input.description,
        input.sort_order
    )
    .fetch_one(pool)
    .await?;
    get_location(pool, me, id).await
}

pub async fn delete_location(pool: &PgPool, me: Uuid, id: Uuid) -> AppResult<()> {
    let res = sqlx::query!(
        "DELETE FROM locations WHERE id = $1 AND owner_id = $2",
        id,
        me
    )
    .execute(pool)
    .await?;
    if res.rows_affected() == 0 {
        return Err(AppError::NotFound("location"));
    }
    Ok(())
}

pub async fn update_location(
    pool: &PgPool,
    me: Uuid,
    id: Uuid,
    input: NewLocation,
) -> AppResult<Location> {
    let name = input.name.trim();
    if name.is_empty() {
        return Err(AppError::BadRequest("name is required".into()));
    }
    let res = sqlx::query!(
        r#"
        UPDATE locations SET name = $3, kind = $4, description = $5, sort_order = $6
        WHERE id = $1 AND owner_id = $2
        "#,
        id,
        me,
        name,
        input.kind as _,
        crate::models::trim_opt(input.description),
        input.sort_order
    )
    .execute(pool)
    .await?;
    if res.rows_affected() == 0 {
        return Err(AppError::NotFound("location"));
    }
    get_location(pool, me, id).await
}

async fn own_location(pool: &PgPool, me: Uuid, id: Uuid) -> AppResult<()> {
    let mine = sqlx::query_scalar!(
        r#"SELECT EXISTS (SELECT 1 FROM locations WHERE id = $1 AND owner_id = $2) AS "mine!""#,
        id,
        me
    )
    .fetch_one(pool)
    .await?;
    if !mine {
        return Err(AppError::NotFound("location"));
    }
    Ok(())
}

/// Who your location is shared with.
pub async fn list_shares(pool: &PgPool, me: Uuid, id: Uuid) -> AppResult<Vec<LocationShare>> {
    own_location(pool, me, id).await?;
    Ok(sqlx::query_as!(
        LocationShare,
        r#"
        SELECT s.user_id, u.username, s.access AS "access: ShareAccess"
        FROM location_shares s JOIN users u ON u.id = s.user_id
        WHERE s.location_id = $1
        ORDER BY lower(u.username)
        "#,
        id
    )
    .fetch_all(pool)
    .await?)
}

/// Shares your location with an account, or changes its access.
pub async fn share(
    pool: &PgPool,
    me: Uuid,
    id: Uuid,
    input: ShareLocation,
) -> AppResult<Vec<LocationShare>> {
    own_location(pool, me, id).await?;
    let user = sqlx::query_scalar!(
        "SELECT id FROM users WHERE lower(username) = lower($1)",
        input.username.trim()
    )
    .fetch_optional(pool)
    .await?
    .ok_or_else(|| {
        AppError::BadRequest(format!(
            "There's no account called {}.",
            input.username.trim()
        ))
    })?;
    if user == me {
        return Err(AppError::BadRequest("That's your own account.".into()));
    }
    sqlx::query!(
        r#"
        INSERT INTO location_shares (location_id, user_id, access) VALUES ($1, $2, $3)
        ON CONFLICT (location_id, user_id) DO UPDATE SET access = EXCLUDED.access
        "#,
        id,
        user,
        input.access as ShareAccess
    )
    .execute(pool)
    .await?;
    list_shares(pool, me, id).await
}

/// Stops sharing your location with an account. The account can also
/// leave a location shared with it (`user` = itself).
pub async fn unshare(pool: &PgPool, me: Uuid, id: Uuid, user: Uuid) -> AppResult<()> {
    let res = sqlx::query!(
        r#"
        DELETE FROM location_shares s USING locations l
        WHERE s.location_id = $1 AND s.user_id = $2 AND l.id = s.location_id
          AND (l.owner_id = $3 OR s.user_id = $3)
        "#,
        id,
        user,
        me
    )
    .execute(pool)
    .await?;
    if res.rows_affected() == 0 {
        return Err(AppError::NotFound("share"));
    }
    Ok(())
}
