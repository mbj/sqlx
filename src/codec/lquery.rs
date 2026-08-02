use crate::decode::Decode;
use crate::encode::{Encode, IsNull};
use crate::error::BoxDynError;
use crate::codec::Type;
use crate::{ArgumentBuffer, HasArrayType, TypeInfo, ValueFormat, ValueRef};
use bitflags::bitflags;
use std::fmt::{self, Display, Formatter};
use std::io::Write;
use std::ops::Deref;
use std::str::FromStr;

use crate::codec::ltree::{LTreeLabel, LTreeParseError};

/// Represents lquery specific errors
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum LQueryParseError {
    #[error("lquery cannot be empty")]
    EmptyString,
    #[error("unexpected character in lquery")]
    UnexpectedCharacter,
    #[error("error parsing integer: {0}")]
    ParseIntError(#[from] std::num::ParseIntError),
    #[error("error parsing integer: {0}")]
    LTreeParrseError(#[from] LTreeParseError),
    /// LQuery version not supported
    #[error("lquery version not supported")]
    InvalidLqueryVersion,
}

/// Container for a Label Tree Query (`lquery`) in Postgres.
///
/// See <https://www.postgresql.org/docs/current/ltree.html>
///
/// ### Note: Requires Postgres 13+
///
/// This integration requires that the `lquery` type support the binary format in the Postgres
/// wire protocol, which only became available in Postgres 13.
/// ([Postgres 13.0 Release Notes, Additional Modules](https://www.postgresql.org/docs/13/release-13.html#id-1.11.6.11.5.14))
///
/// Ideally, SQLx's Postgres driver should support falling back to text format for types
/// which don't have `typsend` and `typrecv` entries in `pg_type`, but that work still needs
/// to be done.
///
/// ### Note: Extension Required
/// The `ltree` extension is not enabled by default in Postgres. You will need to do so explicitly:
///
/// ```ignore
/// CREATE EXTENSION IF NOT EXISTS "ltree";
/// ```
#[derive(Clone, Debug, Default, PartialEq)]
pub struct LQuery {
    levels: Vec<LQueryLevel>,
}

impl LQuery {
    /// creates default/empty lquery
    pub fn new() -> Self {
        Self::default()
    }

    pub fn from(levels: Vec<LQueryLevel>) -> Self {
        Self { levels }
    }

    /// push a query level
    pub fn push(&mut self, level: LQueryLevel) {
        self.levels.push(level);
    }

    /// pop a query level
    pub fn pop(&mut self) -> Option<LQueryLevel> {
        self.levels.pop()
    }

    /// creates lquery from an iterator with checking labels
    // TODO: this should just be removed but I didn't want to bury it in a massive diff
    #[deprecated = "renamed to `try_from_iter()`"]
    #[allow(clippy::should_implement_trait)]
    pub fn from_iter<I, S>(levels: I) -> Result<Self, LQueryParseError>
    where
        S: Into<String>,
        I: IntoIterator<Item = S>,
    {
        let mut lquery = Self::default();
        for level in levels {
            lquery.push(LQueryLevel::from_str(&level.into())?);
        }
        Ok(lquery)
    }

    /// Create an `LQUERY` from an iterator of label strings.
    ///
    /// Returns an error if any label fails to parse according to [`LQueryLevel::from_str()`].
    pub fn try_from_iter<I, S>(levels: I) -> Result<Self, LQueryParseError>
    where
        S: AsRef<str>,
        I: IntoIterator<Item = S>,
    {
        levels
            .into_iter()
            .map(|level| level.as_ref().parse::<LQueryLevel>())
            .collect()
    }
}

impl FromIterator<LQueryLevel> for LQuery {
    fn from_iter<T: IntoIterator<Item = LQueryLevel>>(iter: T) -> Self {
        Self::from(iter.into_iter().collect())
    }
}

impl IntoIterator for LQuery {
    type Item = LQueryLevel;
    type IntoIter = std::vec::IntoIter<Self::Item>;

    fn into_iter(self) -> Self::IntoIter {
        self.levels.into_iter()
    }
}

impl FromStr for LQuery {
    type Err = LQueryParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(Self {
            levels: s
                .split('.')
                .map(LQueryLevel::from_str)
                .collect::<Result<_, Self::Err>>()?,
        })
    }
}

impl Display for LQuery {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        let mut iter = self.levels.iter();
        if let Some(label) = iter.next() {
            write!(f, "{label}")?;
            for label in iter {
                write!(f, ".{label}")?;
            }
        }
        Ok(())
    }
}

impl Deref for LQuery {
    type Target = [LQueryLevel];

    fn deref(&self) -> &Self::Target {
        &self.levels
    }
}

impl Type for LQuery {
    fn type_info() -> TypeInfo {
        // Since `ltree` is enabled by an extension, it does not have a stable OID.
        TypeInfo::with_name("lquery")
    }
}

impl HasArrayType for LQuery {
    fn array_type_info() -> TypeInfo {
        TypeInfo::with_name("_lquery")
    }
}

