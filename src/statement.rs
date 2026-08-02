use super::{Column, TypeInfo};
use crate::column::ColumnIndex;
use crate::error::Error;
use crate::ustr::UStr;
use crate::sql_str::SqlStr;
use crate::{Either, HashMap};
use crate::Arguments;
use std::sync::Arc;

use crate::arguments::IntoArguments;
use crate::from_row::FromRow;
use crate::query::{query_statement, query_statement_with, Query};
use crate::query_as::{query_statement_as, query_statement_as_with, QueryAs};
use crate::query_scalar::{
    query_statement_scalar, query_statement_scalar_with, QueryScalar,
};

/// An explicitly prepared statement.
///
/// Statements are prepared and cached by default, per connection. This type allows you to
/// look at that cache in-between the statement being prepared and it being executed. This contains
/// the expected columns to be returned and the expected parameter types.
///
/// Statements can be re-used with any connection and on first-use it will be re-prepared and
/// cached within the connection.
#[derive(Debug, Clone)]
pub struct Statement {
    pub(crate) sql: SqlStr,
    pub(crate) metadata: Arc<StatementMetadata>,
}

#[derive(Debug, Default)]
pub(crate) struct StatementMetadata {
    pub(crate) columns: Vec<Column>,
    // This `Arc` is not redundant; it's used to avoid deep-copying this map for the `Any` backend.
    // See `sqlx-postgres/src/any.rs`
    pub(crate) column_names: Arc<HashMap<UStr, usize>>,
    pub(crate) parameters: Vec<TypeInfo>,
}

impl Statement {
    /// Get the original SQL text used to create this statement.
    pub fn into_sql(self) -> SqlStr {
        self.sql
    }

    /// Get the original SQL text used to create this statement.
    pub fn sql(&self) -> &SqlStr {
        &self.sql
    }

    /// Get the expected parameters for this statement.
    ///
    /// PostgreSQL provides full type information.
    pub fn parameters(&self) -> Option<Either<&[TypeInfo], usize>> {
        Some(Either::Left(&self.metadata.parameters))
    }

    /// Get the columns expected to be returned by executing this statement.
    pub fn columns(&self) -> &[Column] {
        &self.metadata.columns
    }

    /// Gets the column information at `index`.
    ///
    /// # Panics
    /// Panics if `index` is out of bounds.
    pub fn column<I>(&self, index: I) -> &Column
    where
        I: ColumnIndex<Self>,
    {
        self.try_column(index).unwrap()
    }

    /// Gets the column information at `index` or a `ColumnIndexOutOfBounds` error if out of bounds.
    pub fn try_column<I>(&self, index: I) -> Result<&Column, Error>
    where
        I: ColumnIndex<Self>,
    {
        Ok(&self.columns()[index.index(self)?])
    }

    #[inline]
    pub fn query(&self) -> Query<'_, Arguments> {
        query_statement(self)
    }

    #[inline]
    pub fn query_with<A>(&self, arguments: A) -> Query<'_, A>
    where
        A: IntoArguments,
    {
        query_statement_with(self, arguments)
    }

    #[inline]
    pub fn query_as<O>(&self) -> QueryAs<'_, O, Arguments>
    where
        O: for<'r> FromRow<'r>,
    {
        query_statement_as(self)
    }

    #[inline]
    pub fn query_as_with<'s, O, A>(&'s self, arguments: A) -> QueryAs<'s, O, A>
    where
        O: for<'r> FromRow<'r>,
        A: IntoArguments,
    {
        query_statement_as_with(self, arguments)
    }

    #[inline]
    pub fn query_scalar<O>(&self) -> QueryScalar<'_, O, Arguments>
    where
        (O,): for<'r> FromRow<'r>,
    {
        query_statement_scalar(self)
    }

    #[inline]
    pub fn query_scalar_with<'s, O, A>(&'s self, arguments: A) -> QueryScalar<'s, O, A>
    where
        (O,): for<'r> FromRow<'r>,
        A: IntoArguments,
    {
        query_statement_scalar_with(self, arguments)
    }
}

impl ColumnIndex<Statement> for &'_ str {
    fn index(&self, statement: &Statement) -> Result<usize, Error> {
        statement
            .metadata
            .column_names
            .get(*self)
            .ok_or_else(|| Error::ColumnNotFound((*self).into()))
            .copied()
    }
}
