use futures_util::{Stream, StreamExt, TryStreamExt};

use sqlx::codec::Oid;
use sqlx::{
    ConnectOptions, Connection, DatabaseError, ErrorPosition, Row, Severity,
};
use sqlx::{bytes::Bytes, error::BoxDynError, AssertSqlSafe, SqlSafeStr};
mod common;
use common::{new, setup_if_needed};
use std::env;
use std::pin::{pin, Pin};
use std::sync::Arc;
use std::time::Duration;

#[tokio::test]
async fn it_connects() -> anyhow::Result<()> {
    let mut conn = new().await?;

    let value = sqlx::query("select 1 + 1")
        .try_map(|row: Row| row.try_get::<i32, _>(0))
        .fetch_one(&mut conn)
        .await?;

    assert_eq!(2i32, value);

    Ok(())
}

#[tokio::test]
async fn it_can_select_void() -> anyhow::Result<()> {
    let mut conn = new().await?;

    // pg_notify just happens to be a function that returns void
    let _: () = sqlx::query_scalar("select pg_notify('chan', 'message');")
        .fetch_one(&mut conn)
        .await?;

    Ok(())
}

#[tokio::test]
async fn it_pings() -> anyhow::Result<()> {
    let mut conn = new().await?;

    conn.ping().await?;

    Ok(())
}

#[tokio::test]
async fn it_pings_after_suspended_query() -> anyhow::Result<()> {
    let mut conn = new().await?;

    sqlx::raw_sql("create temporary table processed_row(val int4 primary key)")
        .execute(&mut conn)
        .await?;

    // This query wants to return 50 rows but we only read the first one.
    // This will return a `SuspendedPortal` that the driver currently ignores.
    let _: i32 = sqlx::query_scalar(
        r#"
            insert into processed_row(val)
            select * from generate_series(1, 50)
            returning val
        "#,
    )
    .fetch_one(&mut conn)
    .await?;

    // `Sync` closes the current autocommit transaction which presumably includes closing any
    // suspended portals.
    conn.ping().await?;

    // Make sure that all the values got inserted even though we only read the first one back.
    let count: i64 = sqlx::query_scalar("select count(*) from processed_row")
        .fetch_one(&mut conn)
        .await?;

    assert_eq!(count, 50);

    Ok(())
}

#[tokio::test]
async fn it_maths() -> anyhow::Result<()> {
    let mut conn = new().await?;

    let value = sqlx::query("select 1 + $1::int")
        .bind(5_i32)
        .try_map(|row: Row| row.try_get::<i32, _>(0))
        .fetch_one(&mut conn)
        .await?;

    assert_eq!(6i32, value);

    Ok(())
}

#[tokio::test]
async fn it_can_inspect_errors() -> anyhow::Result<()> {
    let mut conn = new().await?;

    let res: Result<_, sqlx::Error> = sqlx::query("select f").execute(&mut conn).await;
    let err = res.unwrap_err();

    // can also do [as_database_error] or use `match ..`
    let err = err.into_database_error().unwrap();

    assert_eq!(err.message(), "column \"f\" does not exist");
    assert_eq!(err.code(), "42703");


    assert_eq!(err.severity(), Severity::Error);
    assert_eq!(err.message(), "column \"f\" does not exist");
    assert_eq!(err.code(), "42703");
    assert_eq!(err.position(), Some(ErrorPosition::Original(8)));
    assert_eq!(err.routine(), Some("errorMissingColumn"));
    assert_eq!(err.constraint(), None);

    Ok(())
}

#[tokio::test]
async fn it_can_inspect_constraint_errors() -> anyhow::Result<()> {
    let mut conn = new().await?;

    let res: Result<_, sqlx::Error> =
        sqlx::query("INSERT INTO products VALUES (1, 'Product 1', 0);")
            .execute(&mut conn)
            .await;
    let err = res.unwrap_err();

    // can also do [as_database_error] or use `match ..`
    let err = err.into_database_error().unwrap();

    assert_eq!(
        err.message(),
        "new row for relation \"products\" violates check constraint \"products_price_check\""
    );
    assert_eq!(err.code(), "23514");


    assert_eq!(err.severity(), Severity::Error);
    assert_eq!(
        err.message(),
        "new row for relation \"products\" violates check constraint \"products_price_check\""
    );
    assert_eq!(err.code(), "23514");
    assert_eq!(err.position(), None);
    assert_eq!(err.routine(), Some("ExecConstraints"));
    assert_eq!(err.constraint(), Some("products_price_check"));

    Ok(())
}

