#![allow(dead_code)]

use std::borrow::Cow;
use std::fmt::{self, Display, Formatter};
use std::ops::Deref;
use std::sync::Arc;

use crate::ustr::UStr;
use crate::codec::Oid;


/// Type information for a PostgreSQL type.
///
/// ### Note: Implementation of `==` ([`PartialEq::eq()`])
/// Because `==` on [`TypeInfo`]s has been used throughout the SQLx API as a synonym for type compatibility,
/// e.g. in the default impl of [`Type::compatible()`][crate::types::Type::compatible],
/// some concessions have been made in the implementation.
///
/// When comparing two `TypeInfo`s using the `==` operator ([`PartialEq::eq()`]),
/// if one was constructed with [`Self::with_oid()`] and the other with [`Self::with_name()`] or
/// [`Self::array_of()`], `==` will return `true`:
///
/// ```
/// # use sqlx::codec::{Oid, TypeInfo};
/// // Potentially surprising result, this assert will pass:
/// assert_eq!(TypeInfo::with_oid(Oid(1)), TypeInfo::with_name("definitely_not_real"));
/// ```
///
/// Since it is not possible in this case to prove the types are _not_ compatible (because
/// both `TypeInfo`s need to be resolved by an active connection to know for sure)
/// and type compatibility is mainly done as a sanity check anyway,
/// it was deemed acceptable to fudge equality in this very specific case.
///
/// This also applies when querying with the text protocol (not using prepared statements,
/// e.g. [`sqlx::raw_sql()`][crate::raw_sql::raw_sql]), as the connection will be unable
/// to look up the type info like it normally does when preparing a statement: it won't know
/// what the OIDs of the output columns will be until it's in the middle of reading the result,
/// and by that time it's too late.
///
/// To compare types for exact equality, use [`Self::type_eq()`] instead.
#[derive(Debug, Clone, PartialEq)]
pub struct TypeInfo(pub(crate) Type);

impl Deref for TypeInfo {
    type Target = Type;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

#[derive(Debug, Clone)]
#[repr(u32)]
pub enum Type {
    Bool,
    Bytea,
    Char,
    Name,
    Int8,
    Int2,
    Int4,
    Text,
    Oid,
    Json,
    JsonArray,
    Point,
    Lseg,
    Path,
    Box,
    Polygon,
    Line,
    LineArray,
    Cidr,
    CidrArray,
    Float4,
    Float8,
    Unknown,
    Circle,
    CircleArray,
    Macaddr8,
    Macaddr8Array,
    Macaddr,
    Inet,
    BoolArray,
    ByteaArray,
    CharArray,
    NameArray,
    Int2Array,
    Int4Array,
    TextArray,
    BpcharArray,
    VarcharArray,
    Int8Array,
    PointArray,
    LsegArray,
    PathArray,
    BoxArray,
    Float4Array,
    Float8Array,
    PolygonArray,
    OidArray,
    MacaddrArray,
    InetArray,
    Bpchar,
    Varchar,
    Date,
    Time,
    Timestamp,
    TimestampArray,
    DateArray,
    TimeArray,
    Timestamptz,
    TimestamptzArray,
    Interval,
    IntervalArray,
    NumericArray,
    Timetz,
    TimetzArray,
    Bit,
    BitArray,
    Varbit,
    VarbitArray,
    Numeric,
    Record,
    RecordArray,
    Uuid,
    UuidArray,
    Jsonb,
    JsonbArray,
    Int4Range,
    Int4RangeArray,
    NumRange,
    NumRangeArray,
    TsRange,
    TsRangeArray,
    TstzRange,
    TstzRangeArray,
    DateRange,
    DateRangeArray,
    Int8Range,
    Int8RangeArray,
    Jsonpath,
    JsonpathArray,
    Money,
    MoneyArray,

    // https://www.postgresql.org/docs/9.3/datatype-pseudo.html
    Void,

    // A realized user-defined type. When a connection sees a DeclareXX variant it resolves
    // into this one before passing it along to `accepts` or inside of `Value` objects.
    Custom(Arc<CustomType>),

    // From [`TypeInfo::with_name`]
    DeclareWithName(UStr),

    // NOTE: Do we want to bring back type declaration by ID? It's notoriously fragile but
    //       someone may have a user for it
    DeclareWithOid(Oid),

    DeclareArrayOf(Arc<ArrayOf>),
}

#[derive(Debug, Clone)]
pub struct CustomType {
    pub(crate) oid: Oid,
    pub(crate) name: UStr,
    pub(crate) kind: TypeKind,
}

#[derive(Debug, Clone)]
pub enum TypeKind {
    Simple,
    Pseudo,
    Domain(TypeInfo),
    Composite(Arc<[(String, TypeInfo)]>),
    Array(TypeInfo),
    Enum(Arc<[String]>),
    Range(TypeInfo),
}

#[derive(Debug, Clone)]
pub struct ArrayOf {
    pub(crate) elem_name: UStr,
    pub(crate) name: Box<str>,
}

impl TypeInfo {
    /// Returns the corresponding `TypeInfo` if the OID is a built-in type and recognized by SQLx.
    pub(crate) fn try_from_oid(oid: Oid) -> Option<Self> {
        Type::try_from_oid(oid).map(Self)
    }

    /// Returns the _kind_ (simple, array, enum, etc.) for this type.
    pub fn kind(&self) -> &TypeKind {
        self.0.kind()
    }

    /// Returns the OID for this type, if available.
    ///
    /// The OID may not be available if SQLx only knows the type by name.
    /// It will have to be resolved by a `Connection` at runtime which
    /// will yield a new and semantically distinct `TypeInfo` instance.
    ///
    /// This method does not perform any such lookup.
    ///
    /// ### Note
    /// With the exception of [the default `pg_type` catalog][pg_type], type OIDs are *not* stable in PostgreSQL.
    /// If a type is added by an extension, its OID will be assigned when the `CREATE EXTENSION` statement is executed,
    /// and so can change depending on what extensions are installed and in what order, as well as the exact
    /// version of PostgreSQL.
    ///
    /// [pg_type]: https://github.com/postgres/postgres/blob/master/src/include/catalog/pg_type.dat
    pub fn oid(&self) -> Option<Oid> {
        self.0.try_oid()
    }

    #[doc(hidden)]
    pub fn __type_feature_gate(&self) -> Option<&'static str> {
        if [
            TypeInfo::DATE,
            TypeInfo::TIME,
            TypeInfo::TIMESTAMP,
            TypeInfo::TIMESTAMPTZ,
            TypeInfo::DATE_ARRAY,
            TypeInfo::TIME_ARRAY,
            TypeInfo::TIMESTAMP_ARRAY,
            TypeInfo::TIMESTAMPTZ_ARRAY,
        ]
        .contains(self)
        {
            Some("time")
        } else if [TypeInfo::UUID, TypeInfo::UUID_ARRAY].contains(self) {
            Some("uuid")
        } else if [
            TypeInfo::JSON,
            TypeInfo::JSONB,
            TypeInfo::JSON_ARRAY,
            TypeInfo::JSONB_ARRAY,
        ]
        .contains(self)
        {
            Some("json")
        } else if [
            TypeInfo::CIDR,
            TypeInfo::INET,
            TypeInfo::CIDR_ARRAY,
            TypeInfo::INET_ARRAY,
        ]
        .contains(self)
        {
            Some("ipnetwork")
        } else if [TypeInfo::MACADDR].contains(self) {
            Some("mac_address")
        } else if [TypeInfo::NUMERIC, TypeInfo::NUMERIC_ARRAY].contains(self) {
            Some("bigdecimal")
        } else {
            None
        }
    }

