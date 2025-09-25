use axum::{
    Extension, Json, Router,
    extract::Path,
    http::{HeaderMap, StatusCode},
    routing::get,
    routing::post,
};
use sqlx::PgPool;

use crate::auth::verify_jwt;

#[derive(sqlx::FromRow, serde::Serialize)]
pub struct Item {
    pub id: i32,
    pub user_id: i32,
    pub name: String,
    pub value: String,
}

#[derive(serde::Deserialize)]
pub struct NewItemPayload {
    pub user_id: i32,
    pub name: String,
    pub value: String,
}

pub fn routes() -> Router {
    Router::new()
        .route("/items/{user_id}", get(get_items_for_user))
        .route("/items", post(create_item))
}

async fn get_items_for_user(
    Path(user_id): Path<i32>,
    headers: HeaderMap,
    Extension(db): Extension<PgPool>,
) -> Result<Json<Vec<Item>>, (StatusCode, String)> {
    let token = headers
        .get("Authorization")
        .and_then(|v| v.to_str().ok())
        .and_then(|s| s.strip_prefix("Bearer "))
        .ok_or((StatusCode::UNAUTHORIZED, "Missing token".to_string()))?;

    let claims = verify_jwt(token)?;
    if claims.sub != user_id {
        return Err((StatusCode::FORBIDDEN, "Forbidden".to_string()));
    }

    let items = sqlx::query_as::<_, Item>("SELECT * FROM items WHERE user_id = $1")
        .bind(user_id)
        .fetch_all(&db)
        .await
        .unwrap_or_default();

    Ok(Json(items))
}

async fn create_item(
    Extension(db): Extension<PgPool>,
    headers: HeaderMap,
    Json(payload): Json<NewItemPayload>,
) -> Result<StatusCode, (StatusCode, String)> {
    let token = headers
        .get("Authorization")
        .and_then(|v| v.to_str().ok())
        .and_then(|s| s.strip_prefix("Bearer "))
        .ok_or((StatusCode::UNAUTHORIZED, "Missing token".to_string()))?;

    let claims = verify_jwt(token)?;
    if claims.sub != payload.user_id {
        return Err((StatusCode::FORBIDDEN, "Forbidden".to_string()));
    }

    if payload.name.is_empty() {
        return Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            "Name cannot be empty".into(),
        ));
    }

    let result = sqlx::query("INSERT INTO items (user_id, name, value) VALUES ($1, $2, $3, $4)")
        .bind(payload.user_id)
        .bind(payload.name)
        .bind(payload.value)
        .execute(&db)
        .await;

    match result {
        Ok(_) => Ok(StatusCode::CREATED),
        Err(_) => Err((StatusCode::INTERNAL_SERVER_ERROR, "Database error".into())),
    }
}
