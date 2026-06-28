use crate::error::BoxDynError;
use crate::sql_str::{SqlSafeStr, SqlStr};

/// A type that may be executed against a database connection.
///
/// Implemented for the following:
///
///  * [`&str`](std::str)
///  * [`Query`](super::query::Query)
///
pub trait Execute<'q>: Send + Sized {
    /// Gets the SQL that will be executed.
    fn sql(self) -> SqlStr;

    /// Gets the previously cached statement, if available.
    fn statement(&self) -> Option<&crate::Statement>;

    /// Returns the arguments to be bound against the query string.
    ///
    /// Returning `Ok(None)` for `Arguments` indicates to use a "simple" query protocol and to not
    /// prepare the query. Returning `Ok(Some(Default::default()))` is an empty arguments object that
    /// will be prepared (and cached) before execution.
    ///
    /// Returns `Err` if encoding any of the arguments failed.
    fn take_arguments(&mut self) -> Result<Option<crate::Arguments>, BoxDynError>;

    /// Returns `true` if the statement should be cached.
    fn persistent(&self) -> bool;
}

impl<T> Execute<'_> for T
where
    T: SqlSafeStr + Send,
{
    #[inline]
    fn sql(self) -> SqlStr {
        self.into_sql_str()
    }

    #[inline]
    fn statement(&self) -> Option<&crate::Statement> {
        None
    }

    #[inline]
    fn take_arguments(&mut self) -> Result<Option<crate::Arguments>, BoxDynError> {
        Ok(None)
    }

    #[inline]
    fn persistent(&self) -> bool {
        true
    }
}

impl<T> Execute<'_> for (T, Option<crate::Arguments>)
where
    T: SqlSafeStr + Send,
{
    #[inline]
    fn sql(self) -> SqlStr {
        self.0.into_sql_str()
    }

    #[inline]
    fn statement(&self) -> Option<&crate::Statement> {
        None
    }

    #[inline]
    fn take_arguments(&mut self) -> Result<Option<crate::Arguments>, BoxDynError> {
        Ok(self.1.take())
    }

    #[inline]
    fn persistent(&self) -> bool {
        true
    }
}