    /// Create a `TypeInfo` from a type name.
    ///
    /// The OID for the type will be fetched from Postgres on use of
    /// a value of this type. The fetched OID will be cached per-connection.
    ///
    /// ### Note: Type Names Prefixed with `_`
    /// In `pg_catalog.pg_type`, Postgres prefixes a type name with `_` to denote an array of that
    /// type, e.g. `int4[]` actually exists in `pg_type` as `_int4`.
    ///
    /// Previously, it was necessary in manual [`HasArrayType`][crate::HasArrayType] impls
    /// to return [`TypeInfo::with_name()`] with the type name prefixed with `_` to denote
    /// an array type, but this would not work with schema-qualified names.
    ///
    /// As of 0.8, [`TypeInfo::array_of()`] is used to declare an array type,
    /// and the Postgres driver is now able to properly resolve arrays of custom types,
    /// even in other schemas, which was not previously supported.
    ///
    /// It is highly recommended to migrate existing usages to [`TypeInfo::array_of()`] where
    /// applicable.
    ///
    /// However, to maintain compatibility, the driver now infers any type name prefixed with `_`
    /// to be an array of that type. This may introduce some breakages for types which use
    /// a `_` prefix but which are not arrays.
    ///
    /// As a workaround, type names with `_` as a prefix but which are not arrays should be wrapped
    /// in quotes (`with_name(r#""_foo""#)`).
    pub const fn with_name(name: &'static str) -> Self {
        Self(Type::DeclareWithName(UStr::Static(name)))
    }

    /// Create a `TypeInfo` of an array from the name of its element type.
    ///
    /// The array type OID will be fetched from Postgres on use of a value of this type.
    /// The fetched OID will be cached per-connection.
    pub fn array_of(elem_name: &'static str) -> Self {
        // to satisfy `name()` and `display_name()`, we need to construct strings to return
        Self(Type::DeclareArrayOf(Arc::new(ArrayOf {
            elem_name: elem_name.into(),
            name: format!("{elem_name}[]").into(),
        })))
    }

    /// Create a `TypeInfo` from an OID.
    ///
    /// Note that the OID for a type is very dependent on the environment. If you only ever use
    /// one database or if this is an unhandled built-in type, you should be fine. Otherwise,
    /// you will be better served using [`Self::with_name()`].
    ///
    /// ### Note: Interaction with `==`
    /// This constructor may give surprising results with `==`.
    ///
    /// See [the type-level docs][Self] for details.
    pub const fn with_oid(oid: Oid) -> Self {
        Self(Type::DeclareWithOid(oid))
    }

    /// Returns `true` if `self` can be compared exactly to `other`.
    ///
    /// Unlike `==`, this will return false if
    pub fn type_eq(&self, other: &Self) -> bool {
        self.eq_impl(other, false)
    }
}

// DEVELOPER PRO TIP: find builtin type OIDs easily by grepping this file
// https://github.com/postgres/postgres/blob/master/src/include/catalog/pg_type.dat
//
// If you have Postgres running locally you can also try
// SELECT oid, typarray FROM pg_type where typname = '<type name>'

impl Type {
    /// Returns the corresponding `Type` if the OID is a built-in type and recognized by SQLx.
    pub(crate) fn try_from_oid(oid: Oid) -> Option<Self> {
        Some(match oid.0 {
            16 => Type::Bool,
            17 => Type::Bytea,
            18 => Type::Char,
            19 => Type::Name,
            20 => Type::Int8,
            21 => Type::Int2,
            23 => Type::Int4,
            25 => Type::Text,
            26 => Type::Oid,
            114 => Type::Json,
            199 => Type::JsonArray,
            600 => Type::Point,
            601 => Type::Lseg,
            602 => Type::Path,
            603 => Type::Box,
            604 => Type::Polygon,
            628 => Type::Line,
            629 => Type::LineArray,
            650 => Type::Cidr,
            651 => Type::CidrArray,
            700 => Type::Float4,
            701 => Type::Float8,
            705 => Type::Unknown,
            718 => Type::Circle,
            719 => Type::CircleArray,
            774 => Type::Macaddr8,
            775 => Type::Macaddr8Array,
            790 => Type::Money,
            791 => Type::MoneyArray,
            829 => Type::Macaddr,
            869 => Type::Inet,
            1000 => Type::BoolArray,
            1001 => Type::ByteaArray,
            1002 => Type::CharArray,
            1003 => Type::NameArray,
            1005 => Type::Int2Array,
            1007 => Type::Int4Array,
            1009 => Type::TextArray,
            1014 => Type::BpcharArray,
            1015 => Type::VarcharArray,
            1016 => Type::Int8Array,
            1017 => Type::PointArray,
            1018 => Type::LsegArray,
            1019 => Type::PathArray,
            1020 => Type::BoxArray,
            1021 => Type::Float4Array,
            1022 => Type::Float8Array,
            1027 => Type::PolygonArray,
            1028 => Type::OidArray,
            1040 => Type::MacaddrArray,
            1041 => Type::InetArray,
            1042 => Type::Bpchar,
            1043 => Type::Varchar,
            1082 => Type::Date,
            1083 => Type::Time,
            1114 => Type::Timestamp,
            1115 => Type::TimestampArray,
            1182 => Type::DateArray,
            1183 => Type::TimeArray,
            1184 => Type::Timestamptz,
            1185 => Type::TimestamptzArray,
            1186 => Type::Interval,
            1187 => Type::IntervalArray,
            1231 => Type::NumericArray,
            1266 => Type::Timetz,
            1270 => Type::TimetzArray,
            1560 => Type::Bit,
            1561 => Type::BitArray,
            1562 => Type::Varbit,
            1563 => Type::VarbitArray,
            1700 => Type::Numeric,
            2278 => Type::Void,
            2249 => Type::Record,
            2287 => Type::RecordArray,
            2950 => Type::Uuid,
            2951 => Type::UuidArray,
            3802 => Type::Jsonb,
            3807 => Type::JsonbArray,
            3904 => Type::Int4Range,
            3905 => Type::Int4RangeArray,
            3906 => Type::NumRange,
            3907 => Type::NumRangeArray,
            3908 => Type::TsRange,
            3909 => Type::TsRangeArray,
            3910 => Type::TstzRange,
            3911 => Type::TstzRangeArray,
            3912 => Type::DateRange,
            3913 => Type::DateRangeArray,
            3926 => Type::Int8Range,
            3927 => Type::Int8RangeArray,
            4072 => Type::Jsonpath,
            4073 => Type::JsonpathArray,

            _ => {
                return None;
            }
        })
    }

