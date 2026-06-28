use byteorder::{BigEndian, ByteOrder};

use crate::decode::Decode;
use crate::encode::{Encode, IsNull};
use crate::error::BoxDynError;
use crate::codec::Type;
use crate::{ArgumentBuffer, HasArrayType, TypeInfo, ValueFormat, ValueRef};

impl Type for f32 {
    fn type_info() -> TypeInfo {
        TypeInfo::FLOAT4
    }
}

impl HasArrayType for f32 {
    fn array_type_info() -> TypeInfo {
        TypeInfo::FLOAT4_ARRAY
    }
}

impl Encode<'_> for f32 {
    fn encode_by_ref(&self, buf: &mut ArgumentBuffer) -> Result<IsNull, BoxDynError> {
        buf.extend(&self.to_be_bytes());

        Ok(IsNull::No)
    }
}

impl Decode<'_> for f32 {
    fn decode(value: ValueRef<'_>) -> Result<Self, BoxDynError> {
        Ok(match value.format() {
            ValueFormat::Binary => BigEndian::read_f32(value.as_bytes()?),
            ValueFormat::Text => value.as_str()?.parse()?,
        })
    }
}

impl Type for f64 {
    fn type_info() -> TypeInfo {
        TypeInfo::FLOAT8
    }
}

impl HasArrayType for f64 {
    fn array_type_info() -> TypeInfo {
        TypeInfo::FLOAT8_ARRAY
    }
}

impl Encode<'_> for f64 {
    fn encode_by_ref(&self, buf: &mut ArgumentBuffer) -> Result<IsNull, BoxDynError> {
        buf.extend(&self.to_be_bytes());

        Ok(IsNull::No)
    }
}

impl Decode<'_> for f64 {
    fn decode(value: ValueRef<'_>) -> Result<Self, BoxDynError> {
        Ok(match value.format() {
            ValueFormat::Binary => BigEndian::read_f64(value.as_bytes()?),
            ValueFormat::Text => value.as_str()?.parse()?,
        })
    }
}
