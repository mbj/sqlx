use std::fmt::Display;
use std::io::Write;
use std::ops::{Deref, DerefMut};
use std::str::FromStr;

use crate::decode::Decode;
use crate::encode::{Encode, IsNull};
use crate::error::BoxDynError;
use crate::codec::Type;
use crate::{ArgumentBuffer, TypeInfo, ValueRef};

/// Map a SQL text value to/from a Rust type using [`Display`] and [`FromStr`].
///
/// This can be useful for types that do not have a direct SQL equivalent, or are simply not
/// supported by SQLx for one reason or another.
///
/// For strongly typed databases like Postgres, this will report the value's type as `TEXT`.
/// Explicit conversion may be necessary on the SQL side depending on the desired type.
///
/// ### Panics
///
/// You should only use this adapter with `Display` implementations that are infallible,
/// otherwise you may encounter panics when attempting to bind a value.
#[derive(Debug, Default, Copy, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct Text<T>(pub T);

impl<T> Text<T> {
    /// Extract the inner value.
    pub fn into_inner(self) -> T {
        self.0
    }
}

impl<T> Deref for Text<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<T> DerefMut for Text<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl<T> Type for Text<T> {
    fn type_info() -> TypeInfo {
        <String as Type>::type_info()
    }

    fn compatible(ty: &TypeInfo) -> bool {
        <String as Type>::compatible(ty)
    }
}

impl<T> Encode<'_> for Text<T>
where
    T: Display,
{
    fn encode_by_ref(&self, buf: &mut ArgumentBuffer) -> Result<IsNull, BoxDynError> {
        write!(**buf, "{}", self.0)?;
        Ok(IsNull::No)
    }
}

impl<'r, T> Decode<'r> for Text<T>
where
    T: FromStr,
    BoxDynError: From<<T as FromStr>::Err>,
{
    fn decode(value: ValueRef<'r>) -> Result<Self, BoxDynError> {
        let s: &str = Decode::decode(value)?;
        Ok(Self(s.parse()?))
    }
}
