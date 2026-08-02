//! [`Type`], [`Encode`], and [`Decode`] implementations for the various [`NonZero*`][non-zero]
//! types from the standard library.
//!
//! [non-zero]: core::num::NonZero

use std::num::{NonZeroI16, NonZeroI32, NonZeroI64, NonZeroI8};

use crate::decode::Decode;
use crate::encode::{Encode, IsNull};
use crate::codec::Type;

macro_rules! impl_non_zero {
    ($($int:ty => $non_zero:ty),* $(,)?) => {
        $(impl Type for $non_zero
        where
            $int: Type,
        {
            fn type_info() -> crate::TypeInfo {
                <$int as Type>::type_info()
            }

            fn compatible(ty: &crate::TypeInfo) -> bool {
                <$int as Type>::compatible(ty)
            }
        }

        impl<'q> Encode<'q> for $non_zero
        where
            $int: Encode<'q>,
        {
            fn encode_by_ref(&self, buf: &mut crate::ArgumentBuffer) -> Result<IsNull, crate::error::BoxDynError> {
                <$int as Encode<'q>>::encode_by_ref(&self.get(), buf)
            }

            fn encode(self, buf: &mut crate::ArgumentBuffer) -> Result<IsNull, crate::error::BoxDynError>
            where
                Self: Sized,
            {
                <$int as Encode<'q>>::encode(self.get(), buf)
            }

            fn produces(&self) -> Option<crate::TypeInfo> {
                <$int as Encode<'q>>::produces(&self.get())
            }
        }

        impl<'r> Decode<'r> for $non_zero
        where
            $int: Decode<'r>,
        {
            fn decode(value: crate::ValueRef<'r>) -> Result<Self, crate::error::BoxDynError> {
                let int = <$int as Decode<'r>>::decode(value)?;
                let non_zero = Self::try_from(int)?;

                Ok(non_zero)
            }
        })*
    };
}

impl_non_zero! {
    i8 => NonZeroI8,
    i16 => NonZeroI16,
    i32 => NonZeroI32,
    i64 => NonZeroI64,
}
