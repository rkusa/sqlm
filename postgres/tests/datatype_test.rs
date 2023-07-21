use sqlm_postgres_macros::sql;

mod string {
    use super::*;

    #[tokio::test]
    async fn test_string() {
        let expected = "foobar".to_string();
        let val: String = sql!("SELECT {expected}::VARCHAR").await.unwrap();
        assert_eq!(val, expected);
    }

    #[tokio::test]
    async fn test_string_option() {
        let expected = "foobar".to_string();
        let val: Option<String> = sql!("SELECT {expected}::VARCHAR").await.unwrap();
        assert_eq!(val, Some(expected));
    }

    #[tokio::test]
    async fn test_string_vec() {
        let expected = vec!["foo".to_string(), "bar".to_string()];
        let val: Vec<String> = sql!("SELECT {expected}::VARCHAR[]").await.unwrap();
        assert_eq!(val, expected);
    }
}

mod i64 {
    use super::*;

    #[tokio::test]
    async fn test_i64() {
        let expected = 42i64;
        let val: i64 = sql!("SELECT {expected}::BIGINT").await.unwrap();
        assert_eq!(val, expected);
    }

    #[tokio::test]
    async fn test_i64_option() {
        let expected = 42i64;
        let val: Option<i64> = sql!("SELECT {expected}::BIGINT").await.unwrap();
        assert_eq!(val, Some(expected));
    }

    #[tokio::test]
    async fn test_i64_vec() {
        let expected = vec![4i64, 2i64];
        let val: Vec<i64> = sql!("SELECT {expected}::BIGINT[]").await.unwrap();
        assert_eq!(val, expected);
    }
}

mod i32 {
    use super::*;

    #[tokio::test]
    async fn test_i32() {
        let expected = 42i32;
        let val: i32 = sql!("SELECT {expected}::INT").await.unwrap();
        assert_eq!(val, expected);
    }

    #[tokio::test]
    async fn test_i32_option() {
        let expected = 42i32;
        let val: Option<i32> = sql!("SELECT {expected}::INT").await.unwrap();
        assert_eq!(val, Some(expected));
    }

    #[tokio::test]
    async fn test_i32_vec() {
        let expected = vec![4i32, 2i32];
        let val: Vec<i32> = sql!("SELECT {expected}::INT[]").await.unwrap();
        assert_eq!(val, expected);
    }
}

mod bool {
    use super::*;

    #[tokio::test]
    async fn test_bool() {
        let expected = true;
        let val: bool = sql!("SELECT {expected}::BOOL").await.unwrap();
        assert_eq!(val, expected);
    }

    #[tokio::test]
    async fn test_bool_option() {
        let expected = true;
        let val: Option<bool> = sql!("SELECT {expected}::BOOL").await.unwrap();
        assert_eq!(val, Some(expected));
    }

    #[tokio::test]
    async fn test_bool_vec() {
        let expected = vec![true, false];
        let val: Vec<bool> = sql!("SELECT {expected}::BOOL[]").await.unwrap();
        assert_eq!(val, expected);
    }
}

#[cfg(feature = "json")]
mod json {
    use super::*;

    #[tokio::test]
    async fn test_json() {
        let expected = serde_json::Value::String("foobar".to_string());
        let val: serde_json::Value = sql!("SELECT {expected}::JSONB").await.unwrap();
        assert_eq!(val, expected);
    }

    #[tokio::test]
    async fn test_json_option() {
        let expected = serde_json::Value::String("foobar".to_string());
        let val: Option<serde_json::Value> = sql!("SELECT {expected}::JSONB").await.unwrap();
        assert_eq!(val, Some(expected));
    }

    #[tokio::test]
    async fn test_json_vec() {
        let expected = vec![
            serde_json::Value::String("foobar".to_string()),
            serde_json::Value::Null,
        ];
        let val: Vec<serde_json::Value> = sql!("SELECT {expected}::JSONB[]").await.unwrap();
        assert_eq!(val, expected);
    }
}

#[cfg(feature = "time")]
mod time {
    use super::*;

    #[tokio::test]
    async fn test_datetime() {
        let expected = ::time::OffsetDateTime::now_utc()
            .replace_nanosecond(0)
            .unwrap();
        let val: ::time::OffsetDateTime = sql!("SELECT {expected}::TIMESTAMP WITH TIME ZONE")
            .await
            .unwrap();
        assert_eq!(val, expected);
    }

    #[tokio::test]
    async fn test_datetime_option() {
        let expected = ::time::OffsetDateTime::now_utc()
            .replace_nanosecond(0)
            .unwrap();
        let val: ::time::OffsetDateTime = sql!("SELECT {expected}::TIMESTAMP WITH TIME ZONE")
            .await
            .unwrap();
        assert_eq!(val, expected);
    }

    #[tokio::test]
    async fn test_datetime_vec() {
        let expected = ::time::OffsetDateTime::now_utc()
            .replace_nanosecond(0)
            .unwrap();
        let expected = vec![expected, expected - ::time::Duration::minutes(5)];
        let val: Vec<::time::OffsetDateTime> =
            sql!("SELECT {expected}::TIMESTAMP WITH TIME ZONE[]")
                .await
                .unwrap();
        assert_eq!(val, expected);
    }
}

#[cfg(feature = "uuid")]
mod uuid {
    use super::*;

    #[tokio::test]
    async fn test_uuid() {
        let expected = ::uuid::Uuid::new_v4();
        let val: ::uuid::Uuid = sql!("SELECT {expected}::UUID").await.unwrap();
        assert_eq!(val, expected);
    }

    #[tokio::test]
    async fn test_uuid_option() {
        let expected = ::uuid::Uuid::new_v4();
        let val: Option<::uuid::Uuid> = sql!("SELECT {expected}::UUID").await.unwrap();
        assert_eq!(val, Some(expected));
    }

    #[tokio::test]
    async fn test_uuid_vec() {
        let expected = vec![::uuid::Uuid::new_v4(), ::uuid::Uuid::new_v4()];
        let val: Vec<::uuid::Uuid> = sql!("SELECT {expected}::UUID[]").await.unwrap();
        assert_eq!(val, expected);
    }
}
