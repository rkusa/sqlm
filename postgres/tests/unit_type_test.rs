use sqlm_postgres::sql;

#[tokio::test]
async fn test_execute_explicit_unit_type() {
    let _: () = sql!("UPDATE users SET name = 'updated' WHERE id = -1")
        .await
        .unwrap();
}

#[tokio::test]
async fn test_execute_infered_from_return() -> Result<(), sqlm_postgres::Error> {
    sql!("UPDATE users SET name = 'updated' WHERE id = -1").await?;
    Ok(())
}