    pub(crate) fn oid(&self) -> Oid {
        match self.try_oid() {
            Some(oid) => oid,
            None => unreachable!("(bug) use of unresolved type declaration [oid]"),
        }
    }

    pub(crate) fn try_oid(&self) -> Option<Oid> {
        Some(match self {
            Type::Bool => Oid(16),
            Type::Bytea => Oid(17),
            Type::Char => Oid(18),
            Type::Name => Oid(19),
            Type::Int8 => Oid(20),
            Type::Int2 => Oid(21),
            Type::Int4 => Oid(23),
            Type::Text => Oid(25),
            Type::Oid => Oid(26),
            Type::Json => Oid(114),
            Type::JsonArray => Oid(199),
            Type::Point => Oid(600),
            Type::Lseg => Oid(601),
            Type::Path => Oid(602),
            Type::Box => Oid(603),
            Type::Polygon => Oid(604),
            Type::Line => Oid(628),
            Type::LineArray => Oid(629),
            Type::Cidr => Oid(650),
            Type::CidrArray => Oid(651),
            Type::Float4 => Oid(700),
            Type::Float8 => Oid(701),
            Type::Unknown => Oid(705),
            Type::Circle => Oid(718),
            Type::CircleArray => Oid(719),
            Type::Macaddr8 => Oid(774),
            Type::Macaddr8Array => Oid(775),
            Type::Money => Oid(790),
            Type::MoneyArray => Oid(791),
            Type::Macaddr => Oid(829),
            Type::Inet => Oid(869),
            Type::BoolArray => Oid(1000),
            Type::ByteaArray => Oid(1001),
            Type::CharArray => Oid(1002),
            Type::NameArray => Oid(1003),
            Type::Int2Array => Oid(1005),
            Type::Int4Array => Oid(1007),
            Type::TextArray => Oid(1009),
            Type::BpcharArray => Oid(1014),
            Type::VarcharArray => Oid(1015),
            Type::Int8Array => Oid(1016),
            Type::PointArray => Oid(1017),
            Type::LsegArray => Oid(1018),
            Type::PathArray => Oid(1019),
            Type::BoxArray => Oid(1020),
            Type::Float4Array => Oid(1021),
            Type::Float8Array => Oid(1022),
            Type::PolygonArray => Oid(1027),
            Type::OidArray => Oid(1028),
            Type::MacaddrArray => Oid(1040),
            Type::InetArray => Oid(1041),
            Type::Bpchar => Oid(1042),
            Type::Varchar => Oid(1043),
            Type::Date => Oid(1082),
            Type::Time => Oid(1083),
            Type::Timestamp => Oid(1114),
            Type::TimestampArray => Oid(1115),
            Type::DateArray => Oid(1182),
            Type::TimeArray => Oid(1183),
            Type::Timestamptz => Oid(1184),
            Type::TimestamptzArray => Oid(1185),
            Type::Interval => Oid(1186),
            Type::IntervalArray => Oid(1187),
            Type::NumericArray => Oid(1231),
            Type::Timetz => Oid(1266),
            Type::TimetzArray => Oid(1270),
            Type::Bit => Oid(1560),
            Type::BitArray => Oid(1561),
            Type::Varbit => Oid(1562),
            Type::VarbitArray => Oid(1563),
            Type::Numeric => Oid(1700),
            Type::Void => Oid(2278),
            Type::Record => Oid(2249),
            Type::RecordArray => Oid(2287),
            Type::Uuid => Oid(2950),
            Type::UuidArray => Oid(2951),
            Type::Jsonb => Oid(3802),
            Type::JsonbArray => Oid(3807),
            Type::Int4Range => Oid(3904),
            Type::Int4RangeArray => Oid(3905),
            Type::NumRange => Oid(3906),
            Type::NumRangeArray => Oid(3907),
            Type::TsRange => Oid(3908),
            Type::TsRangeArray => Oid(3909),
            Type::TstzRange => Oid(3910),
            Type::TstzRangeArray => Oid(3911),
            Type::DateRange => Oid(3912),
            Type::DateRangeArray => Oid(3913),
            Type::Int8Range => Oid(3926),
            Type::Int8RangeArray => Oid(3927),
            Type::Jsonpath => Oid(4072),
            Type::JsonpathArray => Oid(4073),

            Type::Custom(ty) => ty.oid,

            Type::DeclareWithOid(oid) => *oid,
            Type::DeclareWithName(_) => {
                return None;
            }
            Type::DeclareArrayOf(_) => {
                return None;
            }
        })
    }

