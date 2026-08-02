use crate::column::ColumnIndex;
use crate::error::{mismatched_types, Error};
use crate::message::DataRow;
use crate::decode::Decode;
use crate::codec::Type;
use crate::statement::StatementMetadata;
use crate::value::ValueFormat;
use crate::{Column, ValueRef};
use std::sync::Arc;

/// A single row from a PostgreSQL query result.
pub struct Row {
    pub(crate) data: DataRow,
    pub(crate) format: ValueFormat,
    pub(crate) metadata: Arc<StatementMetadata>,
}

impl Row {
    /// Returns `true` if this row has no columns.
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Returns the number of columns in this row.
    #[inline]
    pub fn len(&self) -> usize {
        self.columns().len()
    }

    /// Gets all columns in this row.
    pub fn columns(&self) -> &[Column] {
        &self.metadata.columns
    }

    /// Gets the column information at `index`.
    ///
    /// # Panics
    ///
    /// Panics if `index` is out of bounds. See [`try_column`](Self::try_column)
    /// for a non-panicking version.
    pub fn column<I>(&self, index: I) -> &Column
    where
        I: ColumnIndex<Self>,
    {
        self.try_column(index).unwrap()
    }

    /// Gets the column information at `index`, or [`Error::ColumnIndexOutOfBounds`]
    /// / [`Error::ColumnNotFound`] on failure.
    pub fn try_column<I>(&self, index: I) -> Result<&Column, Error>
    where
        I: ColumnIndex<Self>,
    {
        Ok(&self.columns()[index.index(self)?])
    }

    /// Index into the row and decode a single value.
    ///
    /// # Panics
    ///
    /// Panics if the column does not exist or its value cannot be decoded into the
    /// requested type. See [`try_get`](Self::try_get) for a non-panicking version.
    #[inline]
    #[track_caller]
    pub fn get<'r, T, I>(&'r self, index: I) -> T
    where
        I: ColumnIndex<Self>,
        T: Decode<'r> + Type,
    {
        self.try_get::<T, I>(index).unwrap()
    }

    /// Index into the row and decode a single value without a type-compatibility check.
    ///
    /// # Panics
    ///
    /// Panics if the column does not exist or its value cannot be decoded into the
    /// requested type. See [`try_get_unchecked`](Self::try_get_unchecked) for a
    /// non-panicking version.
    #[inline]
    pub fn get_unchecked<'r, T, I>(&'r self, index: I) -> T
    where
        I: ColumnIndex<Self>,
        T: Decode<'r>,
    {
        self.try_get_unchecked::<T, I>(index).unwrap()
    }

    /// Index into the row and decode a single value.
    pub fn try_get<'r, T, I>(&'r self, index: I) -> Result<T, Error>
    where
        I: ColumnIndex<Self>,
        T: Decode<'r> + Type,
    {
        let value = self.try_get_raw(&index)?;

        if !value.is_null() {
            let ty = value.type_info();

            if !ty.is_null() && !T::compatible(&ty) {
                return Err(Error::ColumnDecode {
                    index: format!("{index:?}"),
                    source: mismatched_types::<T>(&ty),
                });
            }
        }

        T::decode(value).map_err(|source| Error::ColumnDecode {
            index: format!("{index:?}"),
            source,
        })
    }

    /// Index into the row and decode a single value without a type-compatibility check.
    #[inline]
    pub fn try_get_unchecked<'r, T, I>(&'r self, index: I) -> Result<T, Error>
    where
        I: ColumnIndex<Self>,
        T: Decode<'r>,
    {
        let value = self.try_get_raw(&index)?;

        T::decode(value).map_err(|source| Error::ColumnDecode {
            index: format!("{index:?}"),
            source,
        })
    }

    /// Index into the row and return the raw (undecoded) [`ValueRef`].
    pub fn try_get_raw<I>(&self, index: I) -> Result<ValueRef<'_>, Error>
    where
        I: ColumnIndex<Self>,
    {
        let index = index.index(self)?;
        let column = &self.metadata.columns[index];
        let value = self.data.get(index);

        Ok(ValueRef {
            format: self.format,
            row: Some(&self.data.storage),
            type_info: column.type_info.clone(),
            value,
        })
    }
}

