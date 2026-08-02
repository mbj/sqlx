use std::borrow::Cow;
use std::rc::Rc;
use std::sync::Arc;

use crate::decode::Decode;
use crate::encode::{Encode, IsNull};
use crate::error::BoxDynError;
use crate::codec::Type;
use crate::{ArgumentBuffer, HasArrayType, TypeInfo, ValueFormat, ValueRef};

impl HasArrayType for u8 {
    fn array_type_info() -> TypeInfo {
        TypeInfo::BYTEA
    }
}

impl HasArrayType for &'_ [u8] {
    fn array_type_info() -> TypeInfo {
        TypeInfo::BYTEA_ARRAY
    }
}

impl HasArrayType for Box<[u8]> {
    fn array_type_info() -> TypeInfo {
        <[&[u8]] as Type>::type_info()
    }
}

impl HasArrayType for Vec<u8> {
    fn array_type_info() -> TypeInfo {
        <[&[u8]] as Type>::type_info()
    }
}

impl<const N: usize> HasArrayType for [u8; N] {
    fn array_type_info() -> TypeInfo {
        <[&[u8]] as Type>::type_info()
    }
}

impl Encode<'_> for &'_ [u8] {
    fn encode_by_ref(&self, buf: &mut ArgumentBuffer) -> Result<IsNull, BoxDynError> {
        buf.extend_from_slice(self);

        Ok(IsNull::No)
    }
}

impl Encode<'_> for Vec<u8> {
    fn encode_by_ref(&self, buf: &mut ArgumentBuffer) -> Result<IsNull, BoxDynError> {
        <&[u8] as Encode>::encode(self, buf)
    }
}

impl<const N: usize> Encode<'_> for [u8; N] {
    fn encode_by_ref(&self, buf: &mut ArgumentBuffer) -> Result<IsNull, BoxDynError> {
        <&[u8] as Encode>::encode(self.as_slice(), buf)
    }
}

impl<'r> Decode<'r> for &'r [u8] {
    fn decode(value: ValueRef<'r>) -> Result<Self, BoxDynError> {
        match value.format() {
            ValueFormat::Binary => value.as_bytes(),
            ValueFormat::Text => {
                Err("unsupported decode to `&[u8]` of BYTEA in a simple query; use a prepared query or decode to `Vec<u8>`".into())
            }
        }
    }
}

fn text_hex_decode_input(value: ValueRef<'_>) -> Result<&[u8], BoxDynError> {
    // BYTEA is formatted as \x followed by hex characters
    value
        .as_bytes()?
        .strip_prefix(b"\\x")
        .ok_or("text does not start with \\x")
        .map_err(Into::into)
}

impl Decode<'_> for Vec<u8> {
    fn decode(value: ValueRef<'_>) -> Result<Self, BoxDynError> {
        Ok(match value.format() {
            ValueFormat::Binary => value.as_bytes()?.to_owned(),
            ValueFormat::Text => hex::decode(text_hex_decode_input(value)?)?,
        })
    }
}

impl<const N: usize> Decode<'_> for [u8; N] {
    fn decode(value: ValueRef<'_>) -> Result<Self, BoxDynError> {
        let mut bytes = [0u8; N];
        match value.format() {
            ValueFormat::Binary => {
                bytes = value.as_bytes()?.try_into()?;
            }
            ValueFormat::Text => hex::decode_to_slice(text_hex_decode_input(value)?, &mut bytes)?,
        };
        Ok(bytes)
    }
}

crate::forward_encode_impl!(Arc<[u8]>, &[u8]);
crate::forward_encode_impl!(Rc<[u8]>, &[u8]);
crate::forward_encode_impl!(Box<[u8]>, &[u8]);
crate::forward_encode_impl!(Cow<'_, [u8]>, &[u8]);
