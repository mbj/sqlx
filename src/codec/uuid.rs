use uuid::Uuid;

use crate::decode::Decode;
use crate::encode::{Encode, IsNull};
use crate::error::BoxDynError;
use crate::codec::Type;
use crate::{ArgumentBuffer, HasArrayType, TypeInfo, ValueFormat, ValueRef};

impl Type for Uuid {
    fn type_info() -> TypeInfo {
        TypeInfo::UUID
    }
}

impl HasArrayType for Uuid {
    fn array_type_info() -> TypeInfo {
        TypeInfo::UUID_ARRAY
    }
}

impl Encode<'_> for Uuid {
    fn encode_by_ref(&self, buf: &mut ArgumentBuffer) -> Result<IsNull, BoxDynError> {
        buf.extend_from_slice(self.as_bytes());

        Ok(IsNull::No)
    }
}

impl Decode<'_> for Uuid {
    fn decode(value: ValueRef<'_>) -> Result<Self, BoxDynError> {
        match value.format() {
            ValueFormat::Binary => Uuid::from_slice(value.as_bytes()?),
            ValueFormat::Text => value.as_str()?.parse(),
        }
        .map_err(Into::into)
    }
}