    pub(crate) fn display_name(&self) -> &str {
        match self {
            Type::Bool => "BOOL",
            Type::Bytea => "BYTEA",
            Type::Char => "\"CHAR\"",
            Type::Name => "NAME",
            Type::Int8 => "INT8",
            Type::Int2 => "INT2",
            Type::Int4 => "INT4",
            Type::Text => "TEXT",
            Type::Oid => "OID",
            Type::Json => "JSON",
            Type::JsonArray => "JSON[]",
            Type::Point => "POINT",
            Type::Lseg => "LSEG",
            Type::Path => "PATH",
            Type::Box => "BOX",
            Type::Polygon => "POLYGON",
            Type::Line => "LINE",
            Type::LineArray => "LINE[]",
            Type::Cidr => "CIDR",
            Type::CidrArray => "CIDR[]",
            Type::Float4 => "FLOAT4",
            Type::Float8 => "FLOAT8",
            Type::Unknown => "UNKNOWN",
            Type::Circle => "CIRCLE",
            Type::CircleArray => "CIRCLE[]",
            Type::Macaddr8 => "MACADDR8",
            Type::Macaddr8Array => "MACADDR8[]",
            Type::Macaddr => "MACADDR",
            Type::Inet => "INET",
            Type::BoolArray => "BOOL[]",
            Type::ByteaArray => "BYTEA[]",
            Type::CharArray => "\"CHAR\"[]",
            Type::NameArray => "NAME[]",
            Type::Int2Array => "INT2[]",
            Type::Int4Array => "INT4[]",
            Type::TextArray => "TEXT[]",
            Type::BpcharArray => "CHAR[]",
            Type::VarcharArray => "VARCHAR[]",
            Type::Int8Array => "INT8[]",
            Type::PointArray => "POINT[]",
            Type::LsegArray => "LSEG[]",
            Type::PathArray => "PATH[]",
            Type::BoxArray => "BOX[]",
            Type::Float4Array => "FLOAT4[]",
            Type::Float8Array => "FLOAT8[]",
            Type::PolygonArray => "POLYGON[]",
            Type::OidArray => "OID[]",
            Type::MacaddrArray => "MACADDR[]",
            Type::InetArray => "INET[]",
            Type::Bpchar => "CHAR",
            Type::Varchar => "VARCHAR",
            Type::Date => "DATE",
            Type::Time => "TIME",
            Type::Timestamp => "TIMESTAMP",
            Type::TimestampArray => "TIMESTAMP[]",
            Type::DateArray => "DATE[]",
            Type::TimeArray => "TIME[]",
            Type::Timestamptz => "TIMESTAMPTZ",
            Type::TimestamptzArray => "TIMESTAMPTZ[]",
            Type::Interval => "INTERVAL",
            Type::IntervalArray => "INTERVAL[]",
            Type::NumericArray => "NUMERIC[]",
            Type::Timetz => "TIMETZ",
            Type::TimetzArray => "TIMETZ[]",
            Type::Bit => "BIT",
            Type::BitArray => "BIT[]",
            Type::Varbit => "VARBIT",
            Type::VarbitArray => "VARBIT[]",
            Type::Numeric => "NUMERIC",
            Type::Record => "RECORD",
            Type::RecordArray => "RECORD[]",
            Type::Uuid => "UUID",
            Type::UuidArray => "UUID[]",
            Type::Jsonb => "JSONB",
            Type::JsonbArray => "JSONB[]",
            Type::Int4Range => "INT4RANGE",
            Type::Int4RangeArray => "INT4RANGE[]",
            Type::NumRange => "NUMRANGE",
            Type::NumRangeArray => "NUMRANGE[]",
            Type::TsRange => "TSRANGE",
            Type::TsRangeArray => "TSRANGE[]",
            Type::TstzRange => "TSTZRANGE",
            Type::TstzRangeArray => "TSTZRANGE[]",
            Type::DateRange => "DATERANGE",
            Type::DateRangeArray => "DATERANGE[]",
            Type::Int8Range => "INT8RANGE",
            Type::Int8RangeArray => "INT8RANGE[]",
            Type::Jsonpath => "JSONPATH",
            Type::JsonpathArray => "JSONPATH[]",
            Type::Money => "MONEY",
            Type::MoneyArray => "MONEY[]",
            Type::Void => "VOID",
            Type::Custom(ty) => &ty.name,
            Type::DeclareWithOid(_) => "?",
            Type::DeclareWithName(name) => name,
            Type::DeclareArrayOf(array) => &array.name,
        }
    }

    pub(crate) fn name(&self) -> &str {
        match self {
            Type::Bool => "bool",
            Type::Bytea => "bytea",
            Type::Char => "char",
            Type::Name => "name",
            Type::Int8 => "int8",
            Type::Int2 => "int2",
            Type::Int4 => "int4",
            Type::Text => "text",
            Type::Oid => "oid",
            Type::Json => "json",
            Type::JsonArray => "_json",
            Type::Point => "point",
            Type::Lseg => "lseg",
            Type::Path => "path",
            Type::Box => "box",
            Type::Polygon => "polygon",
            Type::Line => "line",
            Type::LineArray => "_line",
            Type::Cidr => "cidr",
            Type::CidrArray => "_cidr",
            Type::Float4 => "float4",
            Type::Float8 => "float8",
            Type::Unknown => "unknown",
            Type::Circle => "circle",
            Type::CircleArray => "_circle",
            Type::Macaddr8 => "macaddr8",
            Type::Macaddr8Array => "_macaddr8",
            Type::Macaddr => "macaddr",
            Type::Inet => "inet",
            Type::BoolArray => "_bool",
            Type::ByteaArray => "_bytea",
            Type::CharArray => "_char",
            Type::NameArray => "_name",
            Type::Int2Array => "_int2",
            Type::Int4Array => "_int4",
            Type::TextArray => "_text",
            Type::BpcharArray => "_bpchar",
            Type::VarcharArray => "_varchar",
            Type::Int8Array => "_int8",
            Type::PointArray => "_point",
            Type::LsegArray => "_lseg",
            Type::PathArray => "_path",
            Type::BoxArray => "_box",
            Type::Float4Array => "_float4",
            Type::Float8Array => "_float8",
            Type::PolygonArray => "_polygon",
            Type::OidArray => "_oid",
            Type::MacaddrArray => "_macaddr",
            Type::InetArray => "_inet",
            Type::Bpchar => "bpchar",
            Type::Varchar => "varchar",
            Type::Date => "date",
            Type::Time => "time",
            Type::Timestamp => "timestamp",
            Type::TimestampArray => "_timestamp",
            Type::DateArray => "_date",
            Type::TimeArray => "_time",
            Type::Timestamptz => "timestamptz",
            Type::TimestamptzArray => "_timestamptz",
            Type::Interval => "interval",
            Type::IntervalArray => "_interval",
            Type::NumericArray => "_numeric",
            Type::Timetz => "timetz",
            Type::TimetzArray => "_timetz",
            Type::Bit => "bit",
            Type::BitArray => "_bit",
            Type::Varbit => "varbit",
            Type::VarbitArray => "_varbit",
            Type::Numeric => "numeric",
            Type::Record => "record",
            Type::RecordArray => "_record",
            Type::Uuid => "uuid",
            Type::UuidArray => "_uuid",
            Type::Jsonb => "jsonb",
            Type::JsonbArray => "_jsonb",
            Type::Int4Range => "int4range",
            Type::Int4RangeArray => "_int4range",
            Type::NumRange => "numrange",
            Type::NumRangeArray => "_numrange",
            Type::TsRange => "tsrange",
            Type::TsRangeArray => "_tsrange",
            Type::TstzRange => "tstzrange",
            Type::TstzRangeArray => "_tstzrange",
            Type::DateRange => "daterange",
            Type::DateRangeArray => "_daterange",
            Type::Int8Range => "int8range",
            Type::Int8RangeArray => "_int8range",
            Type::Jsonpath => "jsonpath",
            Type::JsonpathArray => "_jsonpath",
            Type::Money => "money",
            Type::MoneyArray => "_money",
            Type::Void => "void",
            Type::Custom(ty) => &ty.name,
            Type::DeclareWithOid(_) => "?",
            Type::DeclareWithName(name) => name,
            Type::DeclareArrayOf(array) => &array.name,
        }
    }

