#[doc(no_inline)]
pub use ::mac_address::MacAddress;

use crate::decode::Decode;
use crate::encode::{Encode, IsNull};
use crate::error::BoxDynError;
use crate::codec::Type;
use crate::{ArgumentBuffer, HasArrayType, TypeInfo, ValueFormat, ValueRef};

impl Type for MacAddress {
    fn type_info() -> TypeInfo {
        TypeInfo::MACADDR
    }

    fn compatible(ty: &TypeInfo) -> bool {
        *ty == TypeInfo::MACADDR
    }
}

impl HasArrayType for MacAddress {
    fn array_type_info() -> TypeInfo {
        TypeInfo::MACADDR_ARRAY
    }
}

impl Encode<'_> for MacAddress {
    fn encode_by_ref(&self, buf: &mut ArgumentBuffer) -> Result<IsNull, BoxDynError> {
        buf.extend_from_slice(&self.bytes()); // write just the address
        Ok(IsNull::No)
    }

    fn size_hint(&self) -> usize {
        6
    }
}

impl Decode<'_> for MacAddress {
    fn decode(value: ValueRef<'_>) -> Result<Self, BoxDynError> {
        let bytes = match value.format() {
            ValueFormat::Binary => value.as_bytes()?,
            ValueFormat::Text => {
                return Ok(value.as_str()?.parse()?);
            }
        };

        if bytes.len() == 6 {
            return Ok(MacAddress::new(bytes.try_into().unwrap()));
        }

        Err("invalid data received when expecting an MACADDR".into())
    }
}
