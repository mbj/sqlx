use crate::decode::Decode;
use crate::encode::{Encode, IsNull};
use crate::error::BoxDynError;
use crate::codec::Type;
use crate::{ArgumentBuffer, HasArrayType, TypeInfo, ValueFormat, ValueRef};
use chrono::{Duration, NaiveTime};
use std::mem;

impl Type for NaiveTime {
    fn type_info() -> TypeInfo {
        TypeInfo::TIME
    }
}

impl HasArrayType for NaiveTime {
    fn array_type_info() -> TypeInfo {
        TypeInfo::TIME_ARRAY
    }
}

impl Encode<'_> for NaiveTime {
    fn encode_by_ref(&self, buf: &mut ArgumentBuffer) -> Result<IsNull, BoxDynError> {
        // TIME is encoded as the microseconds since midnight
        let micros = (*self - NaiveTime::default())
            .num_microseconds()
            .ok_or_else(|| format!("Time out of range for PostgreSQL: {self}"))?;

        Encode::encode(micros, buf)
    }

    fn size_hint(&self) -> usize {
        mem::size_of::<u64>()
    }
}

impl<'r> Decode<'r> for NaiveTime {
    fn decode(value: ValueRef<'r>) -> Result<Self, BoxDynError> {
        Ok(match value.format() {
            ValueFormat::Binary => {
                // TIME is encoded as the microseconds since midnight
                let us: i64 = Decode::decode(value)?;
                NaiveTime::default() + Duration::microseconds(us)
            }

            ValueFormat::Text => NaiveTime::parse_from_str(value.as_str()?, "%H:%M:%S%.f")?,
        })
    }
}

#[test]
fn check_naive_time_default_is_midnight() {
    // Just a canary in case this changes.
    assert_eq!(
        NaiveTime::from_hms_opt(0, 0, 0),
        Some(NaiveTime::default()),
        "implementation assumes `NaiveTime::default()` equals midnight"
    );
}