impl ColumnIndex<Row> for &'_ str {
    fn index(&self, row: &Row) -> Result<usize, Error> {
        row.metadata
            .column_names
            .get(*self)
            .ok_or_else(|| Error::ColumnNotFound((*self).into()))
            .copied()
    }
}

impl std::fmt::Debug for Row {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Row ")?;

        let mut debug_map = f.debug_map();

        for column in self.columns().iter() {
            match self.try_get_raw(column.ordinal()) {
                Ok(value) => {
                    debug_map.entry(
                        &column.name(),
                        &fmt_value_debug(&value.to_owned()),
                    );
                }
                Err(error) => {
                    debug_map.entry(&column.name(), &format!("decode error: {error:?}"));
                }
            }
        }

        debug_map.finish()
    }
}

/// `Debug`-shim holding a `&Value` and a formatter fn-pointer. Constructed by
/// [`fmt_value_debug`]; consumed via its `Debug` impl.
struct FmtValue<'v> {
    value: &'v crate::Value,
    fmt: fn(&'v crate::Value, &mut std::fmt::Formatter<'_>) -> std::fmt::Result,
}

impl<'v> FmtValue<'v> {
    /// Decode the value as `T` and forward to `T`'s `Debug` impl.
    fn debug<T>(value: &'v crate::Value) -> Self
    where
        T: Decode<'v> + std::fmt::Debug + std::any::Any,
    {
        Self {
            value,
            fmt: |value, f| {
                let info = value.type_info();
                match T::decode(value.as_ref()) {
                    Ok(value) => std::fmt::Debug::fmt(&value, f),
                    Err(e) => {
                        if e.is::<crate::error::UnexpectedNullError>() {
                            f.write_str("NULL")
                        } else {
                            f.write_fmt(format_args!(
                                "(error decoding SQL type {} as {}: {e:?})",
                                info.name(),
                                std::any::type_name::<T>()
                            ))
                        }
                    }
                }
            },
        }
    }

    /// Prints the SQL type name without decoding the payload.
    fn unknown(value: &'v crate::Value) -> Self {
        Self {
            value,
            fmt: |value, f| {
                f.write_fmt(format_args!(
                    "(unknown SQL type {})",
                    value.type_info().name()
                ))
            },
        }
    }
}

impl std::fmt::Debug for FmtValue<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        (self.fmt)(self.value, f)
    }
}

/// Pick a `FmtValue` decoder appropriate for the value's SQL type.
///
/// Common built-in types (`bool`, integers, floats, `String`, `Vec<u8>`) are decoded
/// and pretty-printed. Anything else falls through to a printer that renders the SQL
/// type name without decoding the payload.
fn fmt_value_debug(value: &crate::Value) -> FmtValue<'_> {
    let info = value.type_info();

    if <bool as Type>::compatible(&info) {
        return FmtValue::debug::<bool>(value);
    }
    if <i16 as Type>::compatible(&info) {
        return FmtValue::debug::<i16>(value);
    }
    if <i32 as Type>::compatible(&info) {
        return FmtValue::debug::<i32>(value);
    }
    if <i64 as Type>::compatible(&info) {
        return FmtValue::debug::<i64>(value);
    }
    if <f32 as Type>::compatible(&info) {
        return FmtValue::debug::<f32>(value);
    }
    if <f64 as Type>::compatible(&info) {
        return FmtValue::debug::<f64>(value);
    }
    if <String as Type>::compatible(&info) {
        return FmtValue::debug::<String>(value);
    }
    if <Vec<u8> as Type>::compatible(&info) {
        return FmtValue::debug::<Vec<u8>>(value);
    }

    FmtValue::unknown(value)
}
