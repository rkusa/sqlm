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

#[tokio::test]
async fn test_scalar_optional() {
    let val: Option<i64> = sql!("SELECT 42::BIGINT").await.unwrap();
    assert_eq!(val, Some(42));

    let val: Option<i64> = sql!("SELECT id FROM users WHERE id = -1").await.unwrap();
    assert_eq!(val, None);
}
