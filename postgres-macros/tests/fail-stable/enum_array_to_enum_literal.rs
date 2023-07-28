use postgres_types::{FromSql, ToSql};
use sqlm_postgres::{sql, Enum};

#[derive(Debug, Default, FromSql, ToSql, Enum, PartialEq, Eq)]
#[postgres(name = "role")]
enum Role {
    #[default]
    #[postgres(name = "user")]
    User,
    #[postgres(name = "admin")]
    Admin,
}

#[tokio::main]
async fn main() {
    let _: Role = sql!(r#"SELECT ARRAY['admin','user']::role[]"#)
        .await
        .unwrap();
}
