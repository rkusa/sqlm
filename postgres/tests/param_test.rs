use sqlm_postgres::sql;

#[tokio::test]
async fn test_option_param_i64() {
    let id: i64 = sql!("SELECT id FROM users WHERE id = {id}", id = Some(1i64))
        .await
        .unwrap();
    assert_eq!(id, 1);
}

#[tokio::test]
async fn test_option_param_string() {
    let id = Some("first".to_string());
    let id: i64 = sql!("SELECT id FROM users WHERE name = {id}")
        .await
        .unwrap();
    assert_eq!(id, 1);
}

#[tokio::test]
async fn test_param_str() {
    let id: i64 = sql!("SELECT id FROM users WHERE name = {id}", id = "first")
        .await
        .unwrap();
    assert_eq!(id, 1);
}

#[tokio::test]
async fn test_option_param_str() {
    let id: i64 = sql!("SELECT id FROM users WHERE name = {id}", id = Some("first"))
        .await
        .unwrap();
    assert_eq!(id, 1);
}

#[tokio::test]
async fn test_option_param_deref() {
    let name = Some(String::from("first"));
    let id: i64 = sql!(
        "SELECT id FROM users WHERE name = {name}",
        name = name.as_deref()
    )
    .await
    .unwrap();
    assert_eq!(id, 1);
}
