use postgres_types::{FromSql, ToSql};
use sqlm_postgres::{sql, Enum, FromRow};

#[derive(Debug, Default, FromSql, ToSql, Enum, PartialEq, Eq)]
#[postgres(name = "role")]
enum Role {
    #[default]
    #[postgres(name = "user")]
    User,
    #[postgres(name = "admin")]
    Admin,
}

#[tokio::test]
async fn test_enum() {
    #[derive(Debug, PartialEq, Eq, FromRow)]
    struct User {
        id: i64,
        role: Role,
    }

    let users: Vec<User> = sql!("SELECT id, role FROM users").await.unwrap();
    assert_eq!(
        users,
        vec![
            User {
                id: 1,
                role: Role::Admin,
            },
            User {
                id: 2,
                role: Role::User
            }
        ]
    );
}
