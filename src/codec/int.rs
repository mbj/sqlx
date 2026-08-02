use byteorder::{BigEndian, ByteOrder};
use std::num::{NonZeroI16, NonZeroI32, NonZeroI64};

use crate::decode::Decode;
use crate::encode::{Encode, IsNull};
use crate::error::BoxDynError;
use crate::codec::Type;
use crate::{ArgumentBuffer, HasArrayType, TypeInfo, ValueFormat, ValueRef};

fn int_decode(value: ValueRef<'_>) -> Result<i64, BoxDynError> {
    Ok(match value.format() {
        ValueFormat::Text => value.as_str()?.parse()?,
        ValueFormat::Binary => {
            let buf = value.as_bytes()?;

            // Return error if buf is empty or is more than 8 bytes
            match buf.len() {
                0 => {
                    return Err("Value Buffer found empty while decoding to integer type".into());
                }
                buf_len @ 9.. => {
                    return Err(format!(
                        "Value Buffer exceeds 8 bytes while decoding to integer type. Buffer size = {} bytes ", buf_len
                    )
                    .into());
                }
                _ => {}
            }

            BigEndian::read_int(buf, buf.len())
        }
    })
}

impl Type for i8 {
    fn type_info() -> TypeInfo {
        TypeInfo::CHAR
    }
}

impl HasArrayType for i8 {
    fn array_type_info() -> TypeInfo {
        TypeInfo::CHAR_ARRAY
    }
}

impl Encode<'_> for i8 {
    fn encode_by_ref(&self, buf: &mut ArgumentBuffer) -> Result<IsNull, BoxDynError> {
        buf.extend(&self.to_be_bytes());

        Ok(IsNull::No)
    }
}

impl Decode<'_> for i8 {
    fn decode(value: ValueRef<'_>) -> Result<Self, BoxDynError> {
        // note: decoding here is for the `"char"` type as Postgres does not have a native 1-byte integer type.
        // https://github.com/postgres/postgres/blob/master/src/backend/utils/adt/char.c#L58-L60
        match value.format() {
            ValueFormat::Binary => int_decode(value)?.try_into().map_err(Into::into),
            ValueFormat::Text => {
                let text = value.as_str()?;

                // A value of 0 is represented with the empty string.
                if text.is_empty() {
                    return Ok(0);
                }

                if text.starts_with('\\') {
                    // For values between 0x80 and 0xFF, it's encoded in octal.
                    return Ok(i8::from_str_radix(text.trim_start_matches('\\'), 8)?);
                }

                // Wrapping is the whole idea.
                #[allow(clippy::cast_possible_wrap)]
                Ok(text.as_bytes()[0] as i8)
            }
        }
    }
}

impl Type for i16 {
    fn type_info() -> TypeInfo {
        TypeInfo::INT2
    }
}

impl HasArrayType for i16 {
    fn array_type_info() -> TypeInfo {
        TypeInfo::INT2_ARRAY
    }
}

impl Encode<'_> for i16 {
    fn encode_by_ref(&self, buf: &mut ArgumentBuffer) -> Result<IsNull, BoxDynError> {
        buf.extend(&self.to_be_bytes());

        Ok(IsNull::No)
    }
}

impl Decode<'_> for i16 {
    fn decode(value: ValueRef<'_>) -> Result<Self, BoxDynError> {
        int_decode(value)?.try_into().map_err(Into::into)
    }
}

impl Type for i32 {
    fn type_info() -> TypeInfo {
        TypeInfo::INT4
    }
}

impl HasArrayType for i32 {
    fn array_type_info() -> TypeInfo {
        TypeInfo::INT4_ARRAY
    }
}

impl Encode<'_> for i32 {
    fn encode_by_ref(&self, buf: &mut ArgumentBuffer) -> Result<IsNull, BoxDynError> {
        buf.extend(&self.to_be_bytes());

        Ok(IsNull::No)
    }
}

impl Decode<'_> for i32 {
    fn decode(value: ValueRef<'_>) -> Result<Self, BoxDynError> {
        int_decode(value)?.try_into().map_err(Into::into)
    }
}

impl Type for i64 {
    fn type_info() -> TypeInfo {
        TypeInfo::INT8
    }
}

impl HasArrayType for i64 {
    fn array_type_info() -> TypeInfo {
        TypeInfo::INT8_ARRAY
    }
}

impl Encode<'_> for i64 {
    fn encode_by_ref(&self, buf: &mut ArgumentBuffer) -> Result<IsNull, BoxDynError> {
        buf.extend(&self.to_be_bytes());

        Ok(IsNull::No)
    }
}

impl Decode<'_> for i64 {
    fn decode(value: ValueRef<'_>) -> Result<Self, BoxDynError> {
        int_decode(value)
    }
}

impl HasArrayType for NonZeroI16 {
    fn array_type_info() -> TypeInfo {
        TypeInfo::INT2_ARRAY
    }
}

impl HasArrayType for NonZeroI32 {
    fn array_type_info() -> TypeInfo {
        TypeInfo::INT4_ARRAY
    }
}

impl HasArrayType for NonZeroI64 {
    fn array_type_info() -> TypeInfo {
        TypeInfo::INT8_ARRAY
    }
}
