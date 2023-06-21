fn main() {
    sqlm_postgres_macros::sql!("foo{} {}", "bar");
}
