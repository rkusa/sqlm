use sqlm_postgres::sql;

#[tokio::main]
async fn main() {
    // `users.name` is nullable and must be an `Option`
    let _: String = sql!("SELECT name FROM users WHERE id = 1").await.unwrap();
}