#[tokio::test]
async fn it_executes() -> anyhow::Result<()> {
    let mut conn = new().await?;

    let _ = conn
        .execute(
            r#"
CREATE TEMPORARY TABLE users (id INTEGER PRIMARY KEY);
            "#,
        )
        .await?;

    for index in 1..=10_i32 {
        let done = sqlx::query("INSERT INTO users (id) VALUES ($1)")
            .bind(index)
            .execute(&mut conn)
            .await?;

        assert_eq!(done.rows_affected(), 1);
    }

    let sum: i32 = sqlx::query("SELECT id FROM users")
        .try_map(|row: Row| row.try_get::<i32, _>(0))
        .fetch(&mut conn)
        .try_fold(0_i32, |acc, x| async move { Ok(acc + x) })
        .await?;

    assert_eq!(sum, 55);

    Ok(())
}

#[tokio::test]
async fn it_can_nest_map() -> anyhow::Result<()> {
    let mut conn = new().await?;

    let res = sqlx::query("SELECT 5")
        .map(|row: Row| row.get(0))
        .map(|int: i32| int.to_string())
        .fetch_one(&mut conn)
        .await?;

    assert_eq!(res, "5");

    Ok(())
}

#[tokio::test]
async fn it_works_with_cache_disabled() -> anyhow::Result<()> {
    setup_if_needed();

    let mut url = url::Url::parse(&env::var("DATABASE_URL")?)?;
    url.query_pairs_mut()
        .append_pair("statement-cache-capacity", "0");

    let mut conn = Connection::connect(url.as_ref()).await?;

    for index in 1..=10_i32 {
        let _ = sqlx::query("SELECT $1")
            .bind(index)
            .execute(&mut conn)
            .await?;
    }

    Ok(())
}

// https://github.com/launchbadge/sqlx/issues/104
#[tokio::test]
async fn it_can_return_interleaved_nulls_issue_104() -> anyhow::Result<()> {
    let mut conn = new().await?;

    let tuple = sqlx::query("SELECT NULL, 10::INT, NULL, 20::INT, NULL, 40::INT, NULL, 80::INT")
        .map(|row: Row| {
            (
                row.get::<Option<i32>, _>(0),
                row.get::<Option<i32>, _>(1),
                row.get::<Option<i32>, _>(2),
                row.get::<Option<i32>, _>(3),
                row.get::<Option<i32>, _>(4),
                row.get::<Option<i32>, _>(5),
                row.get::<Option<i32>, _>(6),
                row.get::<Option<i32>, _>(7),
            )
        })
        .fetch_one(&mut conn)
        .await?;

    assert_eq!(tuple.0, None);
    assert_eq!(tuple.1, Some(10));
    assert_eq!(tuple.2, None);
    assert_eq!(tuple.3, Some(20));
    assert_eq!(tuple.4, None);
    assert_eq!(tuple.5, Some(40));
    assert_eq!(tuple.6, None);
    assert_eq!(tuple.7, Some(80));

    Ok(())
}

#[tokio::test]
async fn it_can_fail_and_recover() -> anyhow::Result<()> {
    let mut conn = new().await?;

    for i in 0..10 {
        // make a query that will fail
        let res = conn
            .execute("INSERT INTO not_found (column) VALUES (10)")
            .await;

        assert!(res.is_err());

        // now try and use the connection
        let val: i32 = conn
            .fetch_one(AssertSqlSafe(format!("SELECT {i}::int4")))
            .await?
            .get(0);

        assert_eq!(val, i);
    }

    Ok(())
}

