use sqlm_postgres_macros::sql;

#[tokio::main]
async fn main() {
    let _: String = sql!("SELECT COUNT(*) FROM users").await.unwrap();
}
