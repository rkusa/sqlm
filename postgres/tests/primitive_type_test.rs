use sqlm_postgres::sql;

#[tokio::test]
async fn test_query_count() {
    let count: i64 = sql!("SELECT COUNT(*) FROM users").await.unwrap();
    assert_eq!(count, 2);
}

#[tokio::test]
async fn test_query_bool() {
    let exists: bool = sql!("SELECT to_regclass('public.users') IS NOT NULL")
        .await
        .unwrap();
    assert!(exists);
}

#[tokio::test]
async fn test_query_option() {
    let val: Option<i64> = sql!("SELECT 42::BIGINT").await.unwrap();
    assert_eq!(val, Some(42));

    let val: Option<i64> = sql!("SELECT id FROM users WHERE id = -1").await.unwrap();
    assert_eq!(val, None);
}

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
        let val: Option<String> = sql!("SELECT NULL::VARCHAR").await.unwrap();
        assert_eq!(val, None);
    }

    #[tokio::test]
    async fn test_string_vec() {
        let expected = vec!["foo".to_string(), "bar".to_string()];
        let val: Vec<String> = sql!("SELECT {expected}::VARCHAR[]").await.unwrap();
        assert_eq!(val, expected);
    }

    #[tokio::test]
    async fn test_vec_of_string() {
        let expected = vec!["foo".to_string(), "bar".to_string()];
        let val: Vec<String> = sql!("SELECT * FROM unnest({expected}::VARCHAR[])")
            .await
            .unwrap();
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
        let val: Option<i64> = sql!("SELECT NULL::BIGINT").await.unwrap();
        assert_eq!(val, None);
    }

    #[tokio::test]
    async fn test_i64_vec() {
        let expected = vec![4i64, 2i64];
        let val: Vec<i64> = sql!("SELECT {expected}::BIGINT[]").await.unwrap();
        assert_eq!(val, expected);
    }

    #[tokio::test]
    async fn test_vec_of_i64() {
        let expected = vec![4i64, 2i64];
        let val: Vec<i64> = sql!("SELECT * FROM unnest({expected}::BIGINT[])")
            .await
            .unwrap();
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
        let val: Option<i32> = sql!("SELECT NULL::INT").await.unwrap();
        assert_eq!(val, None);
    }

    #[tokio::test]
    async fn test_i32_vec() {
        let expected = vec![4i32, 2i32];
        let val: Vec<i32> = sql!("SELECT {expected}::INT[]").await.unwrap();
        assert_eq!(val, expected);
    }

    #[tokio::test]
    async fn test_vec_of_i32() {
        let expected = vec![4i32, 2i32];
        let val: Vec<i32> = sql!("SELECT * FROM unnest({expected}::INT[])")
            .await
            .unwrap();
        assert_eq!(val, expected);
    }
}

mod f32 {
    use super::*;

    #[tokio::test]
    async fn test_f32() {
        let expected = 42.0f32;
        let val: f32 = sql!("SELECT {expected}::FLOAT4").await.unwrap();
        assert_eq!(val, expected);
    }

    #[tokio::test]
    async fn test_f32_option() {
        let expected = 42.0f32;
        let val: Option<f32> = sql!("SELECT {expected}::FLOAT4").await.unwrap();
        assert_eq!(val, Some(expected));
        let val: Option<f32> = sql!("SELECT NULL::FLOAT4").await.unwrap();
        assert_eq!(val, None);
    }

    #[tokio::test]
    async fn test_f32_vec() {
        let expected = vec![4.0f32, 2.0f32];
        let val: Vec<f32> = sql!("SELECT {expected}::FLOAT4[]").await.unwrap();
        assert_eq!(val, expected);
    }

    #[tokio::test]
    async fn test_vec_of_f32() {
        let expected = vec![4.0f32, 2.0f32];
        let val: Vec<f32> = sql!("SELECT * FROM unnest({expected}::FLOAT4[])")
            .await
            .unwrap();
        assert_eq!(val, expected);
    }
}

mod f64 {
    use super::*;

    #[tokio::test]
    async fn test_f64() {
        let expected = 42.0f64;
        let val: f64 = sql!("SELECT {expected}::FLOAT8").await.unwrap();
        assert_eq!(val, expected);
    }

    #[tokio::test]
    async fn test_f64_option() {
        let expected = 42.0f64;
        let val: Option<f64> = sql!("SELECT {expected}::FLOAT8").await.unwrap();
        assert_eq!(val, Some(expected));
        let val: Option<f64> = sql!("SELECT NULL::FLOAT8").await.unwrap();
        assert_eq!(val, None);
    }

    #[tokio::test]
    async fn test_f64_vec() {
        let expected = vec![4.0f64, 2.0f64];
        let val: Vec<f64> = sql!("SELECT {expected}::FLOAT8[]").await.unwrap();
        assert_eq!(val, expected);
    }

    #[tokio::test]
    async fn test_vec_of_f64() {
        let expected = vec![4.0f64, 2.0f64];
        let val: Vec<f64> = sql!("SELECT * FROM unnest({expected}::FLOAT8[])")
            .await
            .unwrap();
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
        let val: Option<bool> = sql!("SELECT NULL::BOOL").await.unwrap();
        assert_eq!(val, None);
    }