#[tokio::test]
async fn it_can_query_scalar() -> anyhow::Result<()> {
    let mut conn = new().await?;

    let scalar: i32 = sqlx::query_scalar("SELECT 42").fetch_one(&mut conn).await?;
    assert_eq!(scalar, 42);

    let scalar: Option<i32> = sqlx::query_scalar("SELECT 42").fetch_one(&mut conn).await?;
    assert_eq!(scalar, Some(42));

    let scalar: Option<i32> = sqlx::query_scalar("SELECT NULL")
        .fetch_one(&mut conn)
        .await?;
    assert_eq!(scalar, None);

    let scalar: Option<i64> = sqlx::query_scalar("SELECT 42::bigint")
        .fetch_optional(&mut conn)
        .await?;
    assert_eq!(scalar, Some(42));

    let scalar: Option<i16> = sqlx::query_scalar("").fetch_optional(&mut conn).await?;
    assert_eq!(scalar, None);

    Ok(())
}

#[tokio::test]
/// This is separate from `it_can_query_scalar` because while implementing it I ran into a
/// bug which that prevented `Vec<i32>` from compiling but allowed Vec<Option<i32>>.
async fn it_can_query_all_scalar() -> anyhow::Result<()> {
    let mut conn = new().await?;

    let scalar: Vec<i32> = sqlx::query_scalar("SELECT $1")
        .bind(42)
        .fetch_all(&mut conn)
        .await?;
    assert_eq!(scalar, vec![42]);

    let scalar: Vec<Option<i32>> = sqlx::query_scalar("SELECT $1 UNION ALL SELECT NULL")
        .bind(42)
        .fetch_all(&mut conn)
        .await?;
    assert_eq!(scalar, vec![Some(42), None]);

    Ok(())
}

#[tokio::test]
async fn test_invalid_query() -> anyhow::Result<()> {
    let mut conn = new().await?;

    conn.execute("definitely not a correct query")
        .await
        .unwrap_err();

    let mut s = conn.fetch("select 1");
    let row = s.try_next().await?.unwrap();

    assert_eq!(row.get::<i32, _>(0), 1i32);

    Ok(())
}

/// Tests the edge case of executing a completely empty query string.
///
/// This gets flagged as an `EmptyQueryResponse` in Postgres. We
/// catch this and just return no rows.
#[tokio::test]
async fn test_empty_query() -> anyhow::Result<()> {
    let mut conn = new().await?;
    let done = conn.execute("").await?;

    assert_eq!(done.rows_affected(), 0);

    Ok(())
}

/// Test a simple select expression. This should return the row.
#[tokio::test]
async fn test_select_expression() -> anyhow::Result<()> {
    let mut conn = new().await?;

    let mut s = conn.fetch("SELECT 5");
    let row = s.try_next().await?.unwrap();

    assert!(5i32 == row.try_get::<i32, _>(0)?);

    Ok(())
}

/// Test that we can interleave reads and writes to the database
/// in one simple query. Using the `Cursor` API we should be
/// able to fetch from both queries in sequence.
#[tokio::test]
async fn test_multi_read_write() -> anyhow::Result<()> {
    let mut conn = new().await?;

    let mut s = conn.fetch(
        "
CREATE TABLE IF NOT EXISTS _sqlx_test_postgres_5112 (
    id BIGSERIAL PRIMARY KEY,
    text TEXT NOT NULL
);

SELECT 'Hello World' as _1;

INSERT INTO _sqlx_test_postgres_5112 (text) VALUES ('this is a test');

SELECT id, text FROM _sqlx_test_postgres_5112;
    ",
    );

    let row = s.try_next().await?.unwrap();

    assert!("Hello World" == row.try_get::<&str, _>("_1")?);

    let row = s.try_next().await?.unwrap();

    let id: i64 = row.try_get("id")?;
    let text: &str = row.try_get("text")?;

    assert_eq!(1_i64, id);
    assert_eq!("this is a test", text);

    Ok(())
}

