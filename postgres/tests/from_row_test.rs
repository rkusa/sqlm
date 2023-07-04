use sqlm_postgres::{sql, sql_unchecked, FromRow};

#[tokio::test]
async fn test_from_row() {
    #[derive(Debug, PartialEq, Eq, FromRow)]
    struct User {
        id: i64,
        name: String,
    }

    let id = 1i64;
    let user: User = sql!("SELECT id, name FROM users WHERE id = {id}")
        .await
        .unwrap();
    assert_eq!(
        user,
        User {
            id: 1,
            name: "first".to_string()
        }
    );
}

#[tokio::test]
async fn test_vec_from_row() {
    #[derive(Debug, PartialEq, Eq, FromRow)]
    struct User {
        id: i64,
        name: Option<String>,
    }

    let users: Vec<User> = sql!("SELECT id, name FROM users LIMIT 2").await.unwrap();
    assert_eq!(
        users,
        vec![
            User {
                id: 1,
                name: Some("first".to_string()),
            },
            User { id: 2, name: None }
        ]
    );
}

#[tokio::test]
async fn test_option_from_row() {
    #[derive(Debug, PartialEq, Eq, FromRow)]
    struct User {
        id: i64,
        name: Option<String>,
    }

    let user: Option<User> = sql!("SELECT id, name FROM users WHERE id = 0")
        .await
        .unwrap();
    assert_eq!(user, None);
}

#[tokio::test]
async fn test_from_row_unchecked() {
    #[derive(Debug, PartialEq, Eq, FromRow)]
    struct User {
        id: i64,
        name: String,
    }

    let id = 1i64;
    let user: User = sql_unchecked!("SELECT id, name FROM users WHERE id = {id}")
        .await
        .unwrap();
    assert_eq!(
        user,
        User {
            id: 1,
            name: "first".to_string()
        }
    );
}

#[tokio::test]
async fn test_null_columns() {
    #[derive(Debug, PartialEq, Eq, FromRow)]
    struct UserNotNullName {
        id: i64,
        name: String,
    }

    #[derive(Debug, PartialEq, Eq, FromRow)]
    struct UserNullName {
        id: i64,
        name: Option<String>,
    }

    let user: UserNullName = sql!("SELECT id, name FROM users WHERE id = 2")
        .await
        .unwrap();
    assert_eq!(user, UserNullName { id: 2, name: None });

    let user: UserNotNullName = sql!("SELECT id, name FROM users WHERE id = 2")
        .await
        .unwrap();
    assert_eq!(
        user,
        UserNotNullName {
            id: 2,
            name: "".to_string()
        }
    );
}

#[tokio::test]
async fn test_field_default_function_call() {
    #[derive(Debug, PartialEq, Eq, FromRow)]
    struct User {
        id: i64,
        #[sqlm(default = default_name())]
        name: String,
    }

    fn default_name() -> String {
        "Unnamed".to_string()
    }

    let user: User = sql!("SELECT id, name FROM users WHERE id = 2")
        .await
        .unwrap();
    assert_eq!(
        user,
        User {
            id: 2,
            name: "Unnamed".to_string()
        }
    );
}

#[tokio::test]
async fn test_field_default_literal() {
    #[derive(Debug, PartialEq, Eq, FromRow)]
    struct User {
        id: i64,
        #[sqlm(default = "Unnamed")]
        name: String,
    }

    let user: User = sql!("SELECT id, name FROM users WHERE id = 2")
        .await
        .unwrap();
    assert_eq!(
        user,
        User {
            id: 2,
            name: "Unnamed".to_string()
        }
    );
}
