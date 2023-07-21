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

#[derive(Debug, PartialEq, Eq, FromRow)]
struct User {
    id: i64,
    role: Role,
}

#[tokio::test]
async fn test_enum() {
    let role: Role = sql!("SELECT role FROM users WHERE id = 1").await.unwrap();
    assert_eq!(role, Role::Admin);
}

#[tokio::test]
async fn test_enum_option() {
    let role: Option<Role> = sql!("SELECT role FROM users WHERE id = 1").await.unwrap();
    assert_eq!(role, Some(Role::Admin));
    let role: Option<Role> = sql!("SELECT role FROM users WHERE id = -1").await.unwrap();
    assert_eq!(role, None);
}

#[tokio::test]
async fn test_enum_vec() {
    let role: Vec<Role> = sql!(r#"SELECT ARRAY['admin','user']::role[]"#)
        .await
        .unwrap();
    assert_eq!(role, vec![Role::Admin, Role::User]);
}

#[tokio::test]
async fn test_enum_property() {
    let users: User = sql!("SELECT id, role FROM users WHERE id = 1")
        .await
        .unwrap();
    assert_eq!(
        users,
        User {
            id: 1,
            role: Role::Admin,
        }
    );
}

#[tokio::test]
async fn test_enum_option_property() {
    #[derive(Debug, PartialEq, Eq, FromRow)]
    struct User {
        id: i64,
        role: Option<Role>,
    }
    let user: User = sql!("SELECT id, role FROM users WHERE id = 1")
        .await
        .unwrap();
    assert_eq!(
        user,
        User {
            id: 1,
            role: Some(Role::Admin),
        }
    );
}

#[tokio::test]
async fn test_enum_property_option() {
    let users: Option<User> = sql!("SELECT id, role FROM users WHERE id = 1")
        .await
        .unwrap();
    assert_eq!(
        users,
        Some(User {
            id: 1,
            role: Role::Admin,
        })
    );
}

#[tokio::test]
async fn test_enum_vec_property() {
    #[derive(Debug, PartialEq, Eq, FromRow)]
    struct User {
        id: i64,
        roles: Vec<Role>,
    }
    let user: User = sql!("SELECT 1::BIGINT AS id, ARRAY['admin', 'user']::role[] AS roles")
        .await
        .unwrap();
    assert_eq!(
        user,
        User {
            id: 1,
            roles: vec![Role::Admin, Role::User],
        }
    );
}

#[tokio::test]
async fn test_enum_property_vec() {
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

// TODO:
// #[tokio::test]
// async fn test_enum_param() {
//     let role = Role::Admin;
//     let users: Vec<User> = sql!("SELECT id, role FROM users WHERE role = {role}")
//         .await
//         .unwrap();
//     assert_eq!(
//         users,
//         vec![User {
//             id: 1,
//             role: Role::Admin,
//         }]
//     );
// }