#[tokio::test]
async fn it_caches_statements() -> anyhow::Result<()> {
    let mut conn = new().await?;

    for i in 0..2 {
        let row = sqlx::query("SELECT $1 AS val")
            .bind(Oid(i))
            .persistent(true)
            .fetch_one(&mut conn)
            .await?;

        let val: Oid = row.get("val");

        assert_eq!(Oid(i), val);
    }

    assert_eq!(1, conn.cached_statements_size());
    conn.clear_cached_statements().await?;
    assert_eq!(0, conn.cached_statements_size());

    for i in 0..2 {
        let row = sqlx::query("SELECT $1 AS val")
            .bind(Oid(i))
            .persistent(false)
            .fetch_one(&mut conn)
            .await?;

        let val: Oid = row.get("val");

        assert_eq!(Oid(i), val);
    }

    assert_eq!(0, conn.cached_statements_size());

    Ok(())
}

#[tokio::test]
async fn it_closes_statement_from_cache_issue_470() -> anyhow::Result<()> {
    setup_if_needed();

    let mut options: ConnectOptions = env::var("DATABASE_URL")?.parse().unwrap();

    // a capacity of 1 means that before each statement (after the first)
    // we will close the previous statement
    options = options.statement_cache_capacity(1);

    let mut conn = Connection::connect_with(&options).await?;

    for i in 0..5 {
        let row = sqlx::query(AssertSqlSafe(format!("SELECT {i}::int4 AS val")))
            .fetch_one(&mut conn)
            .await?;

        let val: i32 = row.get("val");

        assert_eq!(i, val);
    }

    assert_eq!(1, conn.cached_statements_size());

    Ok(())
}

#[tokio::test]
async fn it_closes_statements_when_not_persistent_issue_3850() -> anyhow::Result<()> {
    let mut conn = new().await?;

    let _row = sqlx::query("SELECT $1 AS val")
        .bind(Oid(1))
        .persistent(false)
        .fetch_one(&mut conn)
        .await?;

    let row = sqlx::query("SELECT count(*) AS num_prepared_statements FROM pg_prepared_statements")
        .persistent(false)
        .fetch_one(&mut conn)
        .await?;

    let n: i64 = row.get("num_prepared_statements");
    assert_eq!(0, n, "no prepared statements should be open");

    Ok(())
}

#[tokio::test]
async fn it_sets_application_name() -> anyhow::Result<()> {
    setup_if_needed();

    let mut options: ConnectOptions = env::var("DATABASE_URL")?.parse().unwrap();
    options = options.application_name("some-name");

    let mut conn = Connection::connect_with(&options).await?;

    let row = sqlx::query("select current_setting('application_name') as app_name")
        .fetch_one(&mut conn)
        .await?;

    let val: String = row.get("app_name");

    assert_eq!("some-name", &val);

    Ok(())
}

#[tokio::test]
async fn it_can_handle_parameter_status_message_issue_484() -> anyhow::Result<()> {
    new().await?.execute("SET NAMES 'UTF8'").await?;
    Ok(())
}

#[tokio::test]
async fn it_can_prepare_then_execute() -> anyhow::Result<()> {
    let mut conn = new().await?;

    let tweet_id: i64 =
        sqlx::query_scalar("INSERT INTO tweet ( text ) VALUES ( 'Hello, World' ) RETURNING id")
            .fetch_one(&mut conn)
            .await?;

    let statement = conn
        .prepare("SELECT * FROM tweet WHERE id = $1".into_sql_str())
        .await?;

    assert_eq!(statement.column(0).name(), "id");
    assert_eq!(statement.column(1).name(), "created_at");
    assert_eq!(statement.column(2).name(), "text");
    assert_eq!(statement.column(3).name(), "owner_id");

    assert_eq!(statement.column(0).type_info().name(), "INT8");
    assert_eq!(statement.column(1).type_info().name(), "TIMESTAMPTZ");
    assert_eq!(statement.column(2).type_info().name(), "TEXT");
    assert_eq!(statement.column(3).type_info().name(), "INT8");

    let row = statement
        .query()
        .bind(tweet_id)
        .fetch_one(&mut conn)
        .await?;
    let tweet_text: &str = row.try_get("text")?;

    assert_eq!(tweet_text, "Hello, World");

    Ok(())
}

