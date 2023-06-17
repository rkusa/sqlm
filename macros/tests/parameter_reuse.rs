use sqlm::Sql;
use sqlm_macros::sql;

#[test]
fn parameter_reuse_named() {
    let sql: Sql<'_, ()> = sql!(
        "SELECT * FROM posts \
         WHERE id > {after} AND id < {after} + 42",
        after = 8,
    );
    assert_eq!(
        sql.query,
        "SELECT * FROM posts WHERE id > $1 AND id < $1 + 42",
    );
    assert_eq!(
        sql.parameters
            .iter()
            .map(ToString::to_string)
            .collect::<Vec<_>>(),
        vec!["8"]
    );
}

#[test]
fn parameter_reuse_positional() {
    let sql: Sql<'_, ()> = sql!(
        "SELECT * FROM posts \
         WHERE id > {} AND id < {0} + 42",
        8,
    );
    assert_eq!(
        sql.query,
        "SELECT * FROM posts WHERE id > $1 AND id < $1 + 42",
    );
    assert_eq!(
        sql.parameters
            .iter()
            .map(ToString::to_string)
            .collect::<Vec<_>>(),
        vec!["8"]
    );
}

#[test]
fn parameter_reuse_variable() {
    let after = 8;
    let sql: Sql<'_, ()> = sql!(
        "SELECT * FROM posts \
         WHERE id > {after} AND id < {after} + 42",
    );
    assert_eq!(
        sql.query,
        "SELECT * FROM posts WHERE id > $1 AND id < $1 + 42",
    );
    assert_eq!(
        sql.parameters
            .iter()
            .map(ToString::to_string)
            .collect::<Vec<_>>(),
        vec!["8"]
    );
}
