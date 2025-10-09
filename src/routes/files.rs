use axum::{
    Extension, Json, Router,
    body::Body,
    extract::{DefaultBodyLimit, Multipart, Path},
    http::{HeaderMap, StatusCode, header},
    response::Response,
    routing::{get, post},
};
use mime_guess::MimeGuess;
use sqlx::PgPool;
use std::path::PathBuf;
use tokio::io::AsyncWriteExt;
use tokio_util::io::ReaderStream;
use tower_http::limit::RequestBodyLimitLayer;

use crate::auth::verify_jwt;

const DEFAULT_UPLOAD_DIR: &str = "./uploads";

#[derive(sqlx::FromRow, serde::Serialize)]
pub struct File {
    pub id: i32,
    pub user_id: i32,
    pub name: String,
    pub mime_type: String,
    pub size: Option<i32>,
}

pub fn routes() -> Router {
    Router::new()
        .route("/files/{user_id}", get(list_files))
        .route("/files", post(upload_file))
        .layer(DefaultBodyLimit::disable())
        .layer(RequestBodyLimitLayer::new(10 * 1000 * 1000 * 10000)) // 100GB
        .route("/files/{user_id}/{file_id}", get(download_file))
}

async fn list_files(
    Path(user_id): Path<i32>,
    headers: HeaderMap,
    Extension(db): Extension<PgPool>,
) -> Result<Json<Vec<File>>, (StatusCode, String)> {
    let token = headers
        .get("Authorization")
        .and_then(|v| v.to_str().ok())
        .and_then(|s| s.strip_prefix("Bearer "))
        .ok_or((StatusCode::UNAUTHORIZED, "Missing token".to_string()))?;

    let claims = verify_jwt(token)?;
    if claims.sub != user_id {
        return Err((StatusCode::FORBIDDEN, "Forbidden".to_string()));
    }

    let files = sqlx::query_as::<_, File>("SELECT * FROM files WHERE user_id = $1")
        .bind(user_id)
        .fetch_all(&db)
        .await
        .unwrap_or_default();

    Ok(Json(files))
}

async fn upload_file(
    Extension(db): Extension<PgPool>,
    headers: HeaderMap,
    mut multipart: Multipart,
) -> Result<StatusCode, (StatusCode, String)> {
    let token = headers
        .get("Authorization")
        .and_then(|v| v.to_str().ok())
        .and_then(|s| s.strip_prefix("Bearer "))
        .ok_or((StatusCode::UNAUTHORIZED, "Missing token".to_string()))?;

    // Verify auth
    let claims = verify_jwt(token)?;

    let upload_dir = std::env::var("PASSIFLORA_DATA_DIR").unwrap_or(DEFAULT_UPLOAD_DIR.to_string());

    while let Some(mut field) = multipart.next_field().await.unwrap() {
        let name = field.file_name().unwrap_or("file").to_string();
        let content_type =
            // field
            // .content_type()
            // .map(|s| s.to_string())
            // .or_else(|| {
                MimeGuess::from_path(&name)
                    .first_raw()
                    .map(|s| s.to_string())
            // })
            .unwrap();

        let result = sqlx::query_as::<_, File>(
            "INSERT INTO files (user_id, name, mime_type) VALUES ($1, $2, $3) RETURNING *",
        )
        .bind(claims.sub)
        .bind(&name)
        .bind(content_type)
        .fetch_one(&db)
        .await
        .map_err(|_| (StatusCode::BAD_REQUEST, "Could not upload file".to_string()))?;

        let save_dir = PathBuf::from(format!("{}/{}", upload_dir, result.user_id));
        tokio::fs::create_dir_all(&save_dir).await.unwrap();
        let save_path = save_dir.join(result.id.to_string());

        let mut size: u32 = 0;
        let mut file = tokio::fs::File::create(&save_path).await.unwrap();
        while let Some(chunk) = field.chunk().await.unwrap() {
            size += chunk.len() as u32;
            file.write_all(&chunk).await.unwrap();
        }

        tracing::info!("Uploaded {}!", &name);

        let _ = sqlx::query_as::<_, File>("UPDATE files SET size = $1 WHERE id = $2")
            .bind(size as i32)
            .bind(result.id)
            .fetch_one(&db)
            .await
            .map_err(|_| {
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "Could not upload file".to_string(),
                )
            });
    }

    Ok(StatusCode::CREATED)
}

async fn download_file(
    Path((user_id, file_id)): Path<(i32, i32)>,
    headers: HeaderMap,
    Extension(db): Extension<PgPool>,
) -> Result<Response, (StatusCode, String)> {
    let token = headers
        .get("Authorization")
        .and_then(|v| v.to_str().ok())
        .and_then(|s| s.strip_prefix("Bearer "))
        .ok_or((StatusCode::UNAUTHORIZED, "Missing token".to_string()))?;

    // Verify auth
    let claims = verify_jwt(token)?;
    if claims.sub != user_id {
        return Err((StatusCode::FORBIDDEN, "Forbidden".to_string()));
    }

    // Fetch file metadata
    let file: File = sqlx::query_as("SELECT * FROM files WHERE id = $1 AND user_id = $2")
        .bind(file_id)
        .bind(user_id)
        .fetch_one(&db)
        .await
        .map_err(|_| (StatusCode::NOT_FOUND, "File not found".to_string()))?;

    let upload_dir = std::env::var("PASSIFLORA_DATA_DIR").unwrap_or(DEFAULT_UPLOAD_DIR.to_string());

    let file_path = PathBuf::from(format!("{}/{}/{}", upload_dir, file.user_id, file.id));
    let file_handle = tokio::fs::File::open(&file_path)
        .await
        .map_err(|_| (StatusCode::NOT_FOUND, "File not found".to_string()))?;

    let stream = ReaderStream::new(file_handle);
    let body = Body::from_stream(stream);

    // Build response with headers
    let mut response = Response::new(body);
    let headers = response.headers_mut();
    headers.insert(
        "Content-Disposition",
        format!("attachment; filename=\"{}\"", file.name)
            .parse()
            .unwrap(),
    );
    headers.insert("Content-Type", file.mime_type.parse().unwrap());

    Ok(response)
}
