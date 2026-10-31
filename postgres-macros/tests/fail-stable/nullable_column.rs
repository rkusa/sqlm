use sqlm_postgres::{FromRow, sql};

#[derive(FromRow)]
struct User {
    id: i64,
    // `users.name` is nullable and must be an `Option`
    name: String,
}

#[tokio::main]
async fn main() {
    let _: User = sql!("SELECT id, name FROM users WHERE id = 1").await.unwrap();
}