#[tokio::test]
async fn it_supports_domain_types_in_composite_domain_types() -> anyhow::Result<()> {
    // Only supported in Postgres 11+
    let mut conn = new().await?;
    if matches!(conn.server_version_num(), Some(version) if version < 110000) {
        return Ok(());
    }

    conn.execute(
        r#"
DROP TABLE IF EXISTS heating_bills;
DROP DOMAIN IF EXISTS winter_year_month;
DROP TYPE IF EXISTS year_month;
DROP DOMAIN IF EXISTS month_id;

CREATE DOMAIN month_id AS INT2 CHECK (1 <= value AND value <= 12);
CREATE TYPE year_month AS (year INT4, month month_id);
CREATE DOMAIN winter_year_month AS year_month CHECK ((value).month <= 3);
CREATE TABLE heating_bills (
  month winter_year_month NOT NULL PRIMARY KEY,
  cost INT4 NOT NULL
);
    "#,
    )
    .await?;

    #[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
    struct MonthId(i16);

    impl sqlx::Type for MonthId {
        fn type_info() -> sqlx::TypeInfo {
            sqlx::TypeInfo::with_name("month_id")
        }

        fn compatible(ty: &sqlx::TypeInfo) -> bool {
            *ty == Self::type_info()
        }
    }

    impl<'r> sqlx::Decode<'r> for MonthId {
        fn decode(
            value: sqlx::ValueRef<'r>,
        ) -> Result<Self, Box<dyn std::error::Error + 'static + Send + Sync>> {
            Ok(Self(<i16 as sqlx::Decode>::decode(value)?))
        }
    }

    impl<'q> sqlx::Encode<'q> for MonthId {
        fn encode_by_ref(
            &self,
            buf: &mut sqlx::ArgumentBuffer,
        ) -> Result<sqlx::encode::IsNull, BoxDynError> {
            <i16 as sqlx::Encode>::encode(self.0, buf)
        }
    }

    #[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
    struct WinterYearMonth {
        year: i32,
        month: MonthId,
    }

    impl sqlx::Type for WinterYearMonth {
        fn type_info() -> sqlx::TypeInfo {
            sqlx::TypeInfo::with_name("winter_year_month")
        }

        fn compatible(ty: &sqlx::TypeInfo) -> bool {
            *ty == Self::type_info()
        }
    }

    impl<'r> sqlx::Decode<'r> for WinterYearMonth {
        fn decode(
            value: sqlx::ValueRef<'r>,
        ) -> Result<Self, Box<dyn std::error::Error + 'static + Send + Sync>> {
            let mut decoder = sqlx::codec::RecordDecoder::new(value)?;

            let year = decoder.try_decode::<i32>()?;
            let month = decoder.try_decode::<MonthId>()?;

            Ok(Self { year, month })
        }
    }

    impl<'q> sqlx::Encode<'q> for WinterYearMonth {
        fn encode_by_ref(
            &self,
            buf: &mut sqlx::ArgumentBuffer,
        ) -> Result<sqlx::encode::IsNull, sqlx::error::BoxDynError> {
            let mut encoder = sqlx::codec::RecordEncoder::new(buf);
            encoder.encode(self.year)?;
            encoder.encode(self.month)?;
            encoder.finish();
            Ok(sqlx::encode::IsNull::No)
        }
    }
    let mut conn = new().await?;

    let result = sqlx::query("DELETE FROM heating_bills;")
        .execute(&mut conn)
        .await;

    let result = result.unwrap();
    assert_eq!(result.rows_affected(), 0);

    let result =
        sqlx::query("INSERT INTO heating_bills(month, cost) VALUES($1::winter_year_month, 100);")
            .bind(WinterYearMonth {
                year: 2021,
                month: MonthId(1),
            })
            .execute(&mut conn)
            .await;

    let result = result.unwrap();
    assert_eq!(result.rows_affected(), 1);

    let result = sqlx::query("DELETE FROM heating_bills;")
        .execute(&mut conn)
        .await;

    let result = result.unwrap();
    assert_eq!(result.rows_affected(), 1);

    Ok(())
}

