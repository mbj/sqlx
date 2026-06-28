use std::ops::{Deref, DerefMut};

use serde::{Deserialize, Serialize};
pub use serde_json::value::RawValue as JsonRawValue;
pub use serde_json::Value as JsonValue;

use crate::decode::Decode;
use crate::encode::{Encode, IsNull};
use crate::error::BoxDynError;
use crate::codec::{array_compatible, Type};
use crate::{ArgumentBuffer, HasArrayType, TypeInfo, ValueFormat, ValueRef};

/// Json for json and jsonb fields.
///
/// Will attempt to cast to type passed in as the generic.
#[derive(
    Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Default, Serialize, Deserialize,
)]
#[serde(transparent)]
pub struct Json<T: ?Sized>(pub T);

impl<T> Json<T> {
    /// Extract the inner value.
    pub fn into_inner(self) -> T {
        self.0
    }
}

impl<T> From<T> for Json<T> {
    fn from(value: T) -> Self {
        Self(value)
    }
}

impl<T> Deref for Json<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<T> DerefMut for Json<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl<T> AsRef<T> for Json<T> {
    fn as_ref(&self) -> &T {
        &self.0
    }
}

impl<T> AsMut<T> for Json<T> {
    fn as_mut(&mut self) -> &mut T {
        &mut self.0
    }
}

// UNSTABLE: for driver use only!
#[doc(hidden)]
impl<T: Serialize> Json<T> {
    pub fn encode_to_string(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string(self)
    }

    pub fn encode_to(&self, buf: &mut Vec<u8>) -> Result<(), serde_json::Error> {
        serde_json::to_writer(buf, self)
    }
}

// UNSTABLE: for driver use only!
#[doc(hidden)]
impl<'a, T: 'a> Json<T>
where
    T: Deserialize<'a>,
{
    pub fn decode_from_string(s: &'a str) -> Result<Self, BoxDynError> {
        serde_json::from_str(s).map_err(Into::into)
    }

    pub fn decode_from_bytes(bytes: &'a [u8]) -> Result<Self, BoxDynError> {
        serde_json::from_slice(bytes).map_err(Into::into)
    }
}

impl Type for JsonValue
where
    Json<Self>: Type,
{
    fn type_info() -> TypeInfo {
        <Json<Self> as Type>::type_info()
    }

    fn compatible(ty: &TypeInfo) -> bool {
        <Json<Self> as Type>::compatible(ty)
    }
}

impl<'q> Encode<'q> for JsonValue
where
    for<'a> Json<&'a Self>: Encode<'q>,
{
    fn encode_by_ref(&self, buf: &mut ArgumentBuffer) -> Result<IsNull, BoxDynError> {
        <Json<&Self> as Encode<'q>>::encode(Json(self), buf)
    }
}

impl<'r> Decode<'r> for JsonValue
where
    Json<Self>: Decode<'r>,
{
    fn decode(value: ValueRef<'r>) -> Result<Self, BoxDynError> {
        <Json<Self> as Decode>::decode(value).map(|item| item.0)
    }
}

impl Type for JsonRawValue
where
    for<'a> Json<&'a Self>: Type,
{
    fn type_info() -> TypeInfo {
        <Json<&Self> as Type>::type_info()
    }

    fn compatible(ty: &TypeInfo) -> bool {
        <Json<&Self> as Type>::compatible(ty)
    }
}

impl<'q> Encode<'q> for JsonRawValue
where
    for<'a> Json<&'a Self>: Encode<'q>,
{
    fn encode_by_ref(&self, buf: &mut ArgumentBuffer) -> Result<IsNull, BoxDynError> {
        <Json<&Self> as Encode<'q>>::encode(Json(self), buf)
    }
}

