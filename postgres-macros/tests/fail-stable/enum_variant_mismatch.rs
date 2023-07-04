use postgres_types::{FromSql, ToSql};
use sqlm_postgres_macros::{sql, Enum, FromRow};

#[derive(Debug, Default, FromSql, ToSql, Enum, PartialEq, Eq)]
#[postgres(name = "role")]
enum Role {
    #[default]
    #[postgres(name = "user")]
    User,

    #[postgres(name = "moderator")]
    Moderator,
}

#[derive(Debug, PartialEq, Eq, FromRow)]
struct User {
    id: i64,
    role: Role,
}

#[tokio::main]
async fn main() {
    let _: Vec<User> = sql!("SELECT id, role FROM users").await.unwrap();
}
