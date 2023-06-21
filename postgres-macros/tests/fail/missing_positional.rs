fn main() {
    sqlm_postgres_macros::sql!("foo{1} {}", "bar");
}
