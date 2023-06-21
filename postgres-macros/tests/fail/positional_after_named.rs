fn main() {
    sqlm_postgres_macros::sql!("{one} {}", one = 1, 2);
}