impl Encode<'_> for LQuery {
    fn encode_by_ref(&self, buf: &mut ArgumentBuffer) -> Result<IsNull, BoxDynError> {
        buf.extend(1i8.to_le_bytes());
        write!(buf, "{self}")?;

        Ok(IsNull::No)
    }
}

impl<'r> Decode<'r> for LQuery {
    fn decode(value: ValueRef<'r>) -> Result<Self, BoxDynError> {
        match value.format() {
            ValueFormat::Binary => {
                let bytes = value.as_bytes()?;
                let version = i8::from_le_bytes([bytes[0]; 1]);
                if version != 1 {
                    return Err(Box::new(LQueryParseError::InvalidLqueryVersion));
                }
                Ok(Self::from_str(std::str::from_utf8(&bytes[1..])?)?)
            }
            ValueFormat::Text => Ok(Self::from_str(value.as_str()?)?),
        }
    }
}

bitflags! {
    /// Modifiers that can be set to non-star labels
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
    pub struct LQueryVariantFlag: u16 {
        /// * - Match any label with this prefix, for example foo* matches foobar
        const ANY_END = 0x01;
        /// @ - Match case-insensitively, for example a@ matches A
        const IN_CASE = 0x02;
        /// % - Match initial underscore-separated words
        const SUBLEXEME = 0x04;
    }
}

impl Display for LQueryVariantFlag {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        if self.contains(LQueryVariantFlag::ANY_END) {
            write!(f, "*")?;
        }
        if self.contains(LQueryVariantFlag::IN_CASE) {
            write!(f, "@")?;
        }
        if self.contains(LQueryVariantFlag::SUBLEXEME) {
            write!(f, "%")?;
        }

        Ok(())
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct LQueryVariant {
    label: LTreeLabel,
    modifiers: LQueryVariantFlag,
}

impl Display for LQueryVariant {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{}{}", self.label, self.modifiers)
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum LQueryLevel {
    /// match any label (*) with optional at least / at most numbers
    Star(Option<u16>, Option<u16>),
    /// match any of specified labels with optional flags
    NonStar(Vec<LQueryVariant>),
    /// match none of specified labels with optional flags
    NotNonStar(Vec<LQueryVariant>),
}

impl FromStr for LQueryLevel {
    type Err = LQueryParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let bytes = s.as_bytes();
        if bytes.is_empty() {
            Err(LQueryParseError::EmptyString)
        } else {
            match bytes[0] {
                b'*' => {
                    if bytes.len() > 1 {
                        let parts = s[2..s.len() - 1].split(',').collect::<Vec<_>>();
                        match parts.len() {
                            1 => {
                                let number = parts[0].parse()?;
                                Ok(LQueryLevel::Star(Some(number), Some(number)))
                            }
                            2 => Ok(LQueryLevel::Star(
                                Some(parts[0].parse()?),
                                Some(parts[1].parse()?),
                            )),
                            _ => Err(LQueryParseError::UnexpectedCharacter),
                        }
                    } else {
                        Ok(LQueryLevel::Star(None, None))
                    }
                }
                b'!' => Ok(LQueryLevel::NotNonStar(
                    s[1..]
                        .split('|')
                        .map(LQueryVariant::from_str)
                        .collect::<Result<Vec<_>, LQueryParseError>>()?,
                )),
                _ => Ok(LQueryLevel::NonStar(
                    s.split('|')
                        .map(LQueryVariant::from_str)
                        .collect::<Result<Vec<_>, LQueryParseError>>()?,
                )),
            }
        }
    }
}

impl FromStr for LQueryVariant {
    type Err = LQueryParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let mut label_length = s.len();
        let mut modifiers = LQueryVariantFlag::empty();

        for b in s.bytes().rev() {
            match b {
                b'@' => modifiers.insert(LQueryVariantFlag::IN_CASE),
                b'*' => modifiers.insert(LQueryVariantFlag::ANY_END),
                b'%' => modifiers.insert(LQueryVariantFlag::SUBLEXEME),
                _ => break,
            }
            label_length -= 1;
        }

        Ok(LQueryVariant {
            label: LTreeLabel::new(&s[0..label_length])?,
            modifiers,
        })
    }
}

fn write_variants(f: &mut Formatter<'_>, variants: &[LQueryVariant], not: bool) -> fmt::Result {
    let mut iter = variants.iter();
    if let Some(variant) = iter.next() {
        write!(f, "{}{}", if not { "!" } else { "" }, variant)?;
        for variant in iter {
            write!(f, ".{variant}")?;
        }
    }
    Ok(())
}

impl Display for LQueryLevel {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            LQueryLevel::Star(Some(at_least), Some(at_most)) => {
                if at_least == at_most {
                    write!(f, "*{{{at_least}}}")
                } else {
                    write!(f, "*{{{at_least},{at_most}}}")
                }
            }
            LQueryLevel::Star(Some(at_least), _) => write!(f, "*{{{at_least},}}"),
            LQueryLevel::Star(_, Some(at_most)) => write!(f, "*{{,{at_most}}}"),
            LQueryLevel::Star(_, _) => write!(f, "*"),
            LQueryLevel::NonStar(variants) => write_variants(f, variants, false),
            LQueryLevel::NotNonStar(variants) => write_variants(f, variants, true),
        }
    }
}
