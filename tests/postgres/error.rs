use sqlx::error::ErrorKind;

mod common;
use common::new;

#[tokio::test]
async fn it_fails_with_unique_violation() -> anyhow::Result<()> {
    let mut conn = new().await?;

    sqlx::query("INSERT INTO tweet(id, text, owner_id) VALUES (1, 'Foo', 1);")
        .execute(&mut conn)
        .await?;

    let res: Result<_, sqlx::Error> = sqlx::query("INSERT INTO tweet VALUES (1, NOW(), 'Foo', 1);")
        .execute(&mut conn)
        .await;
    let err = res.unwrap_err();

    let err = err.into_database_error().unwrap();

    assert_eq!(err.kind(), ErrorKind::UniqueViolation);

    Ok(())
}

#[tokio::test]
async fn it_fails_with_foreign_key_violation() -> anyhow::Result<()> {
    let mut conn = new().await?;

    let res: Result<_, sqlx::Error> =
        sqlx::query("INSERT INTO tweet_reply (tweet_id, text) VALUES (1, 'Reply!');")
            .execute(&mut conn)
            .await;
    let err = res.unwrap_err();

    let err = err.into_database_error().unwrap();

    assert_eq!(err.kind(), ErrorKind::ForeignKeyViolation);

    Ok(())
}

#[tokio::test]
async fn it_fails_with_not_null_violation() -> anyhow::Result<()> {
    let mut conn = new().await?;

    let res: Result<_, sqlx::Error> = sqlx::query("INSERT INTO tweet (text) VALUES (null);")
        .execute(&mut conn)
        .await;
    let err = res.unwrap_err();

    let err = err.into_database_error().unwrap();

    assert_eq!(err.kind(), ErrorKind::NotNullViolation);

    Ok(())
}

#[tokio::test]
async fn it_fails_with_check_violation() -> anyhow::Result<()> {
    let mut conn = new().await?;

    let res: Result<_, sqlx::Error> =
        sqlx::query("INSERT INTO products VALUES (1, 'Product 1', 0);")
            .execute(&mut conn)
            .await;
    let err = res.unwrap_err();

    let err = err.into_database_error().unwrap();

    assert_eq!(err.kind(), ErrorKind::CheckViolation);

    Ok(())
}

#[tokio::test]
async fn it_fails_with_exclude_violation() -> anyhow::Result<()> {
    let mut conn = new().await?;

    sqlx::query("INSERT INTO circles VALUES (circle('(0,0)'::point, 5.0));")
        .execute(&mut conn)
        .await?;

    let res: Result<_, sqlx::Error> =
        sqlx::query("INSERT INTO circles VALUES (circle('(0,2.0)'::point, 2.0));")
            .execute(&mut conn)
            .await;
    let err = res.unwrap_err();

    let err = err.into_database_error().unwrap();

    assert_eq!(err.kind(), ErrorKind::ExclusionViolation);

    Ok(())
}
