use crate::decode::Decode;
use crate::encode::{Encode, IsNull};
use crate::error::BoxDynError;
use crate::codec::array_compatible;
use crate::codec::Type;
use crate::{ArgumentBuffer, HasArrayType, TypeInfo, ValueRef};
use std::borrow::Cow;
use std::rc::Rc;
use std::sync::Arc;

impl Type for str {
    fn type_info() -> TypeInfo {
        TypeInfo::TEXT
    }

    fn compatible(ty: &TypeInfo) -> bool {
        [
            TypeInfo::TEXT,
            TypeInfo::NAME,
            TypeInfo::BPCHAR,
            TypeInfo::VARCHAR,
            TypeInfo::UNKNOWN,
            TypeInfo::with_name("citext"),
        ]
        .contains(ty)
    }
}

impl Type for String {
    fn type_info() -> TypeInfo {
        <&str as Type>::type_info()
    }

    fn compatible(ty: &TypeInfo) -> bool {
        <&str as Type>::compatible(ty)
    }
}

impl HasArrayType for &'_ str {
    fn array_type_info() -> TypeInfo {
        TypeInfo::TEXT_ARRAY
    }

    fn array_compatible(ty: &TypeInfo) -> bool {
        array_compatible::<&str>(ty)
    }
}

impl HasArrayType for Cow<'_, str> {
    fn array_type_info() -> TypeInfo {
        <&str as HasArrayType>::array_type_info()
    }

    fn array_compatible(ty: &TypeInfo) -> bool {
        <&str as HasArrayType>::array_compatible(ty)
    }
}

impl HasArrayType for Box<str> {
    fn array_type_info() -> TypeInfo {
        <&str as HasArrayType>::array_type_info()
    }

    fn array_compatible(ty: &TypeInfo) -> bool {
        <&str as HasArrayType>::array_compatible(ty)
    }
}

impl HasArrayType for String {
    fn array_type_info() -> TypeInfo {
        <&str as HasArrayType>::array_type_info()
    }

    fn array_compatible(ty: &TypeInfo) -> bool {
        <&str as HasArrayType>::array_compatible(ty)
    }
}

impl Encode<'_> for &'_ str {
    fn encode_by_ref(&self, buf: &mut ArgumentBuffer) -> Result<IsNull, BoxDynError> {
        buf.extend(self.as_bytes());

        Ok(IsNull::No)
    }
}

impl<'r> Decode<'r> for &'r str {
    fn decode(value: ValueRef<'r>) -> Result<Self, BoxDynError> {
        value.as_str()
    }
}

impl Decode<'_> for String {
    fn decode(value: ValueRef<'_>) -> Result<Self, BoxDynError> {
        Ok(value.as_str()?.to_owned())
    }
}

crate::forward_encode_impl!(Arc<str>, &str);
crate::forward_encode_impl!(Rc<str>, &str);
crate::forward_encode_impl!(Cow<'_, str>, &str);
crate::forward_encode_impl!(Box<str>, &str);
crate::forward_encode_impl!(String, &str);
