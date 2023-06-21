use sqlm::Sql;
use sqlm_macros::sql;

#[test]
fn parameter_order() {
    let sql: Sql<'_, ()> = sql!(
        "SELECT * FROM posts WHERE id > {after} AND id < {before}",
        before = 42,
        after = 0,
    );
    assert_eq!(sql.query, "SELECT * FROM posts WHERE id > $1 AND id < $2",);
    assert_eq!(
        sql.parameters
            .iter()
            .map(ToString::to_string)
            .collect::<Vec<_>>(),
        vec!["0", "42"]
    );
}
