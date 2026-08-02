use crate::decode::Decode;
use crate::encode::{Encode, IsNull};
use crate::error::BoxDynError;
use crate::codec::Type;
use crate::{ArgumentBuffer, HasArrayType, TypeInfo, ValueFormat, ValueRef};
use chrono::{
    DateTime, Duration, FixedOffset, Local, NaiveDate, NaiveDateTime, Offset, TimeZone, Utc,
};
use std::mem;

impl Type for NaiveDateTime {
    fn type_info() -> TypeInfo {
        TypeInfo::TIMESTAMP
    }
}

impl<Tz: TimeZone> Type for DateTime<Tz> {
    fn type_info() -> TypeInfo {
        TypeInfo::TIMESTAMPTZ
    }
}

impl HasArrayType for NaiveDateTime {
    fn array_type_info() -> TypeInfo {
        TypeInfo::TIMESTAMP_ARRAY
    }
}

impl<Tz: TimeZone> HasArrayType for DateTime<Tz> {
    fn array_type_info() -> TypeInfo {
        TypeInfo::TIMESTAMPTZ_ARRAY
    }
}

impl Encode<'_> for NaiveDateTime {
    fn encode_by_ref(&self, buf: &mut ArgumentBuffer) -> Result<IsNull, BoxDynError> {
        // TIMESTAMP is encoded as the microseconds since the epoch
        let micros = (*self - postgres_epoch_datetime())
            .num_microseconds()
            .ok_or_else(|| format!("NaiveDateTime out of range for Postgres: {self:?}"))?;

        Encode::encode(micros, buf)
    }

    fn size_hint(&self) -> usize {
        mem::size_of::<i64>()
    }
}

impl<'r> Decode<'r> for NaiveDateTime {
    fn decode(value: ValueRef<'r>) -> Result<Self, BoxDynError> {
        Ok(match value.format() {
            ValueFormat::Binary => {
                // TIMESTAMP is encoded as the microseconds since the epoch
                let us = Decode::decode(value)?;
                postgres_epoch_datetime() + Duration::microseconds(us)
            }

            ValueFormat::Text => {
                let s = value.as_str()?;
                NaiveDateTime::parse_from_str(
                    s,
                    if s.contains('+') {
                        // Contains a time-zone specifier
                        // This is given for timestamptz for some reason
                        // Postgres already guarantees this to always be UTC
                        "%Y-%m-%d %H:%M:%S%.f%#z"
                    } else {
                        "%Y-%m-%d %H:%M:%S%.f"
                    },
                )?
            }
        })
    }
}

impl<Tz: TimeZone> Encode<'_> for DateTime<Tz> {
    fn encode_by_ref(&self, buf: &mut ArgumentBuffer) -> Result<IsNull, BoxDynError> {
        Encode::encode(self.naive_utc(), buf)
    }

    fn size_hint(&self) -> usize {
        mem::size_of::<i64>()
    }
}

impl<'r> Decode<'r> for DateTime<Local> {
    fn decode(value: ValueRef<'r>) -> Result<Self, BoxDynError> {
        let fixed = <DateTime<FixedOffset> as Decode>::decode(value)?;
        Ok(Local.from_utc_datetime(&fixed.naive_utc()))
    }
}

impl<'r> Decode<'r> for DateTime<Utc> {
    fn decode(value: ValueRef<'r>) -> Result<Self, BoxDynError> {
        let fixed = <DateTime<FixedOffset> as Decode>::decode(value)?;
        Ok(Utc.from_utc_datetime(&fixed.naive_utc()))
    }
}

impl<'r> Decode<'r> for DateTime<FixedOffset> {
    fn decode(value: ValueRef<'r>) -> Result<Self, BoxDynError> {
        Ok(match value.format() {
            ValueFormat::Binary => {
                let naive = <NaiveDateTime as Decode>::decode(value)?;
                Utc.fix().from_utc_datetime(&naive)
            }

            ValueFormat::Text => {
                let s = value.as_str()?;
                DateTime::parse_from_str(
                    s,
                    if s.contains('+') || s.contains('-') {
                        // Contains a time-zone specifier
                        // This is given for timestamptz for some reason
                        // Postgres already guarantees this to always be UTC
                        "%Y-%m-%d %H:%M:%S%.f%#z"
                    } else {
                        "%Y-%m-%d %H:%M:%S%.f"
                    },
                )?
            }
        })
    }
}

#[inline]
fn postgres_epoch_datetime() -> NaiveDateTime {
    NaiveDate::from_ymd_opt(2000, 1, 1)
        .expect("expected 2000-01-01 to be a valid NaiveDate")
        .and_hms_opt(0, 0, 0)
        .expect("expected 2000-01-01T00:00:00 to be a valid NaiveDateTime")
}