    pub(crate) fn kind(&self) -> &TypeKind {
        match self {
            Type::Bool => &TypeKind::Simple,
            Type::Bytea => &TypeKind::Simple,
            Type::Char => &TypeKind::Simple,
            Type::Name => &TypeKind::Simple,
            Type::Int8 => &TypeKind::Simple,
            Type::Int2 => &TypeKind::Simple,
            Type::Int4 => &TypeKind::Simple,
            Type::Text => &TypeKind::Simple,
            Type::Oid => &TypeKind::Simple,
            Type::Json => &TypeKind::Simple,
            Type::JsonArray => &TypeKind::Array(TypeInfo(Type::Json)),
            Type::Point => &TypeKind::Simple,
            Type::Lseg => &TypeKind::Simple,
            Type::Path => &TypeKind::Simple,
            Type::Box => &TypeKind::Simple,
            Type::Polygon => &TypeKind::Simple,
            Type::Line => &TypeKind::Simple,
            Type::LineArray => &TypeKind::Array(TypeInfo(Type::Line)),
            Type::Cidr => &TypeKind::Simple,
            Type::CidrArray => &TypeKind::Array(TypeInfo(Type::Cidr)),
            Type::Float4 => &TypeKind::Simple,
            Type::Float8 => &TypeKind::Simple,
            Type::Unknown => &TypeKind::Simple,
            Type::Circle => &TypeKind::Simple,
            Type::CircleArray => &TypeKind::Array(TypeInfo(Type::Circle)),
            Type::Macaddr8 => &TypeKind::Simple,
            Type::Macaddr8Array => &TypeKind::Array(TypeInfo(Type::Macaddr8)),
            Type::Macaddr => &TypeKind::Simple,
            Type::Inet => &TypeKind::Simple,
            Type::BoolArray => &TypeKind::Array(TypeInfo(Type::Bool)),
            Type::ByteaArray => &TypeKind::Array(TypeInfo(Type::Bytea)),
            Type::CharArray => &TypeKind::Array(TypeInfo(Type::Char)),
            Type::NameArray => &TypeKind::Array(TypeInfo(Type::Name)),
            Type::Int2Array => &TypeKind::Array(TypeInfo(Type::Int2)),
            Type::Int4Array => &TypeKind::Array(TypeInfo(Type::Int4)),
            Type::TextArray => &TypeKind::Array(TypeInfo(Type::Text)),
            Type::BpcharArray => &TypeKind::Array(TypeInfo(Type::Bpchar)),
            Type::VarcharArray => &TypeKind::Array(TypeInfo(Type::Varchar)),
            Type::Int8Array => &TypeKind::Array(TypeInfo(Type::Int8)),
            Type::PointArray => &TypeKind::Array(TypeInfo(Type::Point)),
            Type::LsegArray => &TypeKind::Array(TypeInfo(Type::Lseg)),
            Type::PathArray => &TypeKind::Array(TypeInfo(Type::Path)),
            Type::BoxArray => &TypeKind::Array(TypeInfo(Type::Box)),
            Type::Float4Array => &TypeKind::Array(TypeInfo(Type::Float4)),
            Type::Float8Array => &TypeKind::Array(TypeInfo(Type::Float8)),
            Type::PolygonArray => &TypeKind::Array(TypeInfo(Type::Polygon)),
            Type::OidArray => &TypeKind::Array(TypeInfo(Type::Oid)),
            Type::MacaddrArray => &TypeKind::Array(TypeInfo(Type::Macaddr)),
            Type::InetArray => &TypeKind::Array(TypeInfo(Type::Inet)),
            Type::Bpchar => &TypeKind::Simple,
            Type::Varchar => &TypeKind::Simple,
            Type::Date => &TypeKind::Simple,
            Type::Time => &TypeKind::Simple,
            Type::Timestamp => &TypeKind::Simple,
            Type::TimestampArray => &TypeKind::Array(TypeInfo(Type::Timestamp)),
            Type::DateArray => &TypeKind::Array(TypeInfo(Type::Date)),
            Type::TimeArray => &TypeKind::Array(TypeInfo(Type::Time)),
            Type::Timestamptz => &TypeKind::Simple,
            Type::TimestamptzArray => &TypeKind::Array(TypeInfo(Type::Timestamptz)),
            Type::Interval => &TypeKind::Simple,
            Type::IntervalArray => &TypeKind::Array(TypeInfo(Type::Interval)),
            Type::NumericArray => &TypeKind::Array(TypeInfo(Type::Numeric)),
            Type::Timetz => &TypeKind::Simple,
            Type::TimetzArray => &TypeKind::Array(TypeInfo(Type::Timetz)),
            Type::Bit => &TypeKind::Simple,
            Type::BitArray => &TypeKind::Array(TypeInfo(Type::Bit)),
            Type::Varbit => &TypeKind::Simple,
            Type::VarbitArray => &TypeKind::Array(TypeInfo(Type::Varbit)),
            Type::Numeric => &TypeKind::Simple,
            Type::Record => &TypeKind::Simple,
            Type::RecordArray => &TypeKind::Array(TypeInfo(Type::Record)),
            Type::Uuid => &TypeKind::Simple,
            Type::UuidArray => &TypeKind::Array(TypeInfo(Type::Uuid)),
            Type::Jsonb => &TypeKind::Simple,
            Type::JsonbArray => &TypeKind::Array(TypeInfo(Type::Jsonb)),
            Type::Int4Range => &TypeKind::Range(TypeInfo::INT4),
            Type::Int4RangeArray => &TypeKind::Array(TypeInfo(Type::Int4Range)),
            Type::NumRange => &TypeKind::Range(TypeInfo::NUMERIC),
            Type::NumRangeArray => &TypeKind::Array(TypeInfo(Type::NumRange)),
            Type::TsRange => &TypeKind::Range(TypeInfo::TIMESTAMP),
            Type::TsRangeArray => &TypeKind::Array(TypeInfo(Type::TsRange)),
            Type::TstzRange => &TypeKind::Range(TypeInfo::TIMESTAMPTZ),
            Type::TstzRangeArray => &TypeKind::Array(TypeInfo(Type::TstzRange)),
            Type::DateRange => &TypeKind::Range(TypeInfo::DATE),
            Type::DateRangeArray => &TypeKind::Array(TypeInfo(Type::DateRange)),
            Type::Int8Range => &TypeKind::Range(TypeInfo::INT8),
            Type::Int8RangeArray => &TypeKind::Array(TypeInfo(Type::Int8Range)),
            Type::Jsonpath => &TypeKind::Simple,
            Type::JsonpathArray => &TypeKind::Array(TypeInfo(Type::Jsonpath)),
            Type::Money => &TypeKind::Simple,
            Type::MoneyArray => &TypeKind::Array(TypeInfo(Type::Money)),

            Type::Void => &TypeKind::Pseudo,

            Type::Custom(ty) => &ty.kind,

            Type::DeclareWithOid(oid) => {
                unreachable!("(bug) use of unresolved type declaration [oid={}]", oid.0);
            }
            Type::DeclareWithName(name) => {
                unreachable!("(bug) use of unresolved type declaration [name={name}]");
            }
            Type::DeclareArrayOf(array) => {
                unreachable!(
                    "(bug) use of unresolved type declaration [array of={}]",
                    array.elem_name
                );
            }
        }
    }

