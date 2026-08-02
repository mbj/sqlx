//! Conversions between Rust and **Postgres** types.
//!
//! # Types
//!
//! | Rust type                             | Postgres type(s)                                     |
//! |---------------------------------------|------------------------------------------------------|
//! | `bool`                                | BOOL                                                 |
//! | `i8`                                  | "CHAR"                                               |
//! | `i16`                                 | SMALLINT, SMALLSERIAL, INT2                          |
//! | `i32`                                 | INT, SERIAL, INT4                                    |
//! | `i64`                                 | BIGINT, BIGSERIAL, INT8                              |
//! | `f32`                                 | REAL, FLOAT4                                         |
//! | `f64`                                 | DOUBLE PRECISION, FLOAT8                             |
//! | `&str`, [`String`]                    | VARCHAR, CHAR(N), TEXT, NAME, CITEXT                 |
//! | `&[u8]`, `Vec<u8>`                    | BYTEA                                                |
//! | `()`                                  | VOID                                                 |
//! | [`Interval`]                          | INTERVAL                                             |
//! | [`Range<T>`](Range)                   | INT8RANGE, INT4RANGE, TSRANGE, TSTZRANGE, DATERANGE, NUMRANGE |
//! | [`Money`]                             | MONEY                                                |
//! | [`LTree`]                             | LTREE                                                |
//! | [`LQuery`]                            | LQUERY                                               |
//! | [`CiText`]                            | CITEXT<sup>1</sup>                                   |
//! | [`Cube`]                              | CUBE                                                 |
//! | [`Point`]                             | POINT                                                |
//! | [`Line`]                              | LINE                                                 |
//! | [`LSeg`]                              | LSEG                                                 |
//! | [`Box`]                               | BOX                                                  |
//! | [`Path`]                              | PATH                                                 |
//! | [`Polygon`]                           | POLYGON                                              |
//! | [`Circle`]                            | CIRCLE                                               |
//! | [`Hstore`]                            | HSTORE                                               |
//!
//! <sup>1</sup> SQLx generally considers `CITEXT` to be compatible with `String`, `&str`, etc.,
//! but this wrapper type is available for edge cases, such as `CITEXT[]` which Postgres
//! does not consider to be compatible with `TEXT[]`.
//!
//! ### [`bigdecimal`](https://crates.io/crates/bigdecimal)
//! Requires the `bigdecimal` Cargo feature flag.
//!
//! | Rust type                             | Postgres type(s)                                        |
//! |---------------------------------------|------------------------------------------------------|
//! | `bigdecimal::BigDecimal`              | NUMERIC                                              |
//!
#![doc=include_str!("codec/bigdecimal-range.md")]
//!
//! ### [`rust_decimal`](https://crates.io/crates/rust_decimal)
//! Requires the `rust_decimal` Cargo feature flag.
//!
//! | Rust type                             | Postgres type(s)                                        |
//! |---------------------------------------|------------------------------------------------------|
//! | `rust_decimal::Decimal`               | NUMERIC                                              |
//!
#![doc=include_str!("codec/rust_decimal-range.md")]
//!
//! ### [`chrono`](https://crates.io/crates/chrono)
//!
//! Requires the `chrono` Cargo feature flag.
//!
//! | Rust type                             | Postgres type(s)                                     |
//! |---------------------------------------|------------------------------------------------------|
//! | `chrono::DateTime<Utc>`               | TIMESTAMPTZ                                          |
//! | `chrono::DateTime<Local>`             | TIMESTAMPTZ                                          |
//! | `chrono::NaiveDateTime`               | TIMESTAMP                                            |
//! | `chrono::NaiveDate`                   | DATE                                                 |
//! | `chrono::NaiveTime`                   | TIME                                                 |
//! | [`TimeTz`]                            | TIMETZ                                               |
//!
//! ### [`time`](https://crates.io/crates/time)
//!
//! Requires the `time` Cargo feature flag.
//!
//! | Rust type                             | Postgres type(s)                                     |
//! |---------------------------------------|------------------------------------------------------|
//! | `time::PrimitiveDateTime`             | TIMESTAMP                                            |
//! | `time::OffsetDateTime`                | TIMESTAMPTZ                                          |
//! | `time::Date`                          | DATE                                                 |
//! | `time::Time`                          | TIME                                                 |
//! | [`TimeTz`]                            | TIMETZ                                               |
//!
//! ### [`uuid`](https://crates.io/crates/uuid)
//!
//! Requires the `uuid` Cargo feature flag.
//!
//! | Rust type                             | Postgres type(s)                                     |
//! |---------------------------------------|------------------------------------------------------|
//! | `uuid::Uuid`                          | UUID                                                 |
//!
//! ### [`ipnetwork`](https://crates.io/crates/ipnetwork)
//!
//! Requires the `ipnetwork` Cargo feature flag (takes precedence over `ipnet` if both are used).
//!
//! | Rust type                             | Postgres type(s)                                     |
//! |---------------------------------------|------------------------------------------------------|
//! | `ipnetwork::IpNetwork`                | INET, CIDR                                           |
//! | `std::net::IpAddr`                    | INET, CIDR                                           |
//!
//! Note that because `IpAddr` does not support network prefixes, it is an error to attempt to decode
//! an `IpAddr` from a `INET` or `CIDR` value with a network prefix smaller than the address' full width:
//! `/32` for IPv4 addresses and `/128` for IPv6 addresses.
//!
//! `IpNetwork` does not have this limitation.
//!
//! ### [`ipnet`](https://crates.io/crates/ipnet)
//!
//! Requires the `ipnet` Cargo feature flag.
//!
//! | Rust type                             | Postgres type(s)                                     |
//! |---------------------------------------|------------------------------------------------------|
//! | `ipnet::IpNet`                        | INET, CIDR                                           |
//! | `std::net::IpAddr`                    | INET, CIDR                                           |
//!
//! The same `IpAddr` limitation for smaller network prefixes applies as with `ipnet`.
//!
//! ### [`mac_address`](https://crates.io/crates/mac_address)
//!
//! Requires the `mac_address` Cargo feature flag.
//!
//! | Rust type                             | Postgres type(s)                                     |
//! |---------------------------------------|------------------------------------------------------|
//! | `mac_address::MacAddress`             | MACADDR                                              |
//!
//! ### [`bit-vec`](https://crates.io/crates/bit-vec)
//!
//! Requires the `bit-vec` Cargo feature flag.
//!
//! | Rust type                             | Postgres type(s)                                     |
//! |---------------------------------------|------------------------------------------------------|
//! | `bit_vec::BitVec`                     | BIT, VARBIT                                          |
//!
//! ### [`json`](https://crates.io/crates/serde_json)
//!
//! Requires the `json` Cargo feature flag.
//!
//! | Rust type                             | Postgres type(s)                                     |
//! |---------------------------------------|------------------------------------------------------|
//! | [`Json<T>`]                           | JSON, JSONB                                          |
//! | `serde_json::Value`                   | JSON, JSONB                                          |
//! | `&serde_json::value::RawValue`        | JSON, JSONB                                          |
//!
//! `Value` and `RawValue` from `serde_json` can be used for unstructured JSON data with
//! Postgres.
//!
//! [`Json<T>`](crate::types::Json) can be used for structured JSON data with Postgres.
//!
//! # [Composite types](https://www.postgresql.org/docs/current/rowtypes.html)
//!
//! User-defined composite types are supported through a derive for `Type`.
//!
//! ```text
//! CREATE TYPE inventory_item AS (
//!     name            text,
//!     supplier_id     integer,
//!     price           numeric
//! );
//! ```
//!
//! ```rust,ignore
//! #[derive(sqlx::Type)]
//! #[sqlx(type_name = "inventory_item")]
//! struct InventoryItem {
//!     name: String,
//!     supplier_id: i32,
//!     price: BigDecimal,
//! }
//! ```
//!
//! Anonymous composite types are represented as tuples. Note that anonymous composites may only
//! be returned and not sent to Postgres (this is a limitation of postgres).
//!
//! # Arrays
//!
//! One-dimensional arrays are supported as `Vec<T>` or `&[T]` where `T` implements `Type`.
//!
//! # [Enumerations](https://www.postgresql.org/docs/current/datatype-enum.html)
//!
//! User-defined enumerations are supported through a derive for `Type`.
//!
//! ```text
//! CREATE TYPE mood AS ENUM ('sad', 'ok', 'happy');
//! ```
//!
//! ```rust,ignore
//! #[derive(sqlx::Type)]
//! #[sqlx(type_name = "mood", rename_all = "lowercase")]
//! enum Mood { Sad, Ok, Happy }
//! ```
//!
//! Rust enumerations may also be defined to be represented as an integer using `repr`.
//! The following type expects a SQL type of `INTEGER` or `INT4` and will convert to/from the
//! Rust enumeration.
//!
//! ```rust,ignore
//! #[derive(sqlx::Type)]
//! #[repr(i32)]
//! enum Mood { Sad = 0, Ok = 1, Happy = 2 }
//! ```
//!
//! Rust enumerations may also be defined to be represented as a string using `type_name = "text"`.
//! The following type expects a SQL type of `TEXT` and will convert to/from the Rust enumeration.
//!
//! ```rust,ignore
//! #[derive(sqlx::Type)]
//! #[sqlx(type_name = "text")]
//! enum Mood { Sad, Ok, Happy }
//! ```
//!
//! Note that an error can occur if you attempt to decode a value not contained within the enum
//! definition.
//!

