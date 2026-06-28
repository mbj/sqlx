use crate::decode::Decode;
use crate::error::{mismatched_types, BoxDynError, Error, UnexpectedNullError};
use crate::bytes::{Buf, Bytes};
use crate::codec::Type;
use crate::TypeInfo;
use std::borrow::Cow;
use std::str::from_utf8;

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
#[repr(u8)]
pub enum ValueFormat {
    Text = 0,
    Binary = 1,
}

/// A reference to a single value from the database.
#[derive(Clone)]
pub struct ValueRef<'r> {
    pub(crate) value: Option<&'r [u8]>,
    pub(crate) row: Option<&'r Bytes>,
    pub(crate) type_info: TypeInfo,
    pub(crate) format: ValueFormat,
}

/// An owned value from the database.
#[derive(Clone)]
pub struct Value {
    pub(crate) value: Option<Bytes>,
    pub(crate) type_info: TypeInfo,
    pub(crate) format: ValueFormat,
}

impl<'r> ValueRef<'r> {
    pub(crate) fn get(
        buf: &mut &'r [u8],
        format: ValueFormat,
        ty: TypeInfo,
    ) -> Result<Self, String> {
        let element_len = buf.get_i32();

        let element_val = if element_len == -1 {
            None
        } else {
            let element_len: usize = element_len
                .try_into()
                .map_err(|_| format!("overflow converting element_len ({element_len}) to usize"))?;

            let val = &buf[..element_len];
            buf.advance(element_len);
            Some(val)
        };

        Ok(ValueRef {
            value: element_val,
            row: None,
            type_info: ty,
            format,
        })
    }

    pub fn format(&self) -> ValueFormat {
        self.format
    }

    pub fn as_bytes(&self) -> Result<&'r [u8], BoxDynError> {
        match &self.value {
            Some(v) => Ok(v),
            None => Err(UnexpectedNullError.into()),
        }
    }

    pub fn as_str(&self) -> Result<&'r str, BoxDynError> {
        Ok(from_utf8(self.as_bytes()?)?)
    }

    /// Creates an owned value from this value reference.
    ///
    /// This is a reference increment in PostgreSQL and thus is `O(1)`.
    pub fn to_owned(&self) -> Value {
        let value = match (self.row, self.value) {
            (Some(row), Some(value)) => Some(row.slice_ref(value)),
            (None, Some(value)) => Some(Bytes::copy_from_slice(value)),
            _ => None,
        };

        Value {
            value,
            format: self.format,
            type_info: self.type_info.clone(),
        }
    }

    /// Get the type information for this value.
    pub fn type_info(&self) -> Cow<'_, TypeInfo> {
        Cow::Borrowed(&self.type_info)
    }

    /// Returns `true` if the SQL value is `NULL`.
    pub fn is_null(&self) -> bool {
        self.value.is_none()
    }
}

impl Value {
    /// Get this value as a reference.
    #[inline]
    pub fn as_ref(&self) -> ValueRef<'_> {
        ValueRef {
            value: self.value.as_deref(),
            row: None,
            type_info: self.type_info.clone(),
            format: self.format,
        }
    }

    /// Get the type information for this value.
    pub fn type_info(&self) -> Cow<'_, TypeInfo> {
        Cow::Borrowed(&self.type_info)
    }

    /// Returns `true` if the SQL value is `NULL`.
    pub fn is_null(&self) -> bool {
        self.value.is_none()
    }

    /// Decode this single value into the requested type.
    ///
    /// # Panics
    /// Panics if the value cannot be decoded into the requested type.
    #[inline]
    pub fn decode<'r, T>(&'r self) -> T
    where
        T: Decode<'r> + Type,
    {
        self.try_decode::<T>().unwrap()
    }

    /// Decode this single value into the requested type without type-compatibility checks.
    #[inline]
    pub fn decode_unchecked<'r, T>(&'r self) -> T
    where
        T: Decode<'r>,
    {
        self.try_decode_unchecked::<T>().unwrap()
    }

    /// Decode this single value into the requested type.
    #[inline]
    pub fn try_decode<'r, T>(&'r self) -> Result<T, Error>
    where
        T: Decode<'r> + Type,
    {
        if !self.is_null() {
            let ty = self.type_info();

            if !ty.is_null() && !T::compatible(&ty) {
                return Err(Error::Decode(mismatched_types::<T>(&ty)));
            }
        }

        self.try_decode_unchecked()
    }

    /// Decode this single value into the requested type without type-compatibility checks.
    #[inline]
    pub fn try_decode_unchecked<'r, T>(&'r self) -> Result<T, Error>
    where
        T: Decode<'r>,
    {
        T::decode(self.as_ref()).map_err(Error::Decode)
    }
}
