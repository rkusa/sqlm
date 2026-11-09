fn main() {
    let id = 1i32;
    let _: sqlm_postgres::Sql<'_, _, i64> = sqlm_postgres::sql!("SELECT id FROM users WHERE id = {id}");
}
