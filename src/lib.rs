#![doc = include_str!("lib.md")]
#![recursion_limit = "512"]

mod ustr;

#[macro_use]
mod async_stream;

#[macro_use]
pub mod arguments;

pub mod connection;

#[macro_use]
pub mod encode;

#[macro_use]
pub mod decode;

#[macro_use]
pub mod codec;

#[macro_use]
pub mod query;

pub mod statement_cache;
pub mod executor;
pub mod from_row;
pub mod io;
pub mod net;
pub mod query_as;
pub mod query_scalar;
pub mod sql_str;
pub mod raw_sql;

mod bind_iter;
pub mod column;
pub mod error;
mod message;
mod options;
mod query_result;
mod row;
mod statement;
mod value;

pub use crate::error::{Error, Result};

pub use either::Either;
pub use std::collections::{hash_map, HashMap};
pub use percent_encoding;
pub use bytes;
pub use url::{self, Url};

pub use arguments::{ArgumentBuffer, Arguments};
pub use bind_iter::BindIterExt;
pub use column::Column;
pub use connection::Connection;
pub use error::{DatabaseError, ErrorPosition};
pub use message::{Severity, TransactionStatus};
pub use options::{ConnectOptions, SslMode};
pub use query_result::QueryResult;
pub use row::Row;
pub use statement::Statement;
pub use codec::{HasArrayType, TypeInfo, TypeKind};
pub use value::{Value, ValueFormat, ValueRef};

/// Block the current thread on `f` using a fresh single-threaded Tokio runtime.
///
/// Used by `#[sqlx::test]`-emitted code to run its async body from a synchronous
/// `#[test]` wrapper.
#[doc(hidden)]
pub fn test_block_on<F: std::future::Future>(f: F) -> F::Output {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .expect("failed to start Tokio runtime")
        .block_on(f)
}


crate::impl_into_arguments_for_arguments!(Arguments);
crate::impl_column_index_for_row!(Row);
crate::impl_column_index_for_statement!(Statement);
crate::impl_encode_for_option!();

/// Convenience re-export of common traits.
pub mod prelude {
    pub use super::Decode;
    pub use super::Encode;
    pub use super::FromRow;
    pub use super::IntoArguments;
    pub use super::Type;
}

pub use crate::arguments::IntoArguments;
pub use crate::column::{ColumnIndex, ColumnOrigin};
pub use crate::decode::Decode;
pub use crate::encode::Encode;
pub use crate::executor::Execute;
pub use crate::from_row::FromRow;
pub use crate::query::{query, query_with};
pub use crate::query_as::{query_as, query_as_with};
pub use crate::query_scalar::{query_scalar, query_scalar_with};
pub use crate::raw_sql::{raw_sql, RawSql};
pub use crate::sql_str::{AssertSqlSafe, SqlSafeStr, SqlStr};
pub use crate::codec::Type;
