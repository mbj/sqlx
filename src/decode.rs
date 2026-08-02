//! Provides [`Decode`] for decoding values from the database.

use std::borrow::Cow;
use std::rc::Rc;
use std::sync::Arc;

use crate::error::BoxDynError;

/// A type that can be decoded from the database.
///
/// ## How can I implement `Decode`?
///
/// A manual implementation of `Decode` can be useful when adding support for
/// types externally to SQLx.
///
/// The following showcases how to implement `Decode` for a type that delegates to
/// string decoding and `FromStr` parsing:
///
/// ```rust
/// use sqlx::decode::Decode;
/// use sqlx::error::BoxDynError;
/// use sqlx::ValueRef;
///
/// struct MyType;
///
/// # impl std::str::FromStr for MyType {
/// #     type Err = BoxDynError;
/// #     fn from_str(_: &str) -> Result<Self, Self::Err> { todo!() }
/// # }
///
/// // `'r` is the lifetime of the `Row` being decoded.
/// impl<'r> Decode<'r> for MyType {
///     fn decode(value: ValueRef<'r>) -> Result<MyType, BoxDynError> {
///         // Delegate to a type that matches the wire format of the value
///         // you want to decode (here, a UTF-8 string).
///         let value = <&str as Decode>::decode(value)?;
///
///         // Then parse it into your type using `FromStr` (or any other conversion).
///         Ok(value.parse()?)
///     }
/// }
/// ```
pub trait Decode<'r>: Sized {
    /// Decode a new value of this type using a raw value from the database.
    fn decode(value: crate::ValueRef<'r>) -> Result<Self, BoxDynError>;
}

// implement `Decode` for Option<T> for all SQL types
impl<'r, T> Decode<'r> for Option<T>
where
    T: Decode<'r>,
{
    fn decode(value: crate::ValueRef<'r>) -> Result<Self, BoxDynError> {
        if value.is_null() {
            Ok(None)
        } else {
            Ok(Some(T::decode(value)?))
        }
    }
}

macro_rules! impl_decode_for_smartpointer {
    ($smart_pointer:tt) => {
        impl<'r, T> Decode<'r> for $smart_pointer<T>
        where
            T: Decode<'r>,
        {
            fn decode(value: crate::ValueRef<'r>) -> Result<Self, BoxDynError> {
                Ok(Self::new(T::decode(value)?))
            }
        }

        impl<'r> Decode<'r> for $smart_pointer<str>
        where
            &'r str: Decode<'r>,
        {
            fn decode(value: crate::ValueRef<'r>) -> Result<Self, BoxDynError> {
                let ref_str = <&str as Decode>::decode(value)?;
                Ok(ref_str.into())
            }
        }

        impl<'r> Decode<'r> for $smart_pointer<[u8]>
        where
            Vec<u8>: Decode<'r>,
        {
            fn decode(value: crate::ValueRef<'r>) -> Result<Self, BoxDynError> {
                // The `Postgres` implementation requires this to be decoded as an owned value because
                // bytes can be sent in text format.
                let bytes = <Vec<u8> as Decode>::decode(value)?;
                Ok(bytes.into())
            }
        }
    };
}

impl_decode_for_smartpointer!(Arc);
impl_decode_for_smartpointer!(Box);
impl_decode_for_smartpointer!(Rc);

// implement `Decode` for Cow<T> for all SQL types
impl<'r, T> Decode<'r> for Cow<'_, T>
where
    // `ToOwned` is required here to satisfy `Cow`
    T: ToOwned + ?Sized,
    <T as ToOwned>::Owned: Decode<'r>,
{
    fn decode(value: crate::ValueRef<'r>) -> Result<Self, BoxDynError> {
        // See https://github.com/launchbadge/sqlx/pull/3674#discussion_r2008611502 for more info
        // about why decoding to a `Cow::Owned` was chosen.
        <<T as ToOwned>::Owned as Decode>::decode(value).map(Cow::Owned)
    }
}
