use sqlm_postgres::sql;

#[tokio::test]
async fn test_scalar_count() {
    let count: i64 = sql!("SELECT COUNT(*) FROM users").await.unwrap();
    assert_eq!(count, 2);
}

#[tokio::test]
async fn test_scalar_bool() {
    let exists: bool = sql!("SELECT to_regclass('public.users') IS NOT NULL")
        .await
        .unwrap();
    assert!(exists);
}
