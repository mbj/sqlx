use crate::decode::Decode;
use crate::encode::{Encode, IsNull};
use crate::error::BoxDynError;
use crate::codec::Type;
use crate::{ArgumentBuffer, HasArrayType, TypeInfo, ValueFormat, ValueRef};
use std::fmt::{self, Display, Formatter};
use std::io::Write;
use std::ops::Deref;
use std::str::FromStr;

/// Represents ltree specific errors
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum LTreeParseError {
    /// LTree labels can only contain [A-Za-z0-9_]
    #[error("ltree label contains invalid characters")]
    InvalidLtreeLabel,

    /// LTree version not supported
    #[error("ltree version not supported")]
    InvalidLtreeVersion,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct LTreeLabel(String);

impl LTreeLabel {
    pub fn new<S>(label: S) -> Result<Self, LTreeParseError>
    where
        S: Into<String>,
    {
        let label = label.into();
        if label.len() <= 256
            && label
                .bytes()
                .all(|c| c.is_ascii_alphabetic() || c.is_ascii_digit() || c == b'_')
        {
            Ok(Self(label))
        } else {
            Err(LTreeParseError::InvalidLtreeLabel)
        }
    }
}

impl Deref for LTreeLabel {
    type Target = str;

    fn deref(&self) -> &Self::Target {
        self.0.as_str()
    }
}

impl FromStr for LTreeLabel {
    type Err = LTreeParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        LTreeLabel::new(s)
    }
}

impl Display for LTreeLabel {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Container for a Label Tree (`ltree`) in Postgres.
///
/// See <https://www.postgresql.org/docs/current/ltree.html>
///
/// ### Note: Requires Postgres 13+
///
/// This integration requires that the `ltree` type support the binary format in the Postgres
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
pub struct LTree {
    labels: Vec<LTreeLabel>,
}

impl LTree {
    /// creates default/empty ltree
    pub fn new() -> Self {
        Self::default()
    }

    /// creates ltree from a [`Vec<LTreeLabel>`]
    pub fn from_labels(labels: Vec<LTreeLabel>) -> Self {
        Self { labels }
    }

    /// creates ltree from an iterator with checking labels
    // TODO: this should just be removed but I didn't want to bury it in a massive diff
    #[deprecated = "renamed to `try_from_iter()`"]
    #[allow(clippy::should_implement_trait)]
    pub fn from_iter<I, S>(labels: I) -> Result<Self, LTreeParseError>
    where
        String: From<S>,
        I: IntoIterator<Item = S>,
    {
        let mut ltree = Self::default();
        for label in labels {
            ltree.push(LTreeLabel::new(label)?);
        }
        Ok(ltree)
    }

    /// Create an `LTREE` from an iterator of label strings.
    ///
    /// Returns an error if any label fails to parse according to [`LTreeLabel::new()`].
    pub fn try_from_iter<I, S>(labels: I) -> Result<Self, LTreeParseError>
    where
        S: Into<String>,
        I: IntoIterator<Item = S>,
    {
        labels.into_iter().map(LTreeLabel::new).collect()
    }

    /// push a label to ltree
    pub fn push(&mut self, label: LTreeLabel) {
        self.labels.push(label);
    }

    /// pop a label from ltree
    pub fn pop(&mut self) -> Option<LTreeLabel> {
        self.labels.pop()
    }
}

impl From<Vec<LTreeLabel>> for LTree {
    fn from(labels: Vec<LTreeLabel>) -> Self {
        Self { labels }
    }
}

impl FromIterator<LTreeLabel> for LTree {
    fn from_iter<T: IntoIterator<Item = LTreeLabel>>(iter: T) -> Self {
        Self {
            labels: iter.into_iter().collect(),
        }
    }
}

impl IntoIterator for LTree {
    type Item = LTreeLabel;
    type IntoIter = std::vec::IntoIter<Self::Item>;

    fn into_iter(self) -> Self::IntoIter {
        self.labels.into_iter()
    }
}

impl FromStr for LTree {
    type Err = LTreeParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(Self {
            labels: s
                .split('.')
                .map(LTreeLabel::new)
                .collect::<Result<Vec<_>, Self::Err>>()?,
        })
    }
}

impl Display for LTree {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        let mut iter = self.labels.iter();
        if let Some(label) = iter.next() {
            write!(f, "{label}")?;
            for label in iter {
                write!(f, ".{label}")?;
            }
        }
        Ok(())
    }
}

impl Deref for LTree {
    type Target = [LTreeLabel];

    fn deref(&self) -> &Self::Target {
        &self.labels
    }
}

impl Type for LTree {
    fn type_info() -> TypeInfo {
        // Since `ltree` is enabled by an extension, it does not have a stable OID.
        TypeInfo::with_name("ltree")
    }
}

impl HasArrayType for LTree {
    fn array_type_info() -> TypeInfo {
        TypeInfo::with_name("_ltree")
    }
}

impl Encode<'_> for LTree {
    fn encode_by_ref(&self, buf: &mut ArgumentBuffer) -> Result<IsNull, BoxDynError> {
        buf.extend(1i8.to_le_bytes());
        write!(buf, "{self}")?;

        Ok(IsNull::No)
    }
}

impl<'r> Decode<'r> for LTree {
    fn decode(value: ValueRef<'r>) -> Result<Self, BoxDynError> {
        match value.format() {
            ValueFormat::Binary => {
                let bytes = value.as_bytes()?;
                let version = i8::from_le_bytes([bytes[0]; 1]);
                if version != 1 {
                    return Err(Box::new(LTreeParseError::InvalidLtreeVersion));
                }
                Ok(Self::from_str(std::str::from_utf8(&bytes[1..])?)?)
            }
            ValueFormat::Text => Ok(Self::from_str(value.as_str()?)?),
        }
    }
}
