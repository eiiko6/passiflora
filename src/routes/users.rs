use axum::{
    Extension, Json, Router, extract::Request, http::StatusCode, middleware::Next,
    response::Response, routing::post,
};
use sqlx::PgPool;
use std::env;
use validator::ValidateEmail;

use crate::auth::{create_jwt, hash_password, verify_password};

const DUMMY_HASH: &str = "$argon2id$v=19$m=4096,t=3,p=1$YWFhYWFhYWFhYWFhYWFhYQ$aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";

#[derive(sqlx::FromRow, serde::Serialize)]
pub struct User {
    pub id: i32,
    pub username: String,
    pub password_hash: String,
    pub email: String,
}

#[derive(serde::Deserialize)]
pub struct LoginPayload {
    pub email: String,
    pub password: String,
}

#[derive(serde::Serialize)]
pub struct LoginResponse {
    pub id: i32,
    pub email: String,
    pub token: String,
}

#[derive(serde::Deserialize)]
pub struct NewUserPayload {
    pub email: String,
    pub username: String,
    pub password: String,
}

pub fn routes() -> Router {
    Router::new()
        .route("/login", post(login))
        .route("/register", post(register_user))
        .layer(axum::middleware::from_fn(registration_guard))
}

async fn registration_guard(req: Request, next: Next) -> Result<Response, StatusCode> {
    if req.uri().path() == "/register"
        && env::var("PASSIFLORA_ALLOW_REGISTRATION").map_or(true, |v| v.to_lowercase() == "false")
    {
        return Err(StatusCode::FORBIDDEN);
    }
    Ok(next.run(req).await)
}

pub async fn login(
    Extension(db): Extension<PgPool>,
    Json(payload): Json<LoginPayload>,
) -> Result<Json<LoginResponse>, (StatusCode, String)> {
    let user = sqlx::query_as::<_, User>("SELECT * FROM users WHERE email = $1")
        .bind(&payload.email)
        .fetch_optional(&db)
        .await
        .map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, "DB error".into()))?;

    let (user_id, password_hash) = if let Some(u) = user {
        (u.id, u.password_hash)
    } else {
        // timing shield
        (0, DUMMY_HASH.to_string())
    };

    if !verify_password(&password_hash, &payload.password) || user_id == 0 {
        return Err((StatusCode::UNAUTHORIZED, "Invalid credentials".into()));
    }

    let token = create_jwt(user_id).map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e))?;

    Ok(Json(LoginResponse {
        id: user_id,
        email: payload.email,
        token,
    }))
}

pub async fn register_user(
    Extension(db): Extension<PgPool>,
    Json(payload): Json<NewUserPayload>,
) -> Result<StatusCode, (StatusCode, String)> {
    if payload.email.is_empty() || payload.username.is_empty() || payload.password.is_empty() {
        return Err((
            StatusCode::BAD_REQUEST,
            "Cannot create a user with empty fields".into(),
        ));
    }

    if !ValidateEmail::validate_email(&payload.email) {
        return Err((StatusCode::BAD_REQUEST, "Invalid email format".into()));
    }

    if payload.password.len() < 8 {
        return Err((
            StatusCode::BAD_REQUEST,
            "Password must be at least 8 characters long".into(),
        ));
    }

    let password_hash =
        hash_password(&payload.password).map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e))?;

    let result =
        sqlx::query("INSERT INTO users (username, email, password_hash) VALUES ($1, $2, $3)")
            .bind(&payload.username)
            .bind(&payload.email)
            .bind(&password_hash)
            .execute(&db)
            .await;

    match result {
        Ok(_) => Ok(StatusCode::CREATED),
        Err(e) => {
            if let Some(db_err) = e.as_database_error() {
                if db_err.code().map(|c| c == "23505").unwrap_or(false) {
                    return Err((
                        StatusCode::CONFLICT,
                        "Email or username already taken".into(),
                    ));
                }
            }
            Err((StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))
        }
    }
}
