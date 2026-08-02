//! Types for working with errors produced by SQLx.

use std::any::type_name;
use std::borrow::Cow;
use std::error::Error as StdError;
use std::fmt::{self, Debug, Display, Formatter};
use std::io;

use atoi::atoi;
use crate::message::{BackendMessage, BackendMessageFormat, Notice, Severity};
use crate::bytes::Bytes;
use crate::codec::Type;

/// A specialized `Result` type for SQLx.
pub type Result<T, E = Error> = ::std::result::Result<T, E>;

// Convenience type alias for usage within SQLx.
// Do not make this type public.
pub type BoxDynError = Box<dyn StdError + 'static + Send + Sync>;

/// An unexpected `NULL` was encountered during decoding.
///
/// Returned from `Row::get` if the value from the database is `NULL`,
/// and you are not decoding into an `Option`.
#[derive(thiserror::Error, Debug)]
#[error("unexpected null; try decoding as an `Option`")]
pub struct UnexpectedNullError;

/// Represents all the ways a method can fail within SQLx.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum Error {
    /// Error occurred while parsing a connection string.
    #[error("error with configuration: {0}")]
    Configuration(#[source] BoxDynError),

    /// One or more of the arguments to the called function was invalid.
    ///
    /// The string contains more information.
    #[error("{0}")]
    InvalidArgument(String),

    /// Error returned from the database.
    #[error("error returned from database: {0}")]
    Database(#[source] Box<DatabaseError>),

    /// Error communicating with the database backend.
    #[error("error communicating with database: {0}")]
    Io(#[from] io::Error),

    /// Error occurred while attempting to establish a TLS connection.
    #[error("error occurred while attempting to establish a TLS connection: {0}")]
    Tls(#[source] BoxDynError),

    /// Unexpected or invalid data encountered while communicating with the database.
    #[error("encountered unexpected or invalid data: {0}")]
    Protocol(String),

    /// No rows returned by a query that expected to return at least one row.
    #[error("no rows returned by a query that expected to return at least one row")]
    RowNotFound,

    /// Type in query doesn't exist. Likely due to typo or missing user type.
    #[error("type named {type_name} not found")]
    TypeNotFound { type_name: String },

    /// Column index was out of bounds.
    #[error("column index out of bounds: the len is {len}, but the index is {index}")]
    ColumnIndexOutOfBounds { index: usize, len: usize },

    /// No column found for the given name.
    #[error("no column found for name: {0}")]
    ColumnNotFound(String),

    /// Error occurred while decoding a value from a specific column.
    #[error("error occurred while decoding column {index}: {source}")]
    ColumnDecode {
        index: String,

        #[source]
        source: BoxDynError,
    },

    /// Error occurred while encoding a value.
    #[error("error occurred while encoding a value: {0}")]
    Encode(#[source] BoxDynError),

    /// Error occurred while decoding a value.
    #[error("error occurred while decoding: {0}")]
    Decode(#[source] BoxDynError),

    /// A background worker has crashed.
    #[error("attempted to communicate with a crashed background worker")]
    WorkerCrashed,

    #[error("attempted to call begin_with at non-zero transaction depth")]
    InvalidSavePointStatement,

    #[error("got unexpected connection status after attempting to begin transaction")]
    BeginFailed,
}

impl Error {
    /// Consumes this error and returns the underlying database error if applicable.
    pub fn into_database_error(self) -> Option<Box<DatabaseError>> {
        match self {
            Error::Database(err) => Some(err),
            _ => None,
        }
    }

    /// Returns the underlying database error if applicable.
    pub fn as_database_error(&self) -> Option<&DatabaseError> {
        match self {
            Error::Database(err) => Some(err),
            _ => None,
        }
    }

    #[doc(hidden)]
    #[inline]
    pub fn protocol(err: impl Display) -> Self {
        Error::Protocol(err.to_string())
    }

    #[doc(hidden)]
    #[inline]
    pub fn database(err: DatabaseError) -> Self {
        Error::Database(Box::new(err))
    }

    #[doc(hidden)]
    #[inline]
    pub fn config(err: impl StdError + Send + Sync + 'static) -> Self {
        Error::Configuration(err.into())
    }

    pub(crate) fn tls(err: impl Into<Box<dyn StdError + Send + Sync + 'static>>) -> Self {
        Error::Tls(err.into())
    }

    #[doc(hidden)]
    #[inline]
    pub fn decode(err: impl Into<Box<dyn StdError + Send + Sync + 'static>>) -> Self {
        Error::Decode(err.into())
    }
}

pub fn mismatched_types<T: Type>(ty: &crate::TypeInfo) -> BoxDynError {
    format!(
        "mismatched types; Rust type `{}` (as SQL type `{}`) is not compatible with SQL type `{}`",
        type_name::<T>(),
        <T as Type>::type_info().name(),
        ty.name()
    )
    .into()
}

/// The error kind.
///
/// This enum is used to identify frequent errors that can be handled by the program.
#[derive(Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum ErrorKind {
    /// Unique/primary key constraint violation.
    UniqueViolation,
    /// Foreign key constraint violation.
    ForeignKeyViolation,
    /// Not-null constraint violation.
    NotNullViolation,
    /// Check constraint violation.
    CheckViolation,
    /// Exclusion constraint violation.
    ExclusionViolation,
    /// An unmapped error.
    Other,
}

impl From<DatabaseError> for Error {
    #[inline]
    fn from(error: DatabaseError) -> Self {
        Error::Database(Box::new(error))
    }
}

/// Format an error message as a `Protocol` error
#[macro_export]
macro_rules! err_protocol {
    ($($fmt_args:tt)*) => {
        $crate::error::Error::Protocol(
            format!(
                "{} ({}:{})",
                // Note: the format string needs to be unmodified (e.g. by `concat!()`)
                // for implicit formatting arguments to work
                format_args!($($fmt_args)*),
                module_path!(),
                line!(),
            )
        )
    };
}

/// An error returned from the PostgreSQL database.
pub struct DatabaseError(pub(crate) Notice);

// Error message fields are documented:
// https://www.postgresql.org/docs/current/protocol-error-fields.html

impl DatabaseError {
    #[inline]
    pub fn severity(&self) -> Severity {
        self.0.severity()
    }

    /// The [SQLSTATE](https://www.postgresql.org/docs/current/errcodes-appendix.html) code for
    /// this error.
    #[inline]
    pub fn code(&self) -> &str {
        self.0.code()
    }

    /// The primary human-readable error message. This should be accurate but
    /// terse (typically one line).
    #[inline]
    pub fn message(&self) -> &str {
        self.0.message()
    }

    /// An optional secondary error message carrying more detail about the problem.
    /// Might run to multiple lines.
    #[inline]
    pub fn detail(&self) -> Option<&str> {
        self.0.get(b'D')
    }

    /// An optional suggestion what to do about the problem.
    #[inline]
    pub fn hint(&self) -> Option<&str> {
        self.0.get(b'H')
    }

    /// Indicates an error cursor position as an index into the original query string; or,
    /// a position into an internally generated query.
    #[inline]
    pub fn position(&self) -> Option<ErrorPosition<'_>> {
        self.0
            .get_raw(b'P')
            .and_then(atoi)
            .map(ErrorPosition::Original)
            .or_else(|| {
                let position = self.0.get_raw(b'p').and_then(atoi)?;
                let query = self.0.get(b'q')?;

                Some(ErrorPosition::Internal { position, query })
            })
    }

    /// An indication of the context in which the error occurred.
    pub fn r#where(&self) -> Option<&str> {
        self.0.get(b'W')
    }

    /// If this error is with a specific database object, the
    /// name of the schema containing that object.
    pub fn schema(&self) -> Option<&str> {
        self.0.get(b's')
    }

    /// If this error is with a specific table, the name of the table.
    pub fn table(&self) -> Option<&str> {
        self.0.get(b't')
    }

    /// If the error is with a specific table column, the name of the column.
    pub fn column(&self) -> Option<&str> {
        self.0.get(b'c')
    }

    /// If the error is with a specific data type, the name of the data type.
    pub fn data_type(&self) -> Option<&str> {
        self.0.get(b'd')
    }

    /// If the error is with a specific constraint, the name of the constraint.
    pub fn constraint(&self) -> Option<&str> {
        self.0.get(b'n')
    }

    /// The file name of the source-code location where this error was reported.
    pub fn file(&self) -> Option<&str> {
        self.0.get(b'F')
    }

    /// The line number of the source-code location where this error was reported.
    pub fn line(&self) -> Option<usize> {
        self.0.get_raw(b'L').and_then(atoi)
    }

    /// The name of the source-code routine reporting this error.
    pub fn routine(&self) -> Option<&str> {
        self.0.get(b'R')
    }

    /// Returns the error kind.
    pub fn kind(&self) -> ErrorKind {
        match self.code() {
            error_codes::UNIQUE_VIOLATION => ErrorKind::UniqueViolation,
            error_codes::FOREIGN_KEY_VIOLATION => ErrorKind::ForeignKeyViolation,
            error_codes::NOT_NULL_VIOLATION => ErrorKind::NotNullViolation,
            error_codes::CHECK_VIOLATION => ErrorKind::CheckViolation,
            error_codes::EXCLUSION_VIOLATION => ErrorKind::ExclusionViolation,
            _ => ErrorKind::Other,
        }
    }

    /// Returns whether the error kind is a violation of a unique/primary key constraint.
    pub fn is_unique_violation(&self) -> bool {
        matches!(self.kind(), ErrorKind::UniqueViolation)
    }

    /// Returns whether the error kind is a violation of a foreign key.
    pub fn is_foreign_key_violation(&self) -> bool {
        matches!(self.kind(), ErrorKind::ForeignKeyViolation)
    }

    /// Returns whether the error kind is a violation of a check.
    pub fn is_check_violation(&self) -> bool {
        matches!(self.kind(), ErrorKind::CheckViolation)
    }

    /// Returns the code (SQLSTATE) as an owned `Cow<str>`.
    #[doc(hidden)]
    pub fn code_cow(&self) -> Option<Cow<'_, str>> {
        Some(Cow::Borrowed(self.code()))
    }
}

#[derive(Debug, Eq, PartialEq)]
pub enum ErrorPosition<'a> {
    /// A position (in characters) into the original query.
    Original(usize),

    /// A position into the internally-generated query.
    Internal {
        /// The position in characters.
        position: usize,

        /// The text of a failed internally-generated command.
        query: &'a str,
    },
}