    /// If `self` is an array type, return the type info for its element.
    pub(crate) fn try_array_element(&self) -> Option<Cow<'_, TypeInfo>> {
        // We explicitly match on all the `None` cases to ensure an exhaustive match.
        match self {
            Type::Bool => None,
            Type::BoolArray => Some(Cow::Owned(TypeInfo(Type::Bool))),
            Type::Bytea => None,
            Type::ByteaArray => Some(Cow::Owned(TypeInfo(Type::Bytea))),
            Type::Char => None,
            Type::CharArray => Some(Cow::Owned(TypeInfo(Type::Char))),
            Type::Name => None,
            Type::NameArray => Some(Cow::Owned(TypeInfo(Type::Name))),
            Type::Int8 => None,
            Type::Int8Array => Some(Cow::Owned(TypeInfo(Type::Int8))),
            Type::Int2 => None,
            Type::Int2Array => Some(Cow::Owned(TypeInfo(Type::Int2))),
            Type::Int4 => None,
            Type::Int4Array => Some(Cow::Owned(TypeInfo(Type::Int4))),
            Type::Text => None,
            Type::TextArray => Some(Cow::Owned(TypeInfo(Type::Text))),
            Type::Oid => None,
            Type::OidArray => Some(Cow::Owned(TypeInfo(Type::Oid))),
            Type::Json => None,
            Type::JsonArray => Some(Cow::Owned(TypeInfo(Type::Json))),
            Type::Point => None,
            Type::PointArray => Some(Cow::Owned(TypeInfo(Type::Point))),
            Type::Lseg => None,
            Type::LsegArray => Some(Cow::Owned(TypeInfo(Type::Lseg))),
            Type::Path => None,
            Type::PathArray => Some(Cow::Owned(TypeInfo(Type::Path))),
            Type::Box => None,
            Type::BoxArray => Some(Cow::Owned(TypeInfo(Type::Box))),
            Type::Polygon => None,
            Type::PolygonArray => Some(Cow::Owned(TypeInfo(Type::Polygon))),
            Type::Line => None,
            Type::LineArray => Some(Cow::Owned(TypeInfo(Type::Line))),
            Type::Cidr => None,
            Type::CidrArray => Some(Cow::Owned(TypeInfo(Type::Cidr))),
            Type::Float4 => None,
            Type::Float4Array => Some(Cow::Owned(TypeInfo(Type::Float4))),
            Type::Float8 => None,
            Type::Float8Array => Some(Cow::Owned(TypeInfo(Type::Float8))),
            Type::Circle => None,
            Type::CircleArray => Some(Cow::Owned(TypeInfo(Type::Circle))),
            Type::Macaddr8 => None,
            Type::Macaddr8Array => Some(Cow::Owned(TypeInfo(Type::Macaddr8))),
            Type::Money => None,
            Type::MoneyArray => Some(Cow::Owned(TypeInfo(Type::Money))),
            Type::Macaddr => None,
            Type::MacaddrArray => Some(Cow::Owned(TypeInfo(Type::Macaddr))),
            Type::Inet => None,
            Type::InetArray => Some(Cow::Owned(TypeInfo(Type::Inet))),
            Type::Bpchar => None,
            Type::BpcharArray => Some(Cow::Owned(TypeInfo(Type::Bpchar))),
            Type::Varchar => None,
            Type::VarcharArray => Some(Cow::Owned(TypeInfo(Type::Varchar))),
            Type::Date => None,
            Type::DateArray => Some(Cow::Owned(TypeInfo(Type::Date))),
            Type::Time => None,
            Type::TimeArray => Some(Cow::Owned(TypeInfo(Type::Time))),
            Type::Timestamp => None,
            Type::TimestampArray => Some(Cow::Owned(TypeInfo(Type::Timestamp))),
            Type::Timestamptz => None,
            Type::TimestamptzArray => Some(Cow::Owned(TypeInfo(Type::Timestamptz))),
            Type::Interval => None,
            Type::IntervalArray => Some(Cow::Owned(TypeInfo(Type::Interval))),
            Type::Timetz => None,
            Type::TimetzArray => Some(Cow::Owned(TypeInfo(Type::Timetz))),
            Type::Bit => None,
            Type::BitArray => Some(Cow::Owned(TypeInfo(Type::Bit))),
            Type::Varbit => None,
            Type::VarbitArray => Some(Cow::Owned(TypeInfo(Type::Varbit))),
            Type::Numeric => None,
            Type::NumericArray => Some(Cow::Owned(TypeInfo(Type::Numeric))),
            Type::Record => None,
            Type::RecordArray => Some(Cow::Owned(TypeInfo(Type::Record))),
            Type::Uuid => None,
            Type::UuidArray => Some(Cow::Owned(TypeInfo(Type::Uuid))),
            Type::Jsonb => None,
            Type::JsonbArray => Some(Cow::Owned(TypeInfo(Type::Jsonb))),
            Type::Int4Range => None,
            Type::Int4RangeArray => Some(Cow::Owned(TypeInfo(Type::Int4Range))),
            Type::NumRange => None,
            Type::NumRangeArray => Some(Cow::Owned(TypeInfo(Type::NumRange))),
            Type::TsRange => None,
            Type::TsRangeArray => Some(Cow::Owned(TypeInfo(Type::TsRange))),
            Type::TstzRange => None,
            Type::TstzRangeArray => Some(Cow::Owned(TypeInfo(Type::TstzRange))),
            Type::DateRange => None,
            Type::DateRangeArray => Some(Cow::Owned(TypeInfo(Type::DateRange))),
            Type::Int8Range => None,
            Type::Int8RangeArray => Some(Cow::Owned(TypeInfo(Type::Int8Range))),
            Type::Jsonpath => None,
            Type::JsonpathArray => Some(Cow::Owned(TypeInfo(Type::Jsonpath))),
            // There is no `UnknownArray`
            Type::Unknown => None,
            // There is no `VoidArray`
            Type::Void => None,

            Type::Custom(ty) => match &ty.kind {
                TypeKind::Simple => None,
                TypeKind::Pseudo => None,
                TypeKind::Domain(_) => None,
                TypeKind::Composite(_) => None,
                TypeKind::Array(ref elem_type_info) => Some(Cow::Borrowed(elem_type_info)),
                TypeKind::Enum(_) => None,
                TypeKind::Range(_) => None,
            },
            Type::DeclareWithOid(_) => None,
            Type::DeclareWithName(name) => {
                // LEGACY: infer the array element name from a `_` prefix
                UStr::strip_prefix(name, "_")
                    .map(|elem| Cow::Owned(TypeInfo(Type::DeclareWithName(elem))))
            }
            Type::DeclareArrayOf(array) => Some(Cow::Owned(TypeInfo(Type::DeclareWithName(
                array.elem_name.clone(),
            )))),
        }
    }

    /// Returns `true` if this type cannot be matched by name.
    fn is_declare_with_oid(&self) -> bool {
        matches!(self, Self::DeclareWithOid(_))
    }

    /// Compare two `Type`s, first by OID, then by array element, then by name.
    ///
    /// If `soft_eq` is true and `self` or `other` is `DeclareWithOid` but not both, return `true`
    /// before checking names.
    fn eq_impl(&self, other: &Self, soft_eq: bool) -> bool {
        if let (Some(a), Some(b)) = (self.try_base_oid(), other.try_base_oid()) {
            // If there are OIDs available, use OIDs to perform a direct match
            return a == b;
        }

        if soft_eq && (self.is_declare_with_oid() || other.is_declare_with_oid()) {
            // If we get to this point, one instance is `DeclareWithOid()` and the other is
            // `DeclareArrayOf()` or `DeclareWithName()`, which means we can't compare the two.
            //
            // Since this is only likely to occur when using the text protocol where we can't
            // resolve type names before executing a query, we can just opt out of typechecking.
            return true;
        }

        if let (Some(elem_a), Some(elem_b)) = (self.try_array_element(), other.try_array_element())
        {
            return elem_a == elem_b;
        }

        // Otherwise, perform a match on the name
        name_eq(self.name(), other.name())
    }

    // Tries to return the OID of the type, returns the OID of the base_type for domain types
    #[inline(always)]
    fn try_base_oid(&self) -> Option<Oid> {
        match self {
            Type::Custom(custom) => match &custom.kind {
                TypeKind::Domain(domain) => domain.try_oid(),
                _ => Some(custom.oid),
            },
            ty => ty.try_oid(),
        }
    }
}