#[tokio::test]
async fn it_resolves_custom_type_in_array() -> anyhow::Result<()> {
    // Only supported in Postgres 11+
    let mut conn = new().await?;
    if matches!(conn.server_version_num(), Some(version) if version < 110000) {
        return Ok(());
    }

    // language=PostgreSQL
    conn.execute(
        r#"
DROP TABLE IF EXISTS pets;
DROP TYPE IF EXISTS pet_name_and_race;

CREATE TYPE pet_name_and_race AS (
  name TEXT,
  race TEXT
);
CREATE TABLE pets (
  owner TEXT NOT NULL,
  name TEXT NOT NULL,
  race TEXT NOT NULL,
  PRIMARY KEY (owner, name)
);
INSERT INTO pets(owner, name, race)
VALUES
  ('Alice', 'Foo', 'cat');
INSERT INTO pets(owner, name, race)
VALUES
  ('Alice', 'Bar', 'dog');
    "#,
    )
    .await?;

    #[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
    struct PetNameAndRace {
        name: String,
        race: String,
    }

    impl sqlx::Type for PetNameAndRace {
        fn type_info() -> sqlx::TypeInfo {
            sqlx::TypeInfo::with_name("pet_name_and_race")
        }
    }

    impl<'r> sqlx::Decode<'r> for PetNameAndRace {
        fn decode(
            value: sqlx::ValueRef<'r>,
        ) -> Result<Self, Box<dyn std::error::Error + 'static + Send + Sync>> {
            let mut decoder = sqlx::codec::RecordDecoder::new(value)?;
            let name = decoder.try_decode::<String>()?;
            let race = decoder.try_decode::<String>()?;
            Ok(Self { name, race })
        }
    }

    #[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
    struct PetNameAndRaceArray(Vec<PetNameAndRace>);

    impl sqlx::Type for PetNameAndRaceArray {
        fn type_info() -> sqlx::TypeInfo {
            // Array type name is the name of the element type prefixed with `_`
            sqlx::TypeInfo::with_name("_pet_name_and_race")
        }
    }

    impl<'r> sqlx::Decode<'r> for PetNameAndRaceArray {
        fn decode(
            value: sqlx::ValueRef<'r>,
        ) -> Result<Self, Box<dyn std::error::Error + 'static + Send + Sync>> {
            Ok(Self(Vec::<PetNameAndRace>::decode(value)?))
        }
    }

    let mut conn = new().await?;

    let row = sqlx::query("select owner, array_agg(row(name, race)::pet_name_and_race) as pets from pets group by owner")
        .fetch_one(&mut conn)
        .await?;

    let pets: PetNameAndRaceArray = row.get("pets");

    assert_eq!(pets.0.len(), 2);
    Ok(())
}


#[tokio::test]
async fn custom_type_resolution_respects_search_path() -> anyhow::Result<()> {
    let mut conn = new().await?;

    conn.execute(
        r#"
DROP TYPE IF EXISTS some_enum_type;
DROP SCHEMA IF EXISTS another CASCADE;

CREATE SCHEMA another;
CREATE TYPE some_enum_type AS ENUM ('a', 'b', 'c');
CREATE TYPE another.some_enum_type AS ENUM ('d', 'e', 'f');
    "#,
    )
    .await?;

    #[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
    struct SomeEnumType(String);

    impl sqlx::Type for SomeEnumType {
        fn type_info() -> sqlx::TypeInfo {
            sqlx::TypeInfo::with_name("some_enum_type")
        }

        fn compatible(ty: &sqlx::TypeInfo) -> bool {
            *ty == Self::type_info()
        }
    }

    impl<'r> sqlx::Decode<'r> for SomeEnumType {
        fn decode(
            value: sqlx::ValueRef<'r>,
        ) -> Result<Self, Box<dyn std::error::Error + 'static + Send + Sync>> {
            Ok(Self(<String as sqlx::Decode>::decode(value)?))
        }
    }

    impl<'q> sqlx::Encode<'q> for SomeEnumType {
        fn encode_by_ref(
            &self,
            buf: &mut sqlx::ArgumentBuffer,
        ) -> Result<sqlx::encode::IsNull, BoxDynError> {
            <String as sqlx::Encode>::encode_by_ref(&self.0, buf)
        }
    }

    let mut conn = new().await?;

    sqlx::query("set search_path = 'another'")
        .execute(&mut conn)
        .await?;

    let result = sqlx::query("SELECT 1 WHERE $1::some_enum_type = 'd'::some_enum_type;")
        .bind(SomeEnumType("d".into()))
        .fetch_all(&mut conn)
        .await;

    let result = result.unwrap();
    assert_eq!(result.len(), 1);

    Ok(())
}

