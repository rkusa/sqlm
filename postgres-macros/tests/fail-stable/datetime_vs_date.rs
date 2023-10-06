use sqlm_postgres_macros::sql;

#[tokio::main]
async fn main() {
    let _: time::Date = sql!("SELECT NOW()::TIMESTAMP WITH TIME ZONE")
        .await
        .unwrap();
}
