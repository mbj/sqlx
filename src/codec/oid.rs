use byteorder::{BigEndian, ByteOrder};

use crate::decode::Decode;
use crate::encode::{Encode, IsNull};
use crate::error::BoxDynError;
use crate::codec::Type;
use crate::{ArgumentBuffer, HasArrayType, TypeInfo, ValueFormat, ValueRef};

/// The PostgreSQL [`OID`] type stores an object identifier,
/// used internally by PostgreSQL as primary keys for various system tables.
///
/// [`OID`]: https://www.postgresql.org/docs/current/datatype-oid.html
#[derive(Debug, Copy, Clone, Hash, PartialEq, Eq, Default)]
pub struct Oid(
    /// The raw unsigned integer value sent over the wire
    pub u32,
);

impl Type for Oid {
    fn type_info() -> TypeInfo {
        TypeInfo::OID
    }
}

impl HasArrayType for Oid {
    fn array_type_info() -> TypeInfo {
        TypeInfo::OID_ARRAY
    }
}

impl Encode<'_> for Oid {
    fn encode_by_ref(&self, buf: &mut ArgumentBuffer) -> Result<IsNull, BoxDynError> {
        buf.extend(&self.0.to_be_bytes());

        Ok(IsNull::No)
    }
}

impl Decode<'_> for Oid {
    fn decode(value: ValueRef<'_>) -> Result<Self, BoxDynError> {
        Ok(Self(match value.format() {
            ValueFormat::Binary => BigEndian::read_u32(value.as_bytes()?),
            ValueFormat::Text => value.as_str()?.parse()?,
        }))
    }
}

