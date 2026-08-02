use crate::decode::Decode;
use crate::encode::{Encode, IsNull};
use crate::error::BoxDynError;
use crate::codec::time::PG_EPOCH;
use crate::codec::Type;
use crate::{ArgumentBuffer, HasArrayType, TypeInfo, ValueFormat, ValueRef};
use std::mem;
use time::macros::format_description;
use time::{Date, Duration};

impl Type for Date {
    fn type_info() -> TypeInfo {
        TypeInfo::DATE
    }
}

impl HasArrayType for Date {
    fn array_type_info() -> TypeInfo {
        TypeInfo::DATE_ARRAY
    }
}

impl Encode<'_> for Date {
    fn encode_by_ref(&self, buf: &mut ArgumentBuffer) -> Result<IsNull, BoxDynError> {
        // DATE is encoded as number of days since epoch (2000-01-01)
        let days: i32 = (*self - PG_EPOCH).whole_days().try_into().map_err(|_| {
            format!("value {self:?} would overflow binary encoding for Postgres DATE")
        })?;
        Encode::encode(days, buf)
    }

    fn size_hint(&self) -> usize {
        mem::size_of::<i32>()
    }
}

impl<'r> Decode<'r> for Date {
    fn decode(value: ValueRef<'r>) -> Result<Self, BoxDynError> {
        Ok(match value.format() {
            ValueFormat::Binary => {
                // DATE is encoded as the days since epoch
                let days: i32 = Decode::decode(value)?;
                PG_EPOCH + Duration::days(days.into())
            }

            ValueFormat::Text => Date::parse(
                value.as_str()?,
                &format_description!("[year]-[month]-[day]"),
            )?,
        })
    }
}
