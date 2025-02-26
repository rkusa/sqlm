use sqlm_postgres::{Sql, sql};

#[test]
fn curly_escape() {
    let sql: Sql<'_, _, ()> = sql!("SELECT '{{1,2,3}}'");
    assert_eq!(sql.query, "SELECT '{1,2,3}'");
}
