use postgres_types::{FromSql, ToSql};
use sqlm_postgres::{FromRow, SqlType, sql};

#[derive(Debug, Default, PartialEq, Eq)]
struct Id(i64);

#[derive(Debug, PartialEq, Eq, FromRow)]
struct User {
    id: Id,
    name: String,
}

impl SqlType for Id {
    type Type = i64;
}

impl<'a> FromSql<'a> for Id {
    fn from_sql(
        ty: &postgres_types::Type,
        raw: &'a [u8],
    ) -> Result<Self, Box<dyn std::error::Error + Sync + Send>> {
        Ok(Id(i64::from_sql(ty, raw)?))
    }

    fn accepts(ty: &postgres_types::Type) -> bool {
        <i64 as FromSql<'_>>::accepts(ty)
    }
}

impl ToSql for Id {
    fn to_sql(
        &self,
        ty: &postgres_types::Type,
        out: &mut postgres_types::private::BytesMut,
    ) -> Result<postgres_types::IsNull, Box<dyn std::error::Error + Sync + Send>>
    where
        Self: Sized,
    {
        i64::to_sql(&self.0, ty, out)
    }

    fn accepts(ty: &postgres_types::Type) -> bool
    where
        Self: Sized,
    {
        <i64 as FromSql<'_>>::accepts(ty)
    }

    fn to_sql_checked(
        &self,
        ty: &postgres_types::Type,
        out: &mut postgres_types::private::BytesMut,
    ) -> Result<postgres_types::IsNull, Box<dyn std::error::Error + Sync + Send>> {
        i64::to_sql_checked(&self.0, ty, out)
    }
}

#[tokio::test]
async fn test() {
    let id: Id = sql!("SELECT id FROM users WHERE id = 1").await.unwrap();
    assert_eq!(id, Id(1));
}

#[tokio::test]
async fn test_option() {
    let role: Option<Id> = sql!("SELECT id FROM users WHERE id = 1").await.unwrap();
    assert_eq!(role, Some(Id(1)));
    let role: Option<Id> = sql!("SELECT id FROM users WHERE id = -1").await.unwrap();
    assert_eq!(role, None);
}

#[tokio::test]
async fn test_vec() {
    let role: Vec<Id> = sql!(r#"SELECT ARRAY[4, 2]::BIGINT[]"#).await.unwrap();
    assert_eq!(role, vec![Id(4), Id(2)]);
}

#[tokio::test]
async fn test_property() {
    let users: User = sql!("SELECT id, name FROM users WHERE id = 1")
        .await
        .unwrap();
    assert_eq!(
        users,
        User {
            id: Id(1),
            name: "first".to_string()
        }
    );
}

#[tokio::test]
async fn test_option_property() {
    #[derive(Debug, PartialEq, Eq, FromRow)]
    struct User {
        id: Option<Id>,
        name: String,
    }
    let user: User = sql!("SELECT id, name FROM users WHERE id = 1")
        .await
        .unwrap();
    assert_eq!(
        user,
        User {
            id: Some(Id(1)),
            name: "first".to_string()
        }
    );
}

#[tokio::test]
async fn test_property_option() {
    let users: Option<User> = sql!("SELECT id, name FROM users WHERE id = 1")
        .await
        .unwrap();
    assert_eq!(
        users,
        Some(User {
            id: Id(1),
            name: "first".to_string()
        })
    );
}

#[tokio::test]
async fn test_vec_property() {
    #[derive(Debug, PartialEq, Eq, FromRow)]
    struct User {
        ids: Vec<Id>,
        name: String,
    }
    let user: User = sql!("SELECT ARRAY[4, 2]::BIGINT[] AS ids, '' AS name")
        .await
        .unwrap();
    assert_eq!(
        user,
        User {
            ids: vec![Id(4), Id(2)],
            name: String::new()
        }
    );
}

#[tokio::test]
async fn test_property_vec() {
    let users: Vec<User> = sql!("SELECT id, name FROM users").await.unwrap();
    assert_eq!(
        users,
        vec![
            User {
                id: Id(1),
                name: "first".to_string()
            },
            User {
                id: Id(2),
                name: String::new()
            }
        ]
    );
}

#[tokio::test]
async fn test_param() {
    let id = Id(1);
    let users: Vec<User> = sql!("SELECT id, name FROM users WHERE id = {id}")
        .await
        .unwrap();
    assert_eq!(
        users,
        vec![User {
            id: Id(1),
            name: "first".to_string()
        }]
    );
}

#[tokio::test]
async fn test_vec_param() {
    let ids = vec![Id(1), Id(2)];
    let users: Vec<User> = sql!("SELECT id, name FROM users WHERE id = ANY({ids})")
        .await
        .unwrap();
    assert_eq!(
        users,
        vec![
            User {
                id: Id(1),
                name: "first".to_string()
            },
            User {
                id: Id(2),
                name: String::new()
            }
        ]
    );
}
