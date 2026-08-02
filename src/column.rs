use crate::error::Error;
use crate::ustr::UStr;
use crate::TypeInfo;
use std::fmt::Debug;
use std::sync::Arc;

/// A column of a Postgres query result or prepared statement.
#[derive(Debug, Clone)]
pub struct Column {
    pub(crate) ordinal: usize,
    pub(crate) name: UStr,
    pub(crate) type_info: TypeInfo,
    pub(crate) origin: ColumnOrigin,
    pub(crate) relation_id: Option<crate::codec::Oid>,
    pub(crate) relation_attribute_no: Option<i16>,
}

impl Column {
    /// Gets the column ordinal.
    ///
    /// This can be used to unambiguously refer to this column within a row in case more than
    /// one column has the same name.
    pub fn ordinal(&self) -> usize {
        self.ordinal
    }

    /// Gets the column name or alias.
    ///
    /// The column name is unreliable (and can change between database minor versions) if this
    /// column is an expression that has not been aliased.
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Gets the type information for the column.
    pub fn type_info(&self) -> &TypeInfo {
        &self.type_info
    }

    /// If this column comes from a table, return the table and original column name.
    ///
    /// Returns [`ColumnOrigin::Expression`] if the column is the result of an expression
    /// or else the source table could not be determined.
    ///
    /// Returns [`ColumnOrigin::Unknown`] if the driver does not have that information.
    pub fn origin(&self) -> ColumnOrigin {
        self.origin.clone()
    }

    /// Returns the OID of the table this column is from, if applicable.
    ///
    /// This will be `None` if the column is the result of an expression.
    ///
    /// Corresponds to column `attrelid` of the `pg_catalog.pg_attribute` table:
    /// <https://www.postgresql.org/docs/current/catalog-pg-attribute.html>
    pub fn relation_id(&self) -> Option<crate::codec::Oid> {
        self.relation_id
    }

    /// Returns the 1-based index of this column in its parent table, if applicable.
    ///
    /// This will be `None` if the column is the result of an expression.
    ///
    /// Corresponds to column `attnum` of the `pg_catalog.pg_attribute` table:
    /// <https://www.postgresql.org/docs/current/catalog-pg-attribute.html>
    pub fn relation_attribute_no(&self) -> Option<i16> {
        self.relation_attribute_no
    }
}

/// A [`Column`] that originates from a table.
#[derive(Debug, Clone)]
pub struct TableColumn {
    /// The name of the table (optionally schema-qualified) that the column comes from.
    pub table: Arc<str>,
    /// The original name of the column.
    pub name: Arc<str>,
}

/// The possible statuses for our knowledge of the origin of a [`Column`].
#[derive(Debug, Clone, Default)]
pub enum ColumnOrigin {
    /// The column is known to originate from a table.
    ///
    /// Included is the table name and original column name.
    Table(TableColumn),
    /// The column originates from an expression, or else its origin could not be determined.
    Expression,
    /// The driver does not know the column origin at this time.
    ///
    /// This may happen if:
    /// * The connection is in the middle of executing a query,
    ///   and cannot query the catalog to fetch this information.
    /// * The connection does not have access to the database catalog.
    #[default]
    Unknown,
}

impl ColumnOrigin {
    /// Returns the true column origin, if known.
    pub fn table_column(&self) -> Option<&TableColumn> {
        if let Self::Table(table_column) = self {
            Some(table_column)
        } else {
            None
        }
    }
}

/// A type that can be used to index into a [`Row`](crate::Row) or
/// [`Statement`](crate::Statement).
///
/// The `get` and `try_get` methods of `Row` accept any type that implements `ColumnIndex`.
/// This trait is implemented for strings which are used to look up a column by name, and for
/// `usize` which is used as a positional index.
pub trait ColumnIndex<T: ?Sized>: Debug {
    /// Returns a valid positional index into the row or statement, or a `ColumnIndexOutOfBounds`
    /// / `ColumnNotFound` error.
    fn index(&self, container: &T) -> Result<usize, Error>;
}

impl<T: ?Sized, I: ColumnIndex<T> + ?Sized> ColumnIndex<T> for &'_ I {
    #[inline]
    fn index(&self, row: &T) -> Result<usize, Error> {
        (**self).index(row)
    }
}

#[macro_export]
macro_rules! impl_column_index_for_row {
    ($R:ident) => {
        impl $crate::column::ColumnIndex<$R> for usize {
            fn index(&self, row: &$R) -> Result<usize, $crate::error::Error> {
                let len = row.len();

                if *self >= len {
                    return Err($crate::error::Error::ColumnIndexOutOfBounds { len, index: *self });
                }

                Ok(*self)
            }
        }
    };
}

#[macro_export]
macro_rules! impl_column_index_for_statement {
    ($S:ident) => {
        impl $crate::column::ColumnIndex<$S> for usize {
            fn index(&self, statement: &$S) -> Result<usize, $crate::error::Error> {
                let len = statement.columns().len();

                if *self >= len {
                    return Err($crate::error::Error::ColumnIndexOutOfBounds { len, index: *self });
                }

                Ok(*self)
            }
        }
    };
}
