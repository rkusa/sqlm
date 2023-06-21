use std::fmt::Debug;

use sqlm_postgres::{sql, Sql};

#[test]
fn parameter_order() {
    let sql: Sql<'_, _, ()> = sql!(
        "SELECT * FROM users WHERE id > {after} AND id < {before}",
        before = 42,
        after = 0,
    );
    assert_eq!(sql.query, "SELECT * FROM users WHERE id > $1 AND id < $2",);
    assert_eq!(
        sql.parameters.iter().map(fmt_debug).collect::<Vec<_>>(),
        vec!["0", "42"]
    );
}

fn fmt_debug(v: impl Debug) -> String {
    format!("{:?}", v)
}
