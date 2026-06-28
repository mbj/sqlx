use either::Either;
use futures_core::future::BoxFuture;
use futures_core::stream::BoxStream;

use crate::error::BoxDynError;
use crate::executor::Execute;
use crate::Connection;
use crate::sql_str::{SqlSafeStr, SqlStr};
use crate::Error;

// AUTHOR'S NOTE: I was just going to call this API `sql()` and `Sql`, respectively,
// but realized that would be extremely annoying to deal with
// because IDE smart completion would always recommend the `Sql` type first.
//
// It doesn't really need a super convenient name anyway as it's not meant to be used very often.

/// One or more raw SQL statements, separated by semicolons (`;`).
///
/// See [`raw_sql()`] for details.
pub struct RawSql(SqlStr);

/// Execute one or more statements as raw SQL, separated by semicolons (`;`).
///
/// This interface can be used to execute both DML
/// (Data Manipulation Language: `SELECT`, `INSERT`, `UPDATE`, `DELETE` and variants)
/// as well as DDL (Data Definition Language: `CREATE TABLE`, `ALTER TABLE`, etc).
///
/// This will not create or cache any prepared statements.
///
/// ### Note: singular DML queries, prefer `query()`
/// This API does not use prepared statements, so usage of it is missing out on their benefits.
///
/// Prefer [`query()`][crate::query::query] instead if executing a single query.
///
/// It's also possible to combine multiple DML queries into one for use with `query()`:
///
/// ##### Common Table Expressions (CTEs: i.e The `WITH` Clause)
/// Common Table Expressions effectively allow you to define aliases for queries
/// that can be referenced like temporary tables:
///
/// ```sql
/// WITH inserted_foos AS (
///     INSERT INTO foo (bar_id) VALUES ($1)
///     RETURNING foo_id, bar_id
/// )
/// SELECT foo_id, bar_id, bar
/// FROM inserted_foos
/// INNER JOIN bar USING (bar_id)
/// ```
///
/// All data-modifying subqueries in a `WITH` clause execute with the same view of the data;
/// they *cannot* see each other's modifications.
///
/// See [the Postgres manual on `WITH`](https://www.postgresql.org/docs/current/queries-with.html)
/// for details.
///
/// ##### `UNION`/`INTERSECT`/`EXCEPT`
/// You can also use various set-theory operations on queries,
/// including `UNION ALL` which simply concatenates their results.
///
/// See [the Postgres manual on `UNION`](https://www.postgresql.org/docs/current/queries-union.html)
/// for details.
///
/// ### Note: query parameters are not supported.
/// Query parameters require the use of prepared statements which this API does support.
///
/// If you require dynamic input data in your SQL, you can use `format!()` but **be very careful
/// doing this with user input**. SQLx does **not** provide escaping or sanitization for inserting
/// dynamic input into queries this way.
///
/// See [`query()`][crate::query::query] for details.
///
/// ### Note: multiple statements and autocommit.
/// By default, when you use this API to execute a SQL string containing multiple statements
/// separated by semicolons (`;`), the database server will treat those statements as all executing
/// within the same transaction block, i.e. wrapped in `BEGIN` and `COMMIT`:
///
/// ```rust,no_run
/// # async fn example() -> sqlx::Result<()> {
/// let mut conn: sqlx::Connection = todo!("e.g. Connection::connect(<DATABASE URL>)");
///
/// sqlx::raw_sql(
///     // Imagine we're moving data from one table to another:
///     // Implicit `BEGIN;`
///     "UPDATE foo SET bar = foobar.bar FROM foobar WHERE foobar.foo_id = foo.id;\
///      DELETE FROM foobar;"
///     // Implicit `COMMIT;`
/// )
///    .execute(&mut conn)
///    .await?;
///
/// # Ok(())
/// # }
/// ```
///
/// If one statement triggers an error, the whole script aborts and rolls back.
/// You can include explicit `BEGIN` and `COMMIT` statements in the SQL string
/// to designate units that can be committed or rolled back piecemeal.
///
/// This also allows for a rudimentary form of pipelining as the whole SQL string is sent in one go.
pub fn raw_sql(sql: impl SqlSafeStr) -> RawSql {
    RawSql(sql.into_sql_str())
}