#[tokio::test]
async fn test_pg_server_num() -> anyhow::Result<()> {
    let conn = new().await?;

    assert!(conn.server_version_num().is_some());

    Ok(())
}

#[tokio::test]
async fn it_encodes_custom_array_issue_1504() -> anyhow::Result<()> {
    use sqlx::encode::IsNull;
    use sqlx::{ArgumentBuffer, TypeInfo};
    use sqlx::{Decode, Encode, Type};

    #[derive(Debug, PartialEq)]
    enum Value {
        String(String),
        Number(i32),
        Array(Vec<Value>),
    }

    impl<'r> Decode<'r> for Value {
        fn decode(
            value: sqlx::ValueRef<'r>,
        ) -> std::result::Result<Self, Box<dyn std::error::Error + 'static + Send + Sync>> {
            let typ = value.type_info().into_owned();

            if typ == TypeInfo::with_name("text") {
                let s = <String as Decode<'_>>::decode(value)?;

                Ok(Self::String(s))
            } else if typ == TypeInfo::with_name("int4") {
                let n = <i32 as Decode<'_>>::decode(value)?;

                Ok(Self::Number(n))
            } else if typ == TypeInfo::with_name("_text") {
                let arr = Vec::<String>::decode(value)?;
                let v = arr.into_iter().map(|s| Value::String(s)).collect();

                Ok(Self::Array(v))
            } else if typ == TypeInfo::with_name("_int4") {
                let arr = Vec::<i32>::decode(value)?;
                let v = arr.into_iter().map(|n| Value::Number(n)).collect();

                Ok(Self::Array(v))
            } else {
                Err("unknown type".into())
            }
        }
    }

    impl Encode<'_> for Value {
        fn produces(&self) -> Option<TypeInfo> {
            match self {
                Self::Array(a) => {
                    if a.len() < 1 {
                        return Some(TypeInfo::with_name("_text"));
                    }

                    match a[0] {
                        Self::String(_) => Some(TypeInfo::with_name("_text")),
                        Self::Number(_) => Some(TypeInfo::with_name("_int4")),
                        Self::Array(_) => None,
                    }
                }
                Self::String(_) => Some(TypeInfo::with_name("text")),
                Self::Number(_) => Some(TypeInfo::with_name("int4")),
            }
        }

        fn encode_by_ref(&self, buf: &mut ArgumentBuffer) -> Result<IsNull, BoxDynError> {
            match self {
                Value::String(s) => <String as Encode<'_>>::encode_by_ref(s, buf),
                Value::Number(n) => <i32 as Encode<'_>>::encode_by_ref(n, buf),
                Value::Array(arr) => arr.encode(buf),
            }
        }
    }

    impl Type for Value {
        fn type_info() -> TypeInfo {
            TypeInfo::with_name("unknown")
        }

        fn compatible(ty: &TypeInfo) -> bool {
            [
                TypeInfo::with_name("text"),
                TypeInfo::with_name("_text"),
                TypeInfo::with_name("int4"),
                TypeInfo::with_name("_int4"),
            ]
            .contains(ty)
        }
    }

    let mut conn = new().await?;

    let (row,): (Value,) = sqlx::query_as("SELECT $1::text[] as Dummy")
        .bind(Value::Array(vec![
            Value::String("Test 0".to_string()),
            Value::String("Test 1".to_string()),
        ]))
        .fetch_one(&mut conn)
        .await?;

    assert_eq!(
        row,
        Value::Array(vec![
            Value::String("Test 0".to_string()),
            Value::String("Test 1".to_string()),
        ])
    );

    let (row,): (Value,) = sqlx::query_as("SELECT $1::int4[] as Dummy")
        .bind(Value::Array(vec![
            Value::Number(3),
            Value::Number(2),
            Value::Number(1),
        ]))
        .fetch_one(&mut conn)
        .await?;

    assert_eq!(
        row,
        Value::Array(vec![Value::Number(3), Value::Number(2), Value::Number(1)])
    );

    Ok(())
}


