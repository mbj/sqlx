use std::mem;

use chrono::{NaiveDate, TimeDelta};

use crate::decode::Decode;
use crate::encode::{Encode, IsNull};
use crate::error::BoxDynError;
use crate::codec::Type;
use crate::{ArgumentBuffer, HasArrayType, TypeInfo, ValueFormat, ValueRef};

impl Type for NaiveDate {
    fn type_info() -> TypeInfo {
        TypeInfo::DATE
    }
}

impl HasArrayType for NaiveDate {
    fn array_type_info() -> TypeInfo {
        TypeInfo::DATE_ARRAY
    }
}

impl Encode<'_> for NaiveDate {
    fn encode_by_ref(&self, buf: &mut ArgumentBuffer) -> Result<IsNull, BoxDynError> {
        // DATE is encoded as the days since epoch
        let days: i32 = (*self - postgres_epoch_date())
            .num_days()
            .try_into()
            .map_err(|_| {
                format!("value {self:?} would overflow binary encoding for Postgres DATE")
            })?;

        Encode::encode(days, buf)
    }

    fn size_hint(&self) -> usize {
        mem::size_of::<i32>()
    }
}

impl<'r> Decode<'r> for NaiveDate {
    fn decode(value: ValueRef<'r>) -> Result<Self, BoxDynError> {
        Ok(match value.format() {
            ValueFormat::Binary => {
                // DATE is encoded as the days since epoch
                let days: i32 = Decode::decode(value)?;

                let days = TimeDelta::try_days(days.into())
                    .unwrap_or_else(|| {
                        unreachable!("BUG: days ({days}) as `i32` multiplied into seconds should not overflow `i64`")
                    });

                postgres_epoch_date() + days
            }

            ValueFormat::Text => NaiveDate::parse_from_str(value.as_str()?, "%Y-%m-%d")?,
        })
    }
}

#[inline]
fn postgres_epoch_date() -> NaiveDate {
    NaiveDate::from_ymd_opt(2000, 1, 1).expect("expected 2000-01-01 to be a valid NaiveDate")
}
