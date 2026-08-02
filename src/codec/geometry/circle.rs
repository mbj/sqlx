use crate::decode::Decode;
use crate::encode::{Encode, IsNull};
use crate::error::BoxDynError;
use crate::codec::Type;
use crate::{ArgumentBuffer, HasArrayType, TypeInfo, ValueFormat, ValueRef};
use crate::bytes::Buf;
use crate::Error;
use std::str::FromStr;

const ERROR: &str = "error decoding CIRCLE";

/// ## Postgres Geometric Circle type
///
/// Description: Circle
/// Representation: `< (x, y), radius >` (center point and radius)
///
/// ```text
/// < ( x , y ) , radius >
/// ( ( x , y ) , radius )
///   ( x , y ) , radius
///     x , y   , radius
/// ```
/// where `(x,y)` is the center point.
///
/// See [Postgres Manual, Section 8.8.7, Geometric Types - Circles][PG.S.8.8.7] for details.
///
/// [PG.S.8.8.7]: https://www.postgresql.org/docs/current/datatype-geometric.html#DATATYPE-CIRCLE
///
#[derive(Debug, Clone, PartialEq)]
pub struct Circle {
    pub x: f64,
    pub y: f64,
    pub radius: f64,
}

impl Type for Circle {
    fn type_info() -> TypeInfo {
        TypeInfo::with_name("circle")
    }
}

impl HasArrayType for Circle {
    fn array_type_info() -> TypeInfo {
        TypeInfo::with_name("_circle")
    }
}

impl<'r> Decode<'r> for Circle {
    fn decode(value: ValueRef<'r>) -> Result<Self, Box<dyn std::error::Error + Send + Sync>> {
        match value.format() {
            ValueFormat::Text => Ok(Circle::from_str(value.as_str()?)?),
            ValueFormat::Binary => Ok(Circle::from_bytes(value.as_bytes()?)?),
        }
    }
}

impl Encode<'_> for Circle {
    fn produces(&self) -> Option<TypeInfo> {
        Some(TypeInfo::with_name("circle"))
    }

    fn encode_by_ref(&self, buf: &mut ArgumentBuffer) -> Result<IsNull, BoxDynError> {
        self.serialize(buf)?;
        Ok(IsNull::No)
    }
}

impl FromStr for Circle {
    type Err = BoxDynError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let sanitised = s.replace(['<', '>', '(', ')', ' '], "");
        let mut parts = sanitised.split(',');

        let x = parts
            .next()
            .and_then(|s| s.trim().parse::<f64>().ok())
            .ok_or_else(|| format!("{}: could not get x from {}", ERROR, s))?;

        let y = parts
            .next()
            .and_then(|s| s.trim().parse::<f64>().ok())
            .ok_or_else(|| format!("{}: could not get y from {}", ERROR, s))?;

        let radius = parts
            .next()
            .and_then(|s| s.trim().parse::<f64>().ok())
            .ok_or_else(|| format!("{}: could not get radius from {}", ERROR, s))?;

        if parts.next().is_some() {
            return Err(format!("{}: too many numbers inputted in {}", ERROR, s).into());
        }

        if radius < 0. {
            return Err(format!("{}: cannot have negative radius: {}", ERROR, s).into());
        }

        Ok(Circle { x, y, radius })
    }
}

impl Circle {
    fn from_bytes(mut bytes: &[u8]) -> Result<Circle, Error> {
        let x = bytes.get_f64();
        let y = bytes.get_f64();
        let r = bytes.get_f64();
        Ok(Circle { x, y, radius: r })
    }

    fn serialize(&self, buff: &mut ArgumentBuffer) -> Result<(), Error> {
        buff.extend_from_slice(&self.x.to_be_bytes());
        buff.extend_from_slice(&self.y.to_be_bytes());
        buff.extend_from_slice(&self.radius.to_be_bytes());
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
mod circle_tests {

    use std::str::FromStr;

    use super::Circle;

    const CIRCLE_BYTES: &[u8] = &[
        63, 241, 153, 153, 153, 153, 153, 154, 64, 1, 153, 153, 153, 153, 153, 154, 64, 10, 102,
        102, 102, 102, 102, 102,
    ];

    #[test]
    fn can_deserialise_circle_type_bytes() {
        let circle = Circle::from_bytes(CIRCLE_BYTES).unwrap();
        assert_eq!(
            circle,
            Circle {
                x: 1.1,
                y: 2.2,
                radius: 3.3
            }
        )
    }

    #[test]
    fn can_deserialise_circle_type_str() {
        let circle = Circle::from_str("<(1, 2), 3 >").unwrap();
        assert_eq!(
            circle,
            Circle {
                x: 1.0,
                y: 2.0,
                radius: 3.0
            }
        );
    }

    #[test]
    fn can_deserialise_circle_type_str_second_syntax() {
        let circle = Circle::from_str("((1, 2), 3 )").unwrap();
        assert_eq!(
            circle,
            Circle {
                x: 1.0,
                y: 2.0,
                radius: 3.0
            }
        );
    }

    #[test]
    fn can_deserialise_circle_type_str_third_syntax() {
        let circle = Circle::from_str("(1, 2), 3 ").unwrap();
        assert_eq!(
            circle,
            Circle {
                x: 1.0,
                y: 2.0,
                radius: 3.0
            }
        );
    }

    #[test]
    fn can_deserialise_circle_type_str_fourth_syntax() {
        let circle = Circle::from_str("1, 2, 3 ").unwrap();
        assert_eq!(
            circle,
            Circle {
                x: 1.0,
                y: 2.0,
                radius: 3.0
            }
        );
    }

    #[test]
    fn cannot_deserialise_circle_invalid_numbers() {
        let input_str = "1, 2, Three";
        let circle = Circle::from_str(input_str);
        assert!(circle.is_err());
        if let Err(err) = circle {
            assert_eq!(
                err.to_string(),
                format!("error decoding CIRCLE: could not get radius from {input_str}")
            )
        }
    }

    #[test]
    fn cannot_deserialise_circle_negative_radius() {
        let input_str = "1, 2, -3";
        let circle = Circle::from_str(input_str);
        assert!(circle.is_err());
        if let Err(err) = circle {
            assert_eq!(
                err.to_string(),
                format!("error decoding CIRCLE: cannot have negative radius: {input_str}")
            )
        }
    }

    #[test]
    fn can_deserialise_circle_type_str_float() {
        let circle = Circle::from_str("<(1.1, 2.2), 3.3>").unwrap();
        assert_eq!(
            circle,
            Circle {
                x: 1.1,
                y: 2.2,
                radius: 3.3
            }
        );
    }

    #[test]
    fn can_serialise_circle_type() {
        let circle = Circle {
            x: 1.1,
            y: 2.2,
            radius: 3.3,
        };
        assert_eq!(circle.serialize_to_vec(), CIRCLE_BYTES,)
    }
}