impl TypeInfo {
    pub fn name(&self) -> &str {
        self.0.display_name()
    }

    pub fn is_null(&self) -> bool {
        false
    }

    #[doc(hidden)]
    pub fn is_void(&self) -> bool {
        matches!(self.0, Type::Void)
    }

    pub fn type_compatible(&self, other: &Self) -> bool {
        self == other
    }
}

impl PartialEq<CustomType> for CustomType {
    fn eq(&self, other: &CustomType) -> bool {
        other.oid == self.oid
    }
}

impl TypeInfo {
    // boolean, state of true or false
    pub(crate) const BOOL: Self = Self(Type::Bool);
    pub(crate) const BOOL_ARRAY: Self = Self(Type::BoolArray);

    // binary data types, variable-length binary string
    pub(crate) const BYTEA: Self = Self(Type::Bytea);
    pub(crate) const BYTEA_ARRAY: Self = Self(Type::ByteaArray);

    // uuid
    pub(crate) const UUID: Self = Self(Type::Uuid);
    pub(crate) const UUID_ARRAY: Self = Self(Type::UuidArray);

    // record
    pub(crate) const RECORD: Self = Self(Type::Record);
    pub(crate) const RECORD_ARRAY: Self = Self(Type::RecordArray);

    //
    // JSON types
    // https://www.postgresql.org/docs/current/datatype-json.html
    //

    pub(crate) const JSON: Self = Self(Type::Json);
    pub(crate) const JSON_ARRAY: Self = Self(Type::JsonArray);

    pub(crate) const JSONB: Self = Self(Type::Jsonb);
    pub(crate) const JSONB_ARRAY: Self = Self(Type::JsonbArray);

    pub(crate) const JSONPATH: Self = Self(Type::Jsonpath);
    pub(crate) const JSONPATH_ARRAY: Self = Self(Type::JsonpathArray);

    //
    // network address types
    // https://www.postgresql.org/docs/current/datatype-net-types.html
    //

    pub(crate) const CIDR: Self = Self(Type::Cidr);
    pub(crate) const CIDR_ARRAY: Self = Self(Type::CidrArray);

    pub(crate) const INET: Self = Self(Type::Inet);
    pub(crate) const INET_ARRAY: Self = Self(Type::InetArray);

    pub(crate) const MACADDR: Self = Self(Type::Macaddr);
    pub(crate) const MACADDR_ARRAY: Self = Self(Type::MacaddrArray);

    pub(crate) const MACADDR8: Self = Self(Type::Macaddr8);
    pub(crate) const MACADDR8_ARRAY: Self = Self(Type::Macaddr8Array);

    //
    // character types
    // https://www.postgresql.org/docs/current/datatype-character.html
    //

    // internal type for object names
    pub(crate) const NAME: Self = Self(Type::Name);
    pub(crate) const NAME_ARRAY: Self = Self(Type::NameArray);

    // character type, fixed-length, blank-padded
    pub(crate) const BPCHAR: Self = Self(Type::Bpchar);
    pub(crate) const BPCHAR_ARRAY: Self = Self(Type::BpcharArray);

    // character type, variable-length with limit
    pub(crate) const VARCHAR: Self = Self(Type::Varchar);
    pub(crate) const VARCHAR_ARRAY: Self = Self(Type::VarcharArray);

    // character type, variable-length
    pub(crate) const TEXT: Self = Self(Type::Text);
    pub(crate) const TEXT_ARRAY: Self = Self(Type::TextArray);

    // unknown type, transmitted as text
    pub(crate) const UNKNOWN: Self = Self(Type::Unknown);

    //
    // numeric types
    // https://www.postgresql.org/docs/current/datatype-numeric.html
    //

    // single-byte internal type
    pub(crate) const CHAR: Self = Self(Type::Char);
    pub(crate) const CHAR_ARRAY: Self = Self(Type::CharArray);

    // internal type for type ids
    pub(crate) const OID: Self = Self(Type::Oid);
    pub(crate) const OID_ARRAY: Self = Self(Type::OidArray);

    // small-range integer; -32768 to +32767
    pub(crate) const INT2: Self = Self(Type::Int2);
    pub(crate) const INT2_ARRAY: Self = Self(Type::Int2Array);

    // typical choice for integer; -2147483648 to +2147483647
    pub(crate) const INT4: Self = Self(Type::Int4);
    pub(crate) const INT4_ARRAY: Self = Self(Type::Int4Array);

    // large-range integer; -9223372036854775808 to +9223372036854775807
    pub(crate) const INT8: Self = Self(Type::Int8);
    pub(crate) const INT8_ARRAY: Self = Self(Type::Int8Array);

    // variable-precision, inexact, 6 decimal digits precision
    pub(crate) const FLOAT4: Self = Self(Type::Float4);
    pub(crate) const FLOAT4_ARRAY: Self = Self(Type::Float4Array);

    // variable-precision, inexact, 15 decimal digits precision
    pub(crate) const FLOAT8: Self = Self(Type::Float8);
    pub(crate) const FLOAT8_ARRAY: Self = Self(Type::Float8Array);

    // user-specified precision, exact
    pub(crate) const NUMERIC: Self = Self(Type::Numeric);
    pub(crate) const NUMERIC_ARRAY: Self = Self(Type::NumericArray);

    // user-specified precision, exact
    pub(crate) const MONEY: Self = Self(Type::Money);
    pub(crate) const MONEY_ARRAY: Self = Self(Type::MoneyArray);

    //
    // date/time types
    // https://www.postgresql.org/docs/current/datatype-datetime.html
    //

    // both date and time (no time zone)
    pub(crate) const TIMESTAMP: Self = Self(Type::Timestamp);
    pub(crate) const TIMESTAMP_ARRAY: Self = Self(Type::TimestampArray);

    // both date and time (with time zone)
    pub(crate) const TIMESTAMPTZ: Self = Self(Type::Timestamptz);
    pub(crate) const TIMESTAMPTZ_ARRAY: Self = Self(Type::TimestamptzArray);

