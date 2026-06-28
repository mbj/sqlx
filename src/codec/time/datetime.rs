use crate::decode::Decode;
use crate::encode::{Encode, IsNull};
use crate::error::BoxDynError;
use crate::codec::time::PG_EPOCH;
use crate::codec::Type;
use crate::{ArgumentBuffer, HasArrayType, TypeInfo, ValueFormat, ValueRef};
use std::borrow::Cow;
use std::mem;
use time::macros::format_description;
use time::macros::offset;
use time::{Duration, OffsetDateTime, PrimitiveDateTime};

impl Type for PrimitiveDateTime {
    fn type_info() -> TypeInfo {
        TypeInfo::TIMESTAMP
    }
}

impl Type for OffsetDateTime {
    fn type_info() -> TypeInfo {
        TypeInfo::TIMESTAMPTZ
    }
}

impl HasArrayType for PrimitiveDateTime {
    fn array_type_info() -> TypeInfo {
        TypeInfo::TIMESTAMP_ARRAY
    }
}

impl HasArrayType for OffsetDateTime {
    fn array_type_info() -> TypeInfo {
        TypeInfo::TIMESTAMPTZ_ARRAY
    }
}

impl Encode<'_> for PrimitiveDateTime {
    fn encode_by_ref(&self, buf: &mut ArgumentBuffer) -> Result<IsNull, BoxDynError> {
        // TIMESTAMP is encoded as the microseconds since the epoch
        let micros: i64 = (*self - PG_EPOCH.midnight())
            .whole_microseconds()
            .try_into()
            .map_err(|_| {
                format!("value {self:?} would overflow binary encoding for Postgres TIME")
            })?;
        Encode::encode(micros, buf)
    }

    fn size_hint(&self) -> usize {
        mem::size_of::<i64>()
    }
}

impl<'r> Decode<'r> for PrimitiveDateTime {
    fn decode(value: ValueRef<'r>) -> Result<Self, BoxDynError> {
        Ok(match value.format() {
            ValueFormat::Binary => {
                // TIMESTAMP is encoded as the microseconds since the epoch
                let us = Decode::decode(value)?;
                PG_EPOCH.midnight() + Duration::microseconds(us)
            }

            ValueFormat::Text => {
                let s = value.as_str()?;

                // If there is no decimal point we need to add one.
                let s = if s.contains('.') {
                    Cow::Borrowed(s)
                } else {
                    Cow::Owned(format!("{s}.0"))
                };

                // Contains a time-zone specifier
                // This is given for timestamptz for some reason
                // Postgres already guarantees this to always be UTC
                if s.contains('+') {
                    PrimitiveDateTime::parse(&s, &format_description!("[year]-[month]-[day] [hour]:[minute]:[second].[subsecond][offset_hour]"))?
                } else {
                    PrimitiveDateTime::parse(
                        &s,
                        &format_description!(
                            "[year]-[month]-[day] [hour]:[minute]:[second].[subsecond]"
                        ),
                    )?
                }
            }
        })
    }
}

impl Encode<'_> for OffsetDateTime {
    fn encode_by_ref(&self, buf: &mut ArgumentBuffer) -> Result<IsNull, BoxDynError> {
        let utc = self.to_offset(offset!(UTC));
        let primitive = PrimitiveDateTime::new(utc.date(), utc.time());

        Encode::encode(primitive, buf)
    }

    fn size_hint(&self) -> usize {
        mem::size_of::<i64>()
    }
}

impl<'r> Decode<'r> for OffsetDateTime {
    fn decode(value: ValueRef<'r>) -> Result<Self, BoxDynError> {
        Ok(<PrimitiveDateTime as Decode>::decode(value)?.assume_utc())
    }
}
