use sqlx::Error;
use std::env;

pub fn setup_if_needed() {
    let _ = env_logger::builder().is_test(true).try_init();
}

// Make a new connection. `DATABASE_URL` must be set in the process environment.
//
// Wipes per-test mutable schema so tests are order-independent.
pub async fn new() -> sqlx::Result<sqlx::Connection> {
    setup_if_needed();

    let db_url = env::var("DATABASE_URL").map_err(|e| Error::Configuration(Box::new(e)))?;

    let mut conn = sqlx::Connection::connect(&db_url).await?;
    conn.execute("TRUNCATE tweet, tweet_reply, products, circles RESTART IDENTITY CASCADE")
        .await?;
    Ok(conn)
}

// Test type encoding and decoding
#[macro_export]
macro_rules! test_type {
    ($name:ident<$ty:ty>($db:ident, $sql:literal, $($text:literal == $value:expr),+ $(,)?)) => {
        $crate::__test_prepared_type!($name<$ty>($db, $sql, $($text == $value),+));
        $crate::test_unprepared_type!($name<$ty>($db, $($text == $value),+));
    };

    ($name:ident<$ty:ty>($db:ident, $($text:literal == $value:expr),+ $(,)?)) => {
        paste::item! {
            $crate::__test_prepared_type!($name<$ty>($db, $crate::[< $db _query_for_test_prepared_type >]!(), $($text == $value),+));
            $crate::test_unprepared_type!($name<$ty>($db, $($text == $value),+));
        }
    };

    ($name:ident<$ty:ty>($db:ident, $($text:literal ~= $value:expr),+ $(,)?)) => {
        paste::item! {
            $crate::__test_prepared_type!($name<$ty>($db, $crate::[< $db _query_for_test_prepared_geometric_type >]!(), $($text == $value),+));
        }
    };
    ($name:ident<$ty:ty>($db:ident, $($text:literal @= $value:expr),+ $(,)?)) => {
        paste::item! {
            $crate::__test_prepared_type!($name<$ty>($db, $crate::[< $db _query_for_test_prepared_geometric_array_type >]!(), $($text == $value),+));
        }
    };


    ($name:ident($db:ident, $($text:literal == $value:expr),+ $(,)?)) => {
        $crate::test_type!($name<$name>($db, $($text == $value),+));
    };
}

// Test type decoding only
#[macro_export]
macro_rules! test_decode_type {
    ($name:ident<$ty:ty>($db:ident, $($text:literal == $value:expr),+ $(,)?)) => {
        $crate::__test_prepared_decode_type!($name<$ty>($db, $($text == $value),+));
        $crate::test_unprepared_type!($name<$ty>($db, $($text == $value),+));
    };

    ($name:ident($db:ident, $($text:literal == $value:expr),+ $(,)?)) => {
        $crate::test_decode_type!($name<$name>($db, $($text == $value),+));
    };
}

// Test type encoding and decoding
#[macro_export]
macro_rules! test_prepared_type {
    ($name:ident<$ty:ty>($db:ident, $sql:literal, $($text:literal == $value:expr),+ $(,)?)) => {
        $crate::__test_prepared_type!($name<$ty>($db, $sql, $($text == $value),+));
    };

    ($name:ident<$ty:ty>($db:ident, $($text:literal == $value:expr),+ $(,)?)) => {
        paste::item! {
            $crate::__test_prepared_type!($name<$ty>($db, $crate::[< $db _query_for_test_prepared_type >]!(), $($text == $value),+));
        }
    };


    ($name:ident($db:ident, $($text:literal == $value:expr),+ $(,)?)) => {
        $crate::__test_prepared_type!($name<$name>($db, $($text == $value),+));
    };
}

// Test type decoding for the simple (unprepared) query API
#[macro_export]
macro_rules! test_unprepared_type {
    ($name:ident<$ty:ty>($db:ident, $($text:literal == $value:expr),+ $(,)?)) => {
        paste::item! {
            #[tokio::test]
            async fn [< test_unprepared_type_ $name >] () -> anyhow::Result<()> {
                use sqlx::prelude::*;
                use sqlx::AssertSqlSafe;
                use futures_util::TryStreamExt;

                let mut conn = $crate::common::new().await?;

                $(
                    let query = format!("SELECT {}", $text);
                    let mut s = conn.fetch(AssertSqlSafe(query));
                    let row = s.try_next().await?.unwrap();
                    let rec = row.try_get::<$ty, _>(0)?;

                    assert_eq!($value, rec);

                    drop(s);
                )+

                Ok(())
            }
        }
    }
}

// Test type decoding only for the prepared query API
#[macro_export]
macro_rules! __test_prepared_decode_type {
    ($name:ident<$ty:ty>($db:ident, $($text:literal == $value:expr),+ $(,)?)) => {
        paste::item! {
            #[tokio::test]
            async fn [< test_prepared_decode_type_ $name >] () -> anyhow::Result<()> {
                use sqlx::AssertSqlSafe;

                let mut conn = $crate::common::new().await?;

                $(
                    let query = format!("SELECT {}", $text);

                    let row = sqlx::query(AssertSqlSafe(query))
                        .fetch_one(&mut conn)
                        .await?;

                    let rec: $ty = row.try_get(0)?;

                    assert_eq!($value, rec);
                )+

                Ok(())
            }
        }
    };
}

// Test type encoding and decoding for the prepared query API
#[macro_export]
macro_rules! __test_prepared_type {
    ($name:ident<$ty:ty>($db:ident, $sql:expr, $($text:literal == $value:expr),+ $(,)?)) => {
        paste::item! {
            #[tokio::test]
            async fn [< test_prepared_type_ $name >] () -> anyhow::Result<()> {
                use sqlx::AssertSqlSafe;

                let mut conn = $crate::common::new().await?;

                $(
                    let query = format!($sql, $text);
                    println!("{query}");

                    let row = sqlx::query(AssertSqlSafe(query))
                        .bind($value)
                        .bind($value)
                        .fetch_one(&mut conn)
                        .await?;

                    let matches: i32 = row.try_get(0)?;
                    let returned: $ty = row.try_get(1)?;
                    let round_trip: $ty = row.try_get(2)?;

                    assert!(matches != 0,
                            "[1] DB value mismatch; given value: {:?}\n\
                             as returned: {:?}\n\
                             round-trip: {:?}",
                            $value, returned, round_trip);

                    assert_eq!($value, returned,
                            "[2] DB value mismatch; given value: {:?}\n\
                                     as returned: {:?}\n\
                                     round-trip: {:?}",
                                    $value, returned, round_trip);

                    assert_eq!($value, round_trip,
                            "[3] DB value mismatch; given value: {:?}\n\
                                     as returned: {:?}\n\
                                     round-trip: {:?}",
                                    $value, returned, round_trip);
                )+

                Ok(())
            }
        }
    };
}

#[macro_export]
macro_rules! Postgres_query_for_test_prepared_type {
    () => {
        "SELECT ({0} is not distinct from $1)::int4, {0}, $2"
    };
}

#[macro_export]
macro_rules! Postgres_query_for_test_prepared_geometric_type {
    () => {
        "SELECT ({0} ~= $1)::int4, {0}, $2"
    };
}

#[macro_export]
macro_rules! Postgres_query_for_test_prepared_geometric_array_type {
    () => {
        "SELECT (SELECT bool_and(geo1.geometry ~= geo2.geometry) FROM unnest({0}) WITH ORDINALITY AS geo1(geometry, idx) JOIN unnest($1) WITH ORDINALITY AS geo2(geometry, idx) ON geo1.idx = geo2.idx)::int4, {0}, $2"
    };
}
