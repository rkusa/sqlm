use sqlm_postgres_macros::sql;

#[tokio::main]
async fn main() {
    let _: time::OffsetDateTime = sql!("SELECT NOW()::DATE").await.unwrap();
}