use std::borrow::Cow;
use std::rc::Rc;
use std::sync::Arc;

mod array;
mod bool;
mod bytes;
mod citext;
mod float;
mod hstore;
mod int;
mod interval;
#[cfg(feature = "json")]
mod json;
mod lquery;
mod ltree;
mod money;
mod non_zero;
mod oid;
mod range;
mod record;
mod str;
mod text;
pub(crate) mod meta;
mod tuple;
mod void;

pub use meta::{ArrayOf, CustomType, TypeInfo, TypeKind};

#[cfg(feature = "bstr")]
pub mod bstr;

#[cfg(any(feature = "chrono", feature = "time"))]
mod time_tz;

#[cfg(feature = "bigdecimal")]
mod bigdecimal;

mod cube;

mod geometry;

#[cfg(any(feature = "bigdecimal", feature = "rust_decimal"))]
mod numeric;

#[cfg(feature = "rust_decimal")]
mod rust_decimal;

#[cfg(feature = "chrono")]
pub mod chrono;

#[cfg(feature = "time")]
pub mod time;

#[cfg(feature = "uuid")]
mod uuid;

#[cfg(feature = "ipnet")]
pub mod ipnet;

#[cfg(feature = "ipnetwork")]
pub mod ipnetwork;

#[cfg(feature = "mac_address")]
pub mod mac_address;

