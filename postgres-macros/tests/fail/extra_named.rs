fn main() {
    sqlm_postgres_macros::sql!("{foo}", foo = 1, bar = 2);
}
