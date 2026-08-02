use crate::decode::Decode;
use crate::encode::{Encode, IsNull};
use crate::error::BoxDynError;
use crate::codec::Type;
use crate::{ArgumentBuffer, HasArrayType, TypeInfo, ValueFormat, ValueRef};
use std::mem;
use time::macros::format_description;
use time::{Duration, Time};

impl Type for Time {
    fn type_info() -> TypeInfo {
        TypeInfo::TIME
    }
}

impl HasArrayType for Time {
    fn array_type_info() -> TypeInfo {
        TypeInfo::TIME_ARRAY
    }
}

impl Encode<'_> for Time {
    fn encode_by_ref(&self, buf: &mut ArgumentBuffer) -> Result<IsNull, BoxDynError> {
        // TIME is encoded as the microseconds since midnight.
        //
        // A truncating cast is fine because `self - Time::MIDNIGHT` cannot exceed a span of 24 hours.
        #[allow(clippy::cast_possible_truncation)]
        let micros: i64 = (*self - Time::MIDNIGHT).whole_microseconds() as i64;
        Encode::encode(micros, buf)
    }

    fn size_hint(&self) -> usize {
        mem::size_of::<u64>()
    }
}

impl<'r> Decode<'r> for Time {
    fn decode(value: ValueRef<'r>) -> Result<Self, BoxDynError> {
        Ok(match value.format() {
            ValueFormat::Binary => {
                // TIME is encoded as the microseconds since midnight
                let us = Decode::decode(value)?;
                Time::MIDNIGHT + Duration::microseconds(us)
            }

            ValueFormat::Text => Time::parse(
                value.as_str()?,
                // Postgres will not include the subsecond part if it's zero.
                &format_description!("[hour]:[minute]:[second][optional [.[subsecond]]]"),
            )?,
        })
    }
}