    #[tokio::test]
    async fn test_bool_vec() {
        let expected = vec![true, false];
        let val: Vec<bool> = sql!("SELECT {expected}::BOOL[]").await.unwrap();
        assert_eq!(val, expected);
    }

    #[tokio::test]
    async fn test_vec_of_bool() {
        let expected = vec![true, false];
        let val: Vec<bool> = sql!("SELECT * FROM unnest({expected}::BOOL[])")
            .await
            .unwrap();
        assert_eq!(val, expected);
    }
}

mod byte {
    use super::*;

    #[tokio::test]
    async fn test_byte() {
        let expected = b"test".to_vec();
        let val: Vec<u8> = sql!("SELECT {expected}::BYTEA").await.unwrap();
        assert_eq!(val, expected);
    }

    #[tokio::test]
    async fn test_byte_option() {
        let expected = b"test".to_vec();
        let val: Option<Vec<u8>> = sql!("SELECT {expected}::BYTEA").await.unwrap();
        assert_eq!(val, Some(expected));
        let val: Option<Vec<u8>> = sql!("SELECT NULL::BYTEA").await.unwrap();
        assert_eq!(val, None);
    }

    #[tokio::test]
    async fn test_byte_vec() {
        let expected = vec![b"a".to_vec(), b"b".to_vec()];
        let val: Vec<Vec<u8>> = sql!("SELECT {expected}::BYTEA[]").await.unwrap();
        assert_eq!(val, expected);
    }

    #[tokio::test]
    async fn test_vec_of_byte() {
        let expected = vec![b"a".to_vec(), b"b".to_vec()];
        let val: Vec<Vec<u8>> = sql!("SELECT * FROM unnest({expected}::BYTEA[])")
            .await
            .unwrap();
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
        let val: Option<serde_json::Value> = sql!("SELECT NULL::JSONB").await.unwrap();
        assert_eq!(val, None);
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

    #[tokio::test]
    async fn test_vec_of_json() {
        let expected = vec![
            serde_json::Value::String("foobar".to_string()),
            serde_json::Value::Null,
        ];
        let val: Vec<serde_json::Value> = sql!("SELECT * FROM unnest({expected}::JSONB[])")
            .await
            .unwrap();
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
        let val: Option<::time::OffsetDateTime> =
            sql!("SELECT {expected}::TIMESTAMP WITH TIME ZONE")
                .await
                .unwrap();
        assert_eq!(val, Some(expected));
        let val: Option<::time::OffsetDateTime> =
            sql!("SELECT NULL::TIMESTAMP WITH TIME ZONE").await.unwrap();
        assert_eq!(val, None);
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

    #[tokio::test]
    async fn test_vec_of_datetime() {
        let expected = ::time::OffsetDateTime::now_utc()
            .replace_nanosecond(0)
            .unwrap();
        let expected = vec![expected, expected - ::time::Duration::minutes(5)];
        let val: Vec<::time::OffsetDateTime> =
            sql!("SELECT * FROM unnest({expected}::TIMESTAMP WITH TIME ZONE[])")
                .await
                .unwrap();
        assert_eq!(val, expected);
    }

    #[tokio::test]
    async fn test_date() {
        let expected = ::time::OffsetDateTime::now_utc().date();
        let val: ::time::Date = sql!("SELECT {expected}::DATE").await.unwrap();
        assert_eq!(val, expected);
    }

    #[tokio::test]
    async fn test_date_option() {
        let expected = ::time::OffsetDateTime::now_utc().date();
        let val: Option<::time::Date> = sql!("SELECT {expected}::DATE").await.unwrap();
        assert_eq!(val, Some(expected));
        let val: Option<::time::Date> = sql!("SELECT NULL::DATE").await.unwrap();
        assert_eq!(val, None);
    }

    #[tokio::test]
    async fn test_date_vec() {
        let expected = ::time::OffsetDateTime::now_utc().date();
        let expected = vec![expected, expected - ::time::Duration::minutes(5)];
        let val: Vec<::time::Date> = sql!("SELECT {expected}::DATE[]").await.unwrap();
        assert_eq!(val, expected);
    }

    #[tokio::test]
    async fn test_vec_of_date() {
        let expected = ::time::OffsetDateTime::now_utc().date();
        let expected = vec![expected, expected - ::time::Duration::minutes(5)];
        let val: Vec<::time::Date> = sql!("SELECT * FROM unnest({expected}::DATE[])")
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
        let val: Option<::uuid::Uuid> = sql!("SELECT NULL::UUID").await.unwrap();
        assert_eq!(val, None);
    }

    #[tokio::test]
    async fn test_uuid_vec() {
        let expected = vec![::uuid::Uuid::new_v4(), ::uuid::Uuid::new_v4()];
        let val: Vec<::uuid::Uuid> = sql!("SELECT {expected}::UUID[]").await.unwrap();
        assert_eq!(val, expected);
    }

    #[tokio::test]
    async fn test_vec_of_uuid() {
        let expected = vec![::uuid::Uuid::new_v4(), ::uuid::Uuid::new_v4()];
        let val: Vec<::uuid::Uuid> = sql!("SELECT * FROM unnest({expected}::UUID[])")
            .await
            .unwrap();
        assert_eq!(val, expected);
    }
}