#[cfg(feature = "bit-vec")]
mod bit_vec;

pub use array::HasArrayType;
pub use citext::CiText;
pub use cube::Cube;
pub use geometry::circle::Circle;
pub use geometry::line::Line;
pub use geometry::line_segment::LSeg;
pub use geometry::path::Path;
pub use geometry::point::Point;
pub use geometry::polygon::Polygon;
pub use geometry::r#box::Box;
pub use hstore::Hstore;
pub use interval::Interval;
pub use lquery::LQuery;
pub use lquery::LQueryLevel;
pub use lquery::LQueryVariant;
pub use lquery::LQueryVariantFlag;
pub use ltree::LTree;
pub use ltree::LTreeLabel;
pub use ltree::LTreeParseError;
pub use money::Money;
pub use oid::Oid;
pub use range::Range;
pub use text::Text;

#[cfg(feature = "json")]
pub use json::{Json, JsonRawValue, JsonValue};

#[cfg(feature = "uuid")]
#[doc(no_inline)]
pub use ::uuid::Uuid;

#[cfg(feature = "bit-vec")]
#[doc(no_inline)]
pub use ::bit_vec::BitVec;

#[cfg(feature = "bigdecimal")]
#[doc(no_inline)]
pub use ::bigdecimal::BigDecimal;

#[cfg(feature = "rust_decimal")]
#[doc(no_inline)]
pub use ::rust_decimal::Decimal;

#[cfg(feature = "bstr")]
pub use bstr::{BStr, BString};

#[cfg(any(feature = "chrono", feature = "time"))]
pub use time_tz::TimeTz;

// used in derive(Type) for `struct`
// but the interface is not considered part of the public API
#[doc(hidden)]
pub use record::{RecordDecoder, RecordEncoder};

/// Indicates that a SQL type is supported for a database.
pub trait Type {
    /// Returns the canonical SQL type for this Rust type.
    fn type_info() -> TypeInfo;

    /// Determines if this Rust type is compatible with the given SQL type.
    fn compatible(ty: &TypeInfo) -> bool {
        Self::type_info().type_compatible(ty)
    }
}

// for references, the underlying SQL type is identical
impl<T: ?Sized + Type> Type for &'_ T {
    fn type_info() -> TypeInfo {
        <T as Type>::type_info()
    }

    fn compatible(ty: &TypeInfo) -> bool {
        <T as Type>::compatible(ty)
    }
}

// for optionals, the underlying SQL type is identical
impl<T: Type> Type for Option<T> {
    fn type_info() -> TypeInfo {
        <T as Type>::type_info()
    }

    fn compatible(ty: &TypeInfo) -> bool {
        ty.is_null() || <T as Type>::compatible(ty)
    }
}

macro_rules! impl_type_for_smartpointer {
    ($smart_pointer:ty) => {
        impl<T> Type for $smart_pointer
        where
            T: Type + ?Sized,
        {
            fn type_info() -> TypeInfo {
                <T as Type>::type_info()
            }

            fn compatible(ty: &TypeInfo) -> bool {
                <T as Type>::compatible(ty)
            }
        }
    };
}

impl_type_for_smartpointer!(Arc<T>);
impl_type_for_smartpointer!(std::boxed::Box<T>);
impl_type_for_smartpointer!(Rc<T>);

impl<T> Type for Cow<'_, T>
where
    T: Type + ToOwned + ?Sized,
{
    fn type_info() -> TypeInfo {
        <T as Type>::type_info()
    }

    fn compatible(ty: &TypeInfo) -> bool {
        <T as Type>::compatible(ty)
    }
}

// Type::compatible impl appropriate for arrays
fn array_compatible<E: Type + ?Sized>(ty: &TypeInfo) -> bool {
    // we require the declared type to be an _array_ with an
    // element type that is acceptable
    if let TypeKind::Array(element) = &ty.kind() {
        return E::compatible(element);
    }

    false
}
