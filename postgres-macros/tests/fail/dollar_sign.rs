fn main() {
    sqlm_postgres_macros::sql!("SELECT {id} = $1", id = 1);
}
