use std::marker::PhantomData;

use either::Either;
use futures_core::stream::BoxStream;
use futures_util::{StreamExt, TryStreamExt};

use crate::arguments::IntoArguments;
use crate::encode::Encode;
use crate::error::{BoxDynError, Error};
use crate::executor::Execute;
use crate::Connection;
use crate::from_row::FromRow;
use crate::query::{query, query_statement, query_statement_with, query_with_result, Query};
use crate::sql_str::{SqlSafeStr, SqlStr};
use crate::codec::Type;

/// A single SQL query as a prepared statement, mapping results using [`FromRow`].
/// Returned by [`query_as()`].
#[must_use = "query must be executed to affect database"]
pub struct QueryAs<'q, O, A> {
    pub(crate) inner: Query<'q, A>,
    pub(crate) output: PhantomData<O>,
}

impl<'q, O: Send, A: Send> Execute<'q> for QueryAs<'q, O, A>
where
    A: 'q + IntoArguments,
{
    #[inline]
    fn sql(self) -> SqlStr {
        self.inner.sql()
    }

    #[inline]
    fn statement(&self) -> Option<&crate::Statement> {
        self.inner.statement()
    }

    #[inline]
    fn take_arguments(&mut self) -> Result<Option<crate::Arguments>, BoxDynError> {
        self.inner.take_arguments()
    }

    #[inline]
    fn persistent(&self) -> bool {
        Execute::persistent(&self.inner)
    }
}

impl<'q, O> QueryAs<'q, O, crate::Arguments> {
    /// Bind a value for use with this SQL query.
    ///
    /// See [`Query::bind`](Query::bind).
    pub fn bind<T: 'q + Encode<'q> + Type>(mut self, value: T) -> Self {
        self.inner = self.inner.bind(value);
        self
    }
}

impl<O, A> QueryAs<'_, O, A> {
    /// If `true`, the statement will get prepared once and cached to the
    /// connection's statement cache.
    ///
    /// If queried once with the flag set to `true`, all subsequent queries
    /// matching the one with the flag will use the cached statement until the
    /// cache is cleared.
    ///
    /// If `false`, the prepared statement will be closed after execution.
    ///
    /// Default: `true`.
    pub fn persistent(mut self, value: bool) -> Self {
        self.inner = self.inner.persistent(value);
        self
    }
}

// FIXME: This is very close, nearly 1:1 with `Map`
// noinspection DuplicatedCode
impl<'q, O, A> QueryAs<'q, O, A>
where
    A: 'q + IntoArguments,
    O: Send + Unpin + for<'r> FromRow<'r>,
{
    /// Execute the query and return the generated results as a stream.
    pub fn fetch<'c>(self, executor: &'c mut Connection) -> BoxStream<'c, Result<O, Error>>
    where
        'q: 'c,
        O: 'c,
        A: 'c,
    {
        executor
            .fetch(self.inner)
            .map(|row| O::from_row(&row?))
            .boxed()
    }

    /// Execute multiple queries and return the generated results as a stream
    /// from each query, in a stream.
    #[deprecated = "Multi-statement prepared queries are not supported. Use `sqlx::raw_sql()` instead."]
    pub fn fetch_many<'c>(
        self,
        executor: &'c mut Connection,
    ) -> BoxStream<'c, Result<Either<crate::QueryResult, O>, Error>>
    where
        'q: 'c,
        O: 'c,
        A: 'c,
    {
        executor
            .fetch_many(self.inner)
            .map(|v| match v {
                Ok(Either::Right(row)) => O::from_row(&row).map(Either::Right),
                Ok(Either::Left(v)) => Ok(Either::Left(v)),
                Err(e) => Err(e),
            })
            .boxed()
    }

    /// Execute the query and return all the resulting rows collected into a [`Vec`].
    #[inline]
    pub async fn fetch_all<'c>(self, executor: &'c mut Connection) -> Result<Vec<O>, Error>
    where
        'q: 'c,
        O: 'c,
        A: 'c,
    {
        self.fetch(executor).try_collect().await
    }

    /// Execute the query, returning the first row or [`Error::RowNotFound`] otherwise.
    pub async fn fetch_one<'c>(self, executor: &'c mut Connection) -> Result<O, Error>
    where
        'q: 'c,
        O: 'c,
        A: 'c,
    {
        self.fetch_optional(executor)
            .await
            .and_then(|row| row.ok_or(Error::RowNotFound))
    }

    /// Execute the query, returning the first row or `None` otherwise.
    pub async fn fetch_optional<'c>(self, executor: &'c mut Connection) -> Result<Option<O>, Error>
    where
        'q: 'c,
        O: 'c,
        A: 'c,
    {
        let row = executor.fetch_optional(self.inner).await?;
        if let Some(row) = row {
            O::from_row(&row).map(Some)
        } else {
            Ok(None)
        }
    }
}

