use crate::decode::Decode;
use crate::error::BoxDynError;
use crate::codec::Type;
use crate::{TypeInfo, ValueRef};

impl Type for () {
    fn type_info() -> TypeInfo {
        TypeInfo::VOID
    }

    fn compatible(ty: &TypeInfo) -> bool {
        // RECORD is here so we can support the empty tuple
        *ty == TypeInfo::VOID || *ty == TypeInfo::RECORD
    }
}

impl<'r> Decode<'r> for () {
    fn decode(_value: ValueRef<'r>) -> Result<Self, BoxDynError> {
        Ok(())
    }
}
