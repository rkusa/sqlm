use std::fmt::Debug;

use sqlm_postgres::{sql, Sql};

#[test]
fn parameter_reuse_named() {
    let sql: Sql<'_, _, ()> = sql!(
        "SELECT * FROM users \
         WHERE id > {after} AND id < {after} + 42",
        after = 8,
    );
    assert_eq!(
        sql.query,
        "SELECT * FROM users WHERE id > $1 AND id < $1 + 42",
    );
    assert_eq!(
        sql.parameters.iter().map(fmt_debug).collect::<Vec<_>>(),
        vec!["8"]
    );
}

#[test]
fn parameter_reuse_positional() {
    let sql: Sql<'_, _, ()> = sql!(
        "SELECT * FROM users \
         WHERE id > {} AND id < {0} + 42",
        8,
    );
    assert_eq!(
        sql.query,
        "SELECT * FROM users WHERE id > $1 AND id < $1 + 42",
    );
    assert_eq!(
        sql.parameters.iter().map(fmt_debug).collect::<Vec<_>>(),
        vec!["8"]
    );
}

#[test]
fn parameter_reuse_variable() {
    let after = 8;
    let sql: Sql<'_, _, ()> = sql!(
        "SELECT * FROM users \
         WHERE id > {after} AND id < {after} + 42",
    );
    assert_eq!(
        sql.query,
        "SELECT * FROM users WHERE id > $1 AND id < $1 + 42",
    );
    assert_eq!(
        sql.parameters.iter().map(fmt_debug).collect::<Vec<_>>(),
        vec!["8"]
    );
}

fn fmt_debug(v: impl Debug) -> String {
    format!("{:?}", v)
}