/// Execute a single SQL query as a prepared statement (transparently cached).
/// Maps rows to Rust types using [`FromRow`].
///
/// For details about prepared statements and allowed SQL syntax, see [`query()`][crate::query::query].
///
/// ### Example: Map Rows using Tuples
/// [`FromRow`] is implemented for tuples of up to 16 elements<sup>1</sup>.
/// Using a tuple of N elements will extract the first N columns from each row using [`Decode`][crate::decode::Decode].
/// Any extra columns are ignored.
///
/// See [`sqlx::types`][crate::types] for the types that can be used.
///
/// The `FromRow` implementation will check [`Type::compatible()`] for each column to ensure a compatible type mapping
/// is used. If an incompatible mapping is detected, an error is returned.
/// To statically assert compatible types at compile time, see the `query!()` family of macros.
///
/// **NOTE**: `SELECT *` is not recommended with this approach because the ordering of returned columns may be different
/// than expected, especially when using joins.
///
/// ```rust,no_run
/// # async fn example1() -> sqlx::Result<()> {
/// use sqlx::Connection;
///
/// let mut conn: Connection = Connection::connect("<Database URL>").await?;
///
/// sqlx::raw_sql(
///     "CREATE TABLE users(id INTEGER PRIMARY KEY, username TEXT UNIQUE)"
/// )
///     .execute(&mut conn)
///     .await?;
///
/// sqlx::query("INSERT INTO users(id, username) VALUES (1, 'alice'), (2, 'bob');")
///     .execute(&mut conn)
///     .await?;
///
/// // Get the first row of the result (note the `LIMIT 1` for efficiency)
/// let oldest_user: (i32, String) = sqlx::query_as(
///     "SELECT id, username FROM users ORDER BY id LIMIT 1"
/// )
///     .fetch_one(&mut conn)
///     .await?;
///
/// assert_eq!(oldest_user.0, 1);
/// assert_eq!(oldest_user.1, "alice");
///
/// // Get at most one row
/// let maybe_charlie: Option<(i32, String)> = sqlx::query_as(
///     "SELECT id, username FROM users WHERE username = 'charlie'"
/// )
///     .fetch_optional(&mut conn)
///     .await?;
///
/// assert_eq!(maybe_charlie, None);
///
/// // Get all rows in result (Beware of the size of the result set! Consider using `LIMIT`)
/// let users: Vec<(i32, String)> = sqlx::query_as(
///     "SELECT id, username FROM users ORDER BY id"
/// )
///     .fetch_all(&mut conn)
///     .await?;
///
/// println!("{users:?}");
/// # Ok(())
/// # }
/// ```
///
/// <sup>1</sup>: It's impossible in Rust to implement a trait for tuples of arbitrary size.
/// For larger result sets, either use an explicit struct (see below) or use [`query()`][crate::query::query]
/// instead and extract columns dynamically.
///
/// ### Example: Map Rows into a struct via [`FromRow`]
/// Implement [`FromRow`] on a struct to map result rows by field name (instead of tuple index).
/// This lets `SELECT *` be safe to use, though duplicate column names from joins still require care.
///
/// A `FromRow` impl looks up each column by name via [`Row::try_get`](crate::row::Row::try_get),
/// which checks [`Type::compatible()`] and returns an error on type mismatch or missing column.
#[inline]
pub fn query_as<'q, O>(sql: impl SqlSafeStr) -> QueryAs<'q, O, crate::Arguments>
where
    O: for<'r> FromRow<'r>,
{
    QueryAs {
        inner: query(sql),
        output: PhantomData,
    }
}

/// Execute a single SQL query, with the given arguments as a prepared statement (transparently cached).
/// Maps rows to Rust types using [`FromRow`].
///
/// For details about prepared statements and allowed SQL syntax, see [`query()`][crate::query::query].
///
/// For details about type mapping from [`FromRow`], see [`query_as()`].
#[inline]
pub fn query_as_with<'q, O, A>(sql: impl SqlSafeStr, arguments: A) -> QueryAs<'q, O, A>
where
    A: IntoArguments,
    O: for<'r> FromRow<'r>,
{
    query_as_with_result(sql, Ok(arguments))
}

/// Same as [`query_as_with`] but takes arguments as a Result
#[inline]
pub fn query_as_with_result<'q, O, A>(
    sql: impl SqlSafeStr,
    arguments: Result<A, BoxDynError>,
) -> QueryAs<'q, O, A>
where
    A: IntoArguments,
    O: for<'r> FromRow<'r>,
{
    QueryAs {
        inner: query_with_result(sql, arguments),
        output: PhantomData,
    }
}

// Make a SQL query from a statement, that is mapped to a concrete type.
pub fn query_statement_as<O>(
    statement: &crate::Statement,
) -> QueryAs<'_, O, crate::Arguments>
where
    O: for<'r> FromRow<'r>,
{
    QueryAs {
        inner: query_statement(statement),
        output: PhantomData,
    }
}

// Make a SQL query from a statement, with the given arguments, that is mapped to a concrete type.
pub fn query_statement_as_with<'q, O, A>(
    statement: &'q crate::Statement,
    arguments: A,
) -> QueryAs<'q, O, A>
where
    A: IntoArguments,
    O: for<'r> FromRow<'r>,
{
    QueryAs {
        inner: query_statement_with(statement, arguments),
        output: PhantomData,
    }
}