    // date (no time of day)
    pub(crate) const DATE: Self = Self(Type::Date);
    pub(crate) const DATE_ARRAY: Self = Self(Type::DateArray);

    // time of day (no date)
    pub(crate) const TIME: Self = Self(Type::Time);
    pub(crate) const TIME_ARRAY: Self = Self(Type::TimeArray);

    // time of day (no date), with time zone
    pub(crate) const TIMETZ: Self = Self(Type::Timetz);
    pub(crate) const TIMETZ_ARRAY: Self = Self(Type::TimetzArray);

    // time interval
    pub(crate) const INTERVAL: Self = Self(Type::Interval);
    pub(crate) const INTERVAL_ARRAY: Self = Self(Type::IntervalArray);

    //
    // geometric types
    // https://www.postgresql.org/docs/current/datatype-geometric.html
    //

    // point on a plane
    pub(crate) const POINT: Self = Self(Type::Point);
    pub(crate) const POINT_ARRAY: Self = Self(Type::PointArray);

    // infinite line
    pub(crate) const LINE: Self = Self(Type::Line);
    pub(crate) const LINE_ARRAY: Self = Self(Type::LineArray);

    // finite line segment
    pub(crate) const LSEG: Self = Self(Type::Lseg);
    pub(crate) const LSEG_ARRAY: Self = Self(Type::LsegArray);

    // rectangular box
    pub(crate) const BOX: Self = Self(Type::Box);
    pub(crate) const BOX_ARRAY: Self = Self(Type::BoxArray);

    // open or closed path
    pub(crate) const PATH: Self = Self(Type::Path);
    pub(crate) const PATH_ARRAY: Self = Self(Type::PathArray);

    // polygon
    pub(crate) const POLYGON: Self = Self(Type::Polygon);
    pub(crate) const POLYGON_ARRAY: Self = Self(Type::PolygonArray);

    // circle
    pub(crate) const CIRCLE: Self = Self(Type::Circle);
    pub(crate) const CIRCLE_ARRAY: Self = Self(Type::CircleArray);

    //
    // bit string types
    // https://www.postgresql.org/docs/current/datatype-bit.html
    //

    pub(crate) const BIT: Self = Self(Type::Bit);
    pub(crate) const BIT_ARRAY: Self = Self(Type::BitArray);

    pub(crate) const VARBIT: Self = Self(Type::Varbit);
    pub(crate) const VARBIT_ARRAY: Self = Self(Type::VarbitArray);

    //
    // range types
    // https://www.postgresql.org/docs/current/rangetypes.html
    //

    pub(crate) const INT4_RANGE: Self = Self(Type::Int4Range);
    pub(crate) const INT4_RANGE_ARRAY: Self = Self(Type::Int4RangeArray);

    pub(crate) const NUM_RANGE: Self = Self(Type::NumRange);
    pub(crate) const NUM_RANGE_ARRAY: Self = Self(Type::NumRangeArray);

    pub(crate) const TS_RANGE: Self = Self(Type::TsRange);
    pub(crate) const TS_RANGE_ARRAY: Self = Self(Type::TsRangeArray);

    pub(crate) const TSTZ_RANGE: Self = Self(Type::TstzRange);
    pub(crate) const TSTZ_RANGE_ARRAY: Self = Self(Type::TstzRangeArray);

    pub(crate) const DATE_RANGE: Self = Self(Type::DateRange);
    pub(crate) const DATE_RANGE_ARRAY: Self = Self(Type::DateRangeArray);

    pub(crate) const INT8_RANGE: Self = Self(Type::Int8Range);
    pub(crate) const INT8_RANGE_ARRAY: Self = Self(Type::Int8RangeArray);

    //
    // pseudo types
    // https://www.postgresql.org/docs/9.3/datatype-pseudo.html
    //

    pub(crate) const VOID: Self = Self(Type::Void);
}

impl Display for TypeInfo {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.pad(self.name())
    }
}

impl PartialEq<Type> for Type {
    fn eq(&self, other: &Type) -> bool {
        self.eq_impl(other, true)
    }
}

/// Check type names for equality, respecting Postgres' case sensitivity rules for identifiers.
///
/// https://www.postgresql.org/docs/current/sql-syntax-lexical.html#SQL-SYNTAX-IDENTIFIERS
fn name_eq(name1: &str, name2: &str) -> bool {
    // Cop-out of processing Unicode escapes by just using string equality.
    if name1.starts_with("U&") {
        // If `name2` doesn't start with `U&` this will automatically be `false`.
        return name1 == name2;
    }

    let mut chars1 = identifier_chars(name1);
    let mut chars2 = identifier_chars(name2);

    while let (Some(a), Some(b)) = (chars1.next(), chars2.next()) {
        if !a.eq(&b) {
            return false;
        }
    }

    chars1.next().is_none() && chars2.next().is_none()
}

struct IdentifierChar {
    ch: char,
    case_sensitive: bool,
}

impl IdentifierChar {
    fn eq(&self, other: &Self) -> bool {
        if self.case_sensitive || other.case_sensitive {
            self.ch == other.ch
        } else {
            self.ch.eq_ignore_ascii_case(&other.ch)
        }
    }
}

/// Return an iterator over all significant characters of an identifier.
///
/// Ignores non-escaped quotation marks.
fn identifier_chars(ident: &str) -> impl Iterator<Item = IdentifierChar> + '_ {
    let mut case_sensitive = false;
    let mut last_char_quote = false;

    ident.chars().filter_map(move |ch| {
        if ch == '"' {
            if last_char_quote {
                last_char_quote = false;
            } else {
                last_char_quote = true;
                return None;
            }
        } else if last_char_quote {
            last_char_quote = false;
            case_sensitive = !case_sensitive;
        }

        Some(IdentifierChar { ch, case_sensitive })
    })
}

#[test]
fn test_name_eq() {
    let test_values = [
        ("foo", "foo", true),
        ("foo", "Foo", true),
        ("foo", "FOO", true),
        ("foo", r#""foo""#, true),
        ("foo", r#""Foo""#, false),
        ("foo", "foo.foo", false),
        ("foo.foo", "foo.foo", true),
        ("foo.foo", "foo.Foo", true),
        ("foo.foo", "foo.FOO", true),
        ("foo.foo", "Foo.foo", true),
        ("foo.foo", "Foo.Foo", true),
        ("foo.foo", "FOO.FOO", true),
        ("foo.foo", "foo", false),
        ("foo.foo", r#"foo."foo""#, true),
        ("foo.foo", r#"foo."Foo""#, false),
        ("foo.foo", r#"foo."FOO""#, false),
    ];

    for (left, right, eq) in test_values {
        assert_eq!(
            name_eq(left, right),
            eq,
            "failed check for name_eq({left:?}, {right:?})"
        );
        assert_eq!(
            name_eq(right, left),
            eq,
            "failed check for name_eq({right:?}, {left:?})"
        );
    }
}
