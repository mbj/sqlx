use crate::codec::array_compatible;
use crate::{ArgumentBuffer, HasArrayType, TypeInfo, ValueRef};
use crate::decode::Decode;
use crate::encode::{Encode, IsNull};
use crate::error::BoxDynError;
use crate::codec::Type;
use std::fmt;
use std::fmt::{Debug, Display, Formatter};
use std::ops::Deref;
use std::str::FromStr;

/// Case-insensitive text (`citext`) support for Postgres.
///
/// Note that SQLx considers the `citext` type to be compatible with `String`
/// and its various derivatives, so direct usage of this type is generally unnecessary.
///
/// However, it may be needed, for example, when binding a `citext[]` array,
/// as Postgres will generally not accept a `text[]` array (mapped from `Vec<String>`) in its place.
///
/// See [the Postgres manual, Appendix F, Section 10][PG.F.10] for details on using `citext`.
///
/// [PG.F.10]: https://www.postgresql.org/docs/current/citext.html
///
/// ### Note: Extension Required
/// The `citext` extension is not enabled by default in Postgres. You will need to do so explicitly:
///
/// ```ignore
/// CREATE EXTENSION IF NOT EXISTS "citext";
/// ```
///
/// ### Note: `PartialEq` is Case-Sensitive
/// This type derives `PartialEq` which forwards to the implementation on `String`, which
/// is case-sensitive. This impl exists mainly for testing.
///
/// To properly emulate the case-insensitivity of `citext` would require use of locale-aware
/// functions in `libc`, and even then would require querying the locale of the database server
/// and setting it locally, which is unsafe.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct CiText(pub String);

impl Type for CiText {
    fn type_info() -> TypeInfo {
        // Since `citext` is enabled by an extension, it does not have a stable OID.
        TypeInfo::with_name("citext")
    }

    fn compatible(ty: &TypeInfo) -> bool {
        <&str as Type>::compatible(ty)
    }
}

impl Deref for CiText {
    type Target = str;

    fn deref(&self) -> &Self::Target {
        self.0.as_str()
    }
}

impl From<String> for CiText {
    fn from(value: String) -> Self {
        Self(value)
    }
}

impl From<CiText> for String {
    fn from(value: CiText) -> Self {
        value.0
    }
}

impl FromStr for CiText {
    type Err = core::convert::Infallible;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(CiText(s.parse()?))
    }
}

impl Display for CiText {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl HasArrayType for CiText {
    fn array_type_info() -> TypeInfo {
        TypeInfo::with_name("_citext")
    }

    fn array_compatible(ty: &TypeInfo) -> bool {
        array_compatible::<&str>(ty)
    }
}

impl Encode<'_> for CiText {
    fn encode_by_ref(&self, buf: &mut ArgumentBuffer) -> Result<IsNull, BoxDynError> {
        <&str as Encode>::encode(&**self, buf)
    }
}

impl Decode<'_> for CiText {
    fn decode(value: ValueRef<'_>) -> Result<Self, BoxDynError> {
        Ok(CiText(value.as_str()?.to_owned()))
    }
}
