use sqlm_postgres::{FromRow, sql};

// `users.id` is `NOT NULL`, `users.name` is nullable (see `testdb.sql`).

#[tokio::test]
async fn test_not_null_column_into_option() {
    let id: Option<i64> = sql!("SELECT id FROM users WHERE id = 1").await.unwrap();
    assert_eq!(id, Some(1));
}

#[tokio::test]
async fn test_nullable_scalar() {
    let name: Option<String> = sql!("SELECT name FROM users WHERE id = 2").await.unwrap();
    assert_eq!(name, None);
}

#[tokio::test]
async fn test_nullable_scalar_rows() {
    let names: Vec<Option<String>> = sql!("SELECT name FROM users ORDER BY id").await.unwrap();
    assert_eq!(names, vec![Some("first".to_string()), None]);
}

#[tokio::test]
async fn test_nullable_struct_columns() {
    #[derive(Debug, PartialEq, Eq, FromRow)]
    struct User {
        id: Option<i64>,
        name: Option<String>,
    }

    let users: Vec<User> = sql!("SELECT id, name FROM users ORDER BY id")
        .await
        .unwrap();
    assert_eq!(
        users,
        vec![
            User {
                id: Some(1),
                name: Some("first".to_string()),
            },
            User {
                id: Some(2),
                name: None
            },
        ]
    );
}

#[tokio::test]
async fn test_unknown_nullability_falls_back_to_default() {
    #[derive(Debug, PartialEq, Eq, FromRow)]
    struct User {
        id: i64,
        name: String,
    }

    // The nullability of an expression is unknown, so reading it into a non-`Option` is allowed
    // and falls back to the default on `NULL`.
    let user: User = sql!("SELECT id, NULLIF(name, '') AS name FROM users WHERE id = 2")
        .await
        .unwrap();
    assert_eq!(
        user,
        User {
            id: 2,
            name: String::new(),
        }
    );
}

#[tokio::test]
async fn test_outer_join_limitation() {
    #[derive(Debug, PartialEq, Eq, FromRow)]
    struct User {
        id: i64,
        other_id: i64,
    }

    // Known limitation: `other_id` refers to the `NOT NULL` column `users.id`, but is `NULL` due
    // to the outer join. This can't be detected, so the value falls back to the default.
    let user: User = sql!(
        "SELECT u.id, o.id AS other_id FROM users u \
         LEFT JOIN users o ON o.id = u.id + 100 WHERE u.id = 1"
    )
    .await
    .unwrap();
    assert_eq!(user, User { id: 1, other_id: 0 });
}