impl Debug for DatabaseError {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.debug_struct("DatabaseError")
            .field("severity", &self.severity())
            .field("code", &self.code())
            .field("message", &self.message())
            .field("detail", &self.detail())
            .field("hint", &self.hint())
            .field("position", &self.position())
            .field("where", &self.r#where())
            .field("schema", &self.schema())
            .field("table", &self.table())
            .field("column", &self.column())
            .field("data_type", &self.data_type())
            .field("constraint", &self.constraint())
            .field("file", &self.file())
            .field("line", &self.line())
            .field("routine", &self.routine())
            .finish()
    }
}

impl Display for DatabaseError {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.write_str(self.message())?;
        if let Some(line) = self.line() {
            write!(f, " at line {line}")?;
        }
        Ok(())
    }
}

impl StdError for DatabaseError {}

// ErrorResponse is the same structure as NoticeResponse but a different format code.
impl BackendMessage for DatabaseError {
    const FORMAT: BackendMessageFormat = BackendMessageFormat::ErrorResponse;

    #[inline(always)]
    fn decode_body(buf: Bytes) -> std::result::Result<Self, Error> {
        Ok(Self(Notice::decode_body(buf)?))
    }
}

/// For reference: <https://www.postgresql.org/docs/current/errcodes-appendix.html>
pub(crate) mod error_codes {
    /// Caused when a unique or primary key is violated.
    pub const UNIQUE_VIOLATION: &str = "23505";
    /// Caused when a foreign key is violated.
    pub const FOREIGN_KEY_VIOLATION: &str = "23503";
    /// Caused when a column marked as NOT NULL received a null value.
    pub const NOT_NULL_VIOLATION: &str = "23502";
    /// Caused when a check constraint is violated.
    pub const CHECK_VIOLATION: &str = "23514";
    /// Caused when a exclude constraint is violated.
    pub const EXCLUSION_VIOLATION: &str = "23P01";
}