impl<'q> Encode<'q> for &'q JsonRawValue
where
    for<'a> Json<&'a Self>: Encode<'q>,
{
    fn encode_by_ref(&self, buf: &mut ArgumentBuffer) -> Result<IsNull, BoxDynError> {
        <Json<&Self> as Encode<'q>>::encode(Json(self), buf)
    }
}

impl<'q> Encode<'q> for Box<JsonRawValue>
where
    for<'a> Json<&'a Self>: Encode<'q>,
{
    fn encode_by_ref(&self, buf: &mut ArgumentBuffer) -> Result<IsNull, BoxDynError> {
        <Json<&Self> as Encode<'q>>::encode(Json(self), buf)
    }
}

impl<'r> Decode<'r> for &'r JsonRawValue
where
    Json<Self>: Decode<'r>,
{
    fn decode(value: ValueRef<'r>) -> Result<Self, BoxDynError> {
        <Json<Self> as Decode>::decode(value).map(|item| item.0)
    }
}

impl<'r> Decode<'r> for Box<JsonRawValue>
where
    Json<Self>: Decode<'r>,
{
    fn decode(value: ValueRef<'r>) -> Result<Self, BoxDynError> {
        <Json<Self> as Decode>::decode(value).map(|item| item.0)
    }
}

// <https://www.postgresql.org/docs/12/datatype-json.html>

// In general, most applications should prefer to store JSON data as jsonb,
// unless there are quite specialized needs, such as legacy assumptions
// about ordering of object keys.

impl<T> Type for Json<T> {
    fn type_info() -> TypeInfo {
        TypeInfo::JSONB
    }

    fn compatible(ty: &TypeInfo) -> bool {
        *ty == TypeInfo::JSON || *ty == TypeInfo::JSONB
    }
}

impl<T> HasArrayType for Json<T> {
    fn array_type_info() -> TypeInfo {
        TypeInfo::JSONB_ARRAY
    }

    fn array_compatible(ty: &TypeInfo) -> bool {
        array_compatible::<Json<T>>(ty)
    }
}

impl HasArrayType for JsonValue {
    fn array_type_info() -> TypeInfo {
        TypeInfo::JSONB_ARRAY
    }

    fn array_compatible(ty: &TypeInfo) -> bool {
        array_compatible::<JsonValue>(ty)
    }
}

impl HasArrayType for JsonRawValue {
    fn array_type_info() -> TypeInfo {
        TypeInfo::JSONB_ARRAY
    }

    fn array_compatible(ty: &TypeInfo) -> bool {
        array_compatible::<JsonRawValue>(ty)
    }
}

impl<T> Encode<'_> for Json<T>
where
    T: Serialize,
{
    fn encode_by_ref(&self, buf: &mut ArgumentBuffer) -> Result<IsNull, BoxDynError> {
        // we have a tiny amount of dynamic behavior depending if we are resolved to be JSON
        // instead of JSONB
        buf.patch_with(|buf, ty: &TypeInfo| {
            if *ty == TypeInfo::JSON || *ty == TypeInfo::JSON_ARRAY {
                buf[0] = b' ';
            }
        });

        // JSONB version (as of 2020-03-20)
        buf.push(1);

        // the JSON data written to the buffer is the same regardless of parameter type
        serde_json::to_writer(&mut **buf, &self.0)?;

        Ok(IsNull::No)
    }
}

impl<'r, T: 'r> Decode<'r> for Json<T>
where
    T: Deserialize<'r>,
{
    fn decode(value: ValueRef<'r>) -> Result<Self, BoxDynError> {
        let mut buf = value.as_bytes()?;

        if value.format() == ValueFormat::Binary && value.type_info == TypeInfo::JSONB {
            // Check JSONB version byte - PostgreSQL currently only supports version 1
            if buf[0] != 1 {
                return Err(
                    format!("unsupported JSONB format version {} (expected 1)", buf[0]).into(),
                );
            }

            buf = &buf[1..];
        }

        serde_json::from_slice(buf).map(Json).map_err(Into::into)
    }
}
