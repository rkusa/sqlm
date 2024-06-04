use sqlm_postgres::sql;

#[tokio::main]
async fn main() {
    let _: String = sql!(r#"SELECT ARRAY['admin','user']::VARCHAR[]"#)
        .await
        .unwrap();
}
