use sqlm_postgres::sql;

#[tokio::test]
async fn test_option_param() {
    let id: i64 = sql!("SELECT id FROM users WHERE id = {id}", id = Some(1i64))
        .await
        .unwrap();
    assert_eq!(id, 1);
}
