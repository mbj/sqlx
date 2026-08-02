use crate::decode::Decode;
use crate::encode::{Encode, IsNull};
use crate::error::BoxDynError;
use crate::codec::Type;
use crate::{ArgumentBuffer, HasArrayType, TypeInfo, ValueFormat, ValueRef};

impl Type for bool {
    fn type_info() -> TypeInfo {
        TypeInfo::BOOL
    }
}

impl HasArrayType for bool {
    fn array_type_info() -> TypeInfo {
        TypeInfo::BOOL_ARRAY
    }
}

impl Encode<'_> for bool {
    fn encode_by_ref(&self, buf: &mut ArgumentBuffer) -> Result<IsNull, BoxDynError> {
        buf.push(*self as u8);

        Ok(IsNull::No)
    }
}

impl Decode<'_> for bool {
    fn decode(value: ValueRef<'_>) -> Result<Self, BoxDynError> {
        Ok(match value.format() {
            ValueFormat::Binary => value.as_bytes()?[0] != 0,

            ValueFormat::Text => match value.as_str()? {
                "t" => true,
                "f" => false,

                s => {
                    return Err(format!("unexpected value {s:?} for boolean").into());
                }
            },
        })
    }
}
