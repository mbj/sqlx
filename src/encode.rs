//! Provides [`Encode`] for encoding values for the database.

use std::borrow::Cow;
use std::mem;
use std::rc::Rc;
use std::sync::Arc;

use crate::error::BoxDynError;

/// The return type of [Encode::encode].
#[must_use]
pub enum IsNull {
    /// The value is null; no data was written.
    Yes,

    /// The value is not null.
    ///
    /// This does not mean that data was written.
    No,
}

impl IsNull {
    pub fn is_null(&self) -> bool {
        matches!(self, IsNull::Yes)
    }
}

/// Encode a single value to be sent to the database.
pub trait Encode<'q> {
    /// Writes the value of `self` into `buf` in the expected format for the database.
    fn encode(self, buf: &mut crate::ArgumentBuffer) -> Result<IsNull, BoxDynError>
    where
        Self: Sized,
    {
        self.encode_by_ref(buf)
    }

    /// Writes the value of `self` into `buf` without moving `self`.
    ///
    /// Where possible, make use of `encode` instead as it can take advantage of re-using
    /// memory.
    fn encode_by_ref(
        &self,
        buf: &mut crate::ArgumentBuffer,
    ) -> Result<IsNull, BoxDynError>;

    fn produces(&self) -> Option<crate::TypeInfo> {
        // `produces` is inherently a hook to allow database drivers to produce value-dependent
        // type information; if the driver doesn't need this, it can leave this as `None`
        None
    }

    #[inline]
    fn size_hint(&self) -> usize {
        mem::size_of_val(self)
    }
}

impl<'q, T> Encode<'q> for &'_ T
where
    T: Encode<'q>,
{
    #[inline]
    fn encode(self, buf: &mut crate::ArgumentBuffer) -> Result<IsNull, BoxDynError> {
        <T as Encode>::encode_by_ref(self, buf)
    }

    #[inline]
    fn encode_by_ref(
        &self,
        buf: &mut crate::ArgumentBuffer,
    ) -> Result<IsNull, BoxDynError> {
        <&T as Encode>::encode(self, buf)
    }

    #[inline]
    fn produces(&self) -> Option<crate::TypeInfo> {
        (**self).produces()
    }

    #[inline]
    fn size_hint(&self) -> usize {
        (**self).size_hint()
    }
}

#[macro_export]
macro_rules! impl_encode_for_option {
    () => {
        impl<'q, T> $crate::encode::Encode<'q> for Option<T>
        where
            T: $crate::encode::Encode<'q> + $crate::codec::Type + 'q,
        {
            #[inline]
            fn produces(&self) -> Option<$crate::TypeInfo> {
                if let Some(v) = self {
                    v.produces()
                } else {
                    T::type_info().into()
                }
            }

            #[inline]
            fn encode(
                self,
                buf: &mut $crate::ArgumentBuffer,
            ) -> Result<$crate::encode::IsNull, $crate::error::BoxDynError> {
                if let Some(v) = self {
                    v.encode(buf)
                } else {
                    Ok($crate::encode::IsNull::Yes)
                }
            }

            #[inline]
            fn encode_by_ref(
                &self,
                buf: &mut $crate::ArgumentBuffer,
            ) -> Result<$crate::encode::IsNull, $crate::error::BoxDynError> {
                if let Some(v) = self {
                    v.encode_by_ref(buf)
                } else {
                    Ok($crate::encode::IsNull::Yes)
                }
            }

            #[inline]
            fn size_hint(&self) -> usize {
                self.as_ref().map_or(0, $crate::encode::Encode::size_hint)
            }
        }
    };
}

macro_rules! impl_encode_for_smartpointer {
    ($smart_pointer:ty) => {
        impl<'q, T> Encode<'q> for $smart_pointer
        where
            T: Encode<'q>,
        {
            #[inline]
            fn encode(
                self,
                buf: &mut crate::ArgumentBuffer,
            ) -> Result<IsNull, BoxDynError> {
                <T as Encode>::encode_by_ref(self.as_ref(), buf)
            }

            #[inline]
            fn encode_by_ref(
                &self,
                buf: &mut crate::ArgumentBuffer,
            ) -> Result<IsNull, BoxDynError> {
                <&T as Encode>::encode(self, buf)
            }

            #[inline]
            fn produces(&self) -> Option<crate::TypeInfo> {
                (**self).produces()
            }

            #[inline]
            fn size_hint(&self) -> usize {
                (**self).size_hint()
            }
        }
    };
}

impl_encode_for_smartpointer!(Arc<T>);
impl_encode_for_smartpointer!(Box<T>);
impl_encode_for_smartpointer!(Rc<T>);

impl<'q, T> Encode<'q> for Cow<'q, T>
where
    T: Encode<'q>,
    T: ToOwned,
{
    #[inline]
    fn encode(self, buf: &mut crate::ArgumentBuffer) -> Result<IsNull, BoxDynError> {
        <&T as Encode>::encode_by_ref(&self.as_ref(), buf)
    }

    #[inline]
    fn encode_by_ref(
        &self,
        buf: &mut crate::ArgumentBuffer,
    ) -> Result<IsNull, BoxDynError> {
        <&T as Encode>::encode_by_ref(&self.as_ref(), buf)
    }

    #[inline]
    fn produces(&self) -> Option<crate::TypeInfo> {
        <&T as Encode>::produces(&self.as_ref())
    }

    #[inline]
    fn size_hint(&self) -> usize {
        <&T as Encode>::size_hint(&self.as_ref())
    }
}

#[macro_export]
macro_rules! forward_encode_impl {
    ($for_type:ty, $forward_to:ty) => {
        impl<'q> $crate::encode::Encode<'q> for $for_type {
            fn encode_by_ref(
                &self,
                buf: &mut $crate::ArgumentBuffer,
            ) -> Result<$crate::encode::IsNull, $crate::error::BoxDynError> {
                <$forward_to as $crate::encode::Encode>::encode(self.as_ref(), buf)
            }
        }
    };
}
