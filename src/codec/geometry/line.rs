use crate::decode::Decode;
use crate::encode::{Encode, IsNull};
use crate::error::BoxDynError;
use crate::codec::Type;
use crate::{ArgumentBuffer, HasArrayType, TypeInfo, ValueFormat, ValueRef};
use crate::bytes::Buf;
use std::str::FromStr;

const ERROR: &str = "error decoding LINE";

/// ## Postgres Geometric Line type
///
/// Description: Infinite line
/// Representation: `{A, B, C}`
///
/// Lines are represented by the linear equation Ax + By + C = 0, where A and B are not both zero.
///
/// See [Postgres Manual, Section 8.8.2, Geometric Types - Lines][PG.S.8.8.2] for details.
///
/// [PG.S.8.8.2]: https://www.postgresql.org/docs/current/datatype-geometric.html#DATATYPE-LINE
///
#[derive(Debug, Clone, PartialEq)]
pub struct Line {
    pub a: f64,
    pub b: f64,
    pub c: f64,
}

impl Type for Line {
    fn type_info() -> TypeInfo {
        TypeInfo::with_name("line")
    }
}

impl HasArrayType for Line {
    fn array_type_info() -> TypeInfo {
        TypeInfo::with_name("_line")
    }
}

impl<'r> Decode<'r> for Line {
    fn decode(value: ValueRef<'r>) -> Result<Self, Box<dyn std::error::Error + Send + Sync>> {
        match value.format() {
            ValueFormat::Text => Ok(Line::from_str(value.as_str()?)?),
            ValueFormat::Binary => Ok(Line::from_bytes(value.as_bytes()?)?),
        }
    }
}

impl Encode<'_> for Line {
    fn produces(&self) -> Option<TypeInfo> {
        Some(TypeInfo::with_name("line"))
    }

    fn encode_by_ref(&self, buf: &mut ArgumentBuffer) -> Result<IsNull, BoxDynError> {
        self.serialize(buf)?;
        Ok(IsNull::No)
    }
}

impl FromStr for Line {
    type Err = BoxDynError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let mut parts = s
            .trim_matches(|c| c == '{' || c == '}' || c == ' ')
            .split(',');

        let a = parts
            .next()
            .and_then(|s| s.trim().parse::<f64>().ok())
            .ok_or_else(|| format!("{}: could not get a from {}", ERROR, s))?;

        let b = parts
            .next()
            .and_then(|s| s.trim().parse::<f64>().ok())
            .ok_or_else(|| format!("{}: could not get b from {}", ERROR, s))?;

        let c = parts
            .next()
            .and_then(|s| s.trim().parse::<f64>().ok())
            .ok_or_else(|| format!("{}: could not get c from {}", ERROR, s))?;

        if parts.next().is_some() {
            return Err(format!("{}: too many numbers inputted in {}", ERROR, s).into());
        }

        Ok(Line { a, b, c })
    }
}

impl Line {
    fn from_bytes(mut bytes: &[u8]) -> Result<Line, BoxDynError> {
        let a = bytes.get_f64();
        let b = bytes.get_f64();
        let c = bytes.get_f64();
        Ok(Line { a, b, c })
    }

    fn serialize(&self, buff: &mut ArgumentBuffer) -> Result<(), BoxDynError> {
        buff.extend_from_slice(&self.a.to_be_bytes());
        buff.extend_from_slice(&self.b.to_be_bytes());
        buff.extend_from_slice(&self.c.to_be_bytes());
        Ok(())
    }

    #[cfg(test)]
    fn serialize_to_vec(&self) -> Vec<u8> {
        let mut buff = ArgumentBuffer::default();
        self.serialize(&mut buff).unwrap();
        buff.to_vec()
    }
}

#[cfg(test)]
mod line_tests {

    use std::str::FromStr;

    use super::Line;

    const LINE_BYTES: &[u8] = &[
        63, 241, 153, 153, 153, 153, 153, 154, 64, 1, 153, 153, 153, 153, 153, 154, 64, 10, 102,
        102, 102, 102, 102, 102,
    ];

    #[test]
    fn can_deserialise_line_type_bytes() {
        let line = Line::from_bytes(LINE_BYTES).unwrap();
        assert_eq!(
            line,
            Line {
                a: 1.1,
                b: 2.2,
                c: 3.3
            }
        )
    }

    #[test]
    fn can_deserialise_line_type_str() {
        let line = Line::from_str("{ 1, 2, 3 }").unwrap();
        assert_eq!(
            line,
            Line {
                a: 1.0,
                b: 2.0,
                c: 3.0
            }
        );
    }

    #[test]
    fn cannot_deserialise_line_too_few_numbers() {
        let input_str = "{ 1, 2 }";
        let line = Line::from_str(input_str);
        assert!(line.is_err());
        if let Err(err) = line {
            assert_eq!(
                err.to_string(),
                format!("error decoding LINE: could not get c from {input_str}")
            )
        }
    }

    #[test]
    fn cannot_deserialise_line_too_many_numbers() {
        let input_str = "{ 1, 2, 3, 4 }";
        let line = Line::from_str(input_str);
        assert!(line.is_err());
        if let Err(err) = line {
            assert_eq!(
                err.to_string(),
                format!("error decoding LINE: too many numbers inputted in {input_str}")
            )
        }
    }

    #[test]
    fn cannot_deserialise_line_invalid_numbers() {
        let input_str = "{ 1, 2, three }";
        let line = Line::from_str(input_str);
        assert!(line.is_err());
        if let Err(err) = line {
            assert_eq!(
                err.to_string(),
                format!("error decoding LINE: could not get c from {input_str}")
            )
        }
    }

    #[test]
    fn can_deserialise_line_type_str_float() {
        let line = Line::from_str("{1.1, 2.2, 3.3}").unwrap();
        assert_eq!(
            line,
            Line {
                a: 1.1,
                b: 2.2,
                c: 3.3
            }
        );
    }

    #[test]
    fn can_serialise_line_type() {
        let line = Line {
            a: 1.1,
            b: 2.2,
            c: 3.3,
        };
        assert_eq!(line.serialize_to_vec(), LINE_BYTES,)
    }
}
