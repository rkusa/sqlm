use sqlm_postgres::{Sql, sql};

#[test]
fn dollar_quoting() {
    let sql: Sql<'_, _, ()> = sql!("SELECT $$a{{b}}$$");
    assert_eq!(sql.query, "SELECT $$a{b}$$");
}

#[test]
fn tagged_dollar_quoting() {
    let sql: Sql<'_, _, ()> = sql!("SELECT $tag$it's$tag$");
    assert_eq!(sql.query, "SELECT $tag$it's$tag$");
}

#[test]
fn dollar_in_identifier() {
    let sql: Sql<'_, _, ()> = sql!("SELECT id AS id$x FROM users");
    assert_eq!(sql.query, "SELECT id AS id$x FROM users");
}

#[test]
fn dollar_in_string_literal() {
    let sql: Sql<'_, _, ()> = sql!("SELECT 'US$ ' || {amount}::TEXT", amount = "5");
    assert_eq!(sql.query, "SELECT 'US$ ' || $1::TEXT");
}