impl Execute<'_> for RawSql {
    fn sql(self) -> SqlStr {
        self.0
    }

    fn statement(&self) -> Option<&crate::Statement> {
        None
    }

    fn take_arguments(&mut self) -> Result<Option<crate::Arguments>, BoxDynError> {
        Ok(None)
    }

    fn persistent(&self) -> bool {
        false
    }
}

impl RawSql {
    /// Execute the SQL string and return the total number of rows affected.
    #[inline]
    pub async fn execute(self, executor: &mut Connection) -> crate::Result<crate::QueryResult> {
        executor.execute(self).await
    }

    /// Execute the SQL string. Returns a stream which gives the number of rows affected for each statement in the string.
    #[inline]
    pub fn execute_many<'e>(
        self,
        executor: &'e mut Connection,
    ) -> BoxStream<'e, crate::Result<crate::QueryResult>>
    {
        executor.execute_many(self)
    }

    /// Execute the SQL string and return the generated results as a stream.
    ///
    /// If the string contains multiple statements, their results will be concatenated together.
    #[inline]
    pub fn fetch<'e>(self, executor: &'e mut Connection) -> BoxStream<'e, Result<crate::Row, Error>>
    {
        executor.fetch(self)
    }

    /// Execute the SQL string and return the generated results as a stream.
    ///
    /// For each query in the stream, any generated rows are returned first,
    /// then the `QueryResult` with the number of rows affected.
    #[inline]
    pub fn fetch_many<'e>(
        self,
        executor: &'e mut Connection,
    ) -> BoxStream<'e, Result<Either<crate::QueryResult, crate::Row>, Error>>
    {
        executor.fetch_many(self)
    }

    /// Execute the SQL string and return all the resulting rows collected into a [`Vec`].
    ///
    /// ### Note: beware result set size.
    /// This will attempt to collect the full result set of the query into memory.
    ///
    /// To avoid exhausting available memory, ensure the result set has a known upper bound,
    /// e.g. using `LIMIT`.
    #[inline]
    pub fn fetch_all<'e>(self, executor: &'e mut Connection) -> BoxFuture<'e, crate::Result<Vec<crate::Row>>>
    {
        executor.fetch_all(self)
    }

    /// Execute the SQL string, returning the first row or [`Error::RowNotFound`] otherwise.
    ///
    /// ### Note: for best performance, ensure the query returns at most one row.
    /// Depending on the driver implementation, if your query can return more than one row,
    /// it may lead to wasted CPU time and bandwidth on the database server.
    ///
    /// Even when the driver implementation takes this into account, ensuring the query returns
    /// at most one row can result in a more optimal query plan.
    ///
    /// If your query has a `WHERE` clause filtering a unique column by a single value, you're good.
    ///
    /// Otherwise, you might want to add `LIMIT 1` to your query.
    #[inline]
    pub fn fetch_one<'e>(self, executor: &'e mut Connection) -> BoxFuture<'e, crate::Result<crate::Row>>
    {
        executor.fetch_one(self)
    }

    /// Execute the SQL string, returning the first row or [`None`] otherwise.
    ///
    /// ### Note: for best performance, ensure the query returns at most one row.
    /// Depending on the driver implementation, if your query can return more than one row,
    /// it may lead to wasted CPU time and bandwidth on the database server.
    ///
    /// Even when the driver implementation takes this into account, ensuring the query returns
    /// at most one row can result in a more optimal query plan.
    ///
    /// If your query has a `WHERE` clause filtering a unique column by a single value, you're good.
    ///
    /// Otherwise, you might want to add `LIMIT 1` to your query.
    #[inline]
    pub async fn fetch_optional(self, executor: &mut Connection) -> crate::Result<Option<crate::Row>> {
        executor.fetch_optional(self).await
    }
}
