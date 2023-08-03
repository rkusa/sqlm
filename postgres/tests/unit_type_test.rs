use sqlm_postgres::sql;

#[tokio::test]
async fn test_unit_type() {
    sql!("UPDATE users SET name = 'updated' WHERE id = -1")
        .await
        .unwrap();
}
