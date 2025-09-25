use sqlx::PgPool;

pub async fn init_db() -> PgPool {
    let database_url = "postgres://passiflora:secret@localhost:5432/passiflora";
    PgPool::connect(database_url)
        .await
        .expect("Failed to connect to database")
}