#[tokio::test]
async fn test_postgres_bytea_hex_deserialization_errors() -> anyhow::Result<()> {
    let mut conn = new().await?;
    conn.execute("SET bytea_output = 'escape';").await?;
    for value in ["", "DEADBEEF"] {
        let query = format!("SELECT '\\x{value}'::bytea");
        let res: sqlx::Result<Vec<u8>> =
            conn.fetch_one(AssertSqlSafe(query)).await?.try_get(0usize);
        // Deserialization only supports hex format so this should error and definitely not panic.
        res.unwrap_err();
    }
    Ok(())
}

#[tokio::test]
async fn test_shrink_buffers() -> anyhow::Result<()> {
    // We don't really have a good way to test that `.shrink_buffers()` functions as expected
    // without exposing a lot of internals, but we can at least be sure it doesn't
    // materially affect the operation of the connection.

    let mut conn = new().await?;

    // The connection buffer is only 8 KiB by default so this should definitely force it to grow.
    let data = vec![0u8; 32 * 1024];

    let ret: Vec<u8> = sqlx::query_scalar("SELECT $1::bytea")
        .bind(&data)
        .fetch_one(&mut conn)
        .await?;

    assert_eq!(ret, data);

    conn.shrink_buffers();

    let ret: i64 = sqlx::query_scalar("SELECT $1::int8")
        .bind(&12345678i64)
        .fetch_one(&mut conn)
        .await?;

    assert_eq!(ret, 12345678i64);

    Ok(())
}

#[tokio::test]
async fn test_error_handling_with_deferred_constraints() -> anyhow::Result<()> {
    let mut conn = new().await?;

    sqlx::query("CREATE TABLE IF NOT EXISTS deferred_constraint ( id INTEGER PRIMARY KEY )")
        .execute(&mut conn)
        .await?;

    sqlx::query("CREATE TABLE IF NOT EXISTS deferred_constraint_fk ( fk INTEGER CONSTRAINT deferred_fk REFERENCES deferred_constraint(id) DEFERRABLE INITIALLY DEFERRED )")
            .execute(&mut conn)
            .await?;

    let result: sqlx::Result<i32> =
        sqlx::query_scalar("INSERT INTO deferred_constraint_fk VALUES (1) RETURNING fk")
            .fetch_one(&mut conn)
            .await;

    let err = result.unwrap_err();
    let db_err = err.as_database_error().unwrap();
    assert_eq!(db_err.constraint(), Some("deferred_fk"));

    Ok(())
}

#[tokio::test]
#[cfg(feature = "bigdecimal")]
async fn test_issue_3052() {
    use sqlx::codec::BigDecimal;

    // https://github.com/launchbadge/sqlx/issues/3052
    // Previously, attempting to bind a `BigDecimal` would panic if the value was out of range.
    // Now, we rewrite it to a sentinel value so that Postgres will return a range error.
    let too_small: BigDecimal = "1E-65536".parse().unwrap();
    let too_large: BigDecimal = "1E262144".parse().unwrap();

    let mut conn = new().await.unwrap();

    let too_small_error = sqlx::query_scalar::<_, BigDecimal>("SELECT $1::numeric")
        .bind(&too_small)
        .fetch_one(&mut conn)
        .await
        .expect_err("Too small number should have failed");
    assert!(
        matches!(&too_small_error, sqlx::Error::Encode(_)),
        "expected encode error, got {too_small_error:?}"
    );

    let too_large_error = sqlx::query_scalar::<_, BigDecimal>("SELECT $1::numeric")
        .bind(&too_large)
        .fetch_one(&mut conn)
        .await
        .expect_err("Too large number should have failed");

    assert!(
        matches!(&too_large_error, sqlx::Error::Encode(_)),
        "expected encode error, got {too_large_error:?}",
    );
}


