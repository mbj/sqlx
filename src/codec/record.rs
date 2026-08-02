use crate::bytes::Buf;

use crate::decode::Decode;
use crate::encode::Encode;
use crate::error::{mismatched_types, BoxDynError};
use crate::codec::{meta, TypeKind};
use crate::codec::Oid;
use crate::codec::Type;
use crate::{ArgumentBuffer, TypeInfo, ValueFormat, ValueRef};

#[doc(hidden)]
pub struct RecordEncoder<'a> {
    buf: &'a mut ArgumentBuffer,
    off: usize,
    num: u32,
}

impl<'a> RecordEncoder<'a> {
    #[doc(hidden)]
    pub fn new(buf: &'a mut ArgumentBuffer) -> Self {
        let off = buf.len();

        // reserve space for a field count
        buf.extend(&(0_u32).to_be_bytes());

        Self { buf, off, num: 0 }
    }

    #[doc(hidden)]
    pub fn finish(&mut self) {
        // fill in the record length
        self.buf[self.off..(self.off + 4)].copy_from_slice(&self.num.to_be_bytes());
    }

    #[doc(hidden)]
    pub fn encode<'q, T>(&mut self, value: T) -> Result<&mut Self, BoxDynError>
    where
        'a: 'q,
        T: Encode<'q> + Type,
    {
        let ty = value.produces().unwrap_or_else(T::type_info);

        if let Some(oid) = ty.oid() {
            self.buf.extend(oid.0.to_be_bytes())
        } else {
            self.buf.push_hole(ty);
        }

        self.buf.encode(value)?;
        self.num += 1;

        Ok(self)
    }
}

#[doc(hidden)]
pub struct RecordDecoder<'r> {
    buf: &'r [u8],
    typ: TypeInfo,
    fmt: ValueFormat,
    ind: usize,
}

impl<'r> RecordDecoder<'r> {
    #[doc(hidden)]
    pub fn new(value: ValueRef<'r>) -> Result<Self, BoxDynError> {
        let fmt = value.format();
        let mut buf = value.as_bytes()?;
        let typ = value.type_info;

        match fmt {
            ValueFormat::Binary => {
                let _len = buf.get_u32();
            }

            ValueFormat::Text => {
                // remove the enclosing `(` .. `)`
                buf = &buf[1..(buf.len() - 1)];
            }
        }

        Ok(Self {
            buf,
            fmt,
            typ,
            ind: 0,
        })
    }

    #[doc(hidden)]
    pub fn try_decode<T>(&mut self) -> Result<T, BoxDynError>
    where
        T: for<'a> Decode<'a> + Type,
    {
        if self.buf.is_empty() {
            return Err(format!("no field `{0}` found on record", self.ind).into());
        }

        match self.fmt {
            ValueFormat::Binary => {
                let element_type_oid = Oid(self.buf.get_u32());
                let element_type_opt = self.find_type_info(&self.typ, element_type_oid)?;

                if let Some(ty) = &element_type_opt {
                    if !ty.is_null() && !T::compatible(ty) {
                        return Err(mismatched_types::<T>(ty));
                    }
                }

                let element_type =
                    element_type_opt
                        .ok_or_else(|| BoxDynError::from(format!("custom types in records are not fully supported yet: failed to retrieve type info for field {} with type oid {}", self.ind, element_type_oid.0)))?;

                self.ind += 1;

                T::decode(ValueRef::get(&mut self.buf, self.fmt, element_type)?)
            }

            ValueFormat::Text => {
                let mut element = String::new();
                let mut quoted = false;
                let mut in_quotes = false;
                let mut in_escape = false;
                let mut prev_ch = '\0';

                while !self.buf.is_empty() {
                    let ch = self.buf.get_u8() as char;
                    match ch {
                        _ if in_escape => {
                            element.push(ch);
                            in_escape = false;
                        }

                        '"' if in_quotes => {
                            in_quotes = false;
                        }

                        '"' => {
                            in_quotes = true;
                            quoted = true;

                            if prev_ch == '"' {
                                element.push('"')
                            }
                        }

                        '\\' if !in_escape => {
                            in_escape = true;
                        }

                        ',' if !in_quotes => break,

                        _ => {
                            element.push(ch);
                        }
                    }
                    prev_ch = ch;
                }

                let buf = if element.is_empty() && !quoted {
                    // completely empty input means NULL
                    None
                } else {
                    Some(element.as_bytes())
                };

                // NOTE: we do not call [`accepts`] or give a chance to from a user as
                //       TEXT sequences are not strongly typed

                T::decode(ValueRef {
                    // NOTE: We pass `0` as the type ID because we don't have a reasonable value
                    //       we could use.
                    type_info: TypeInfo::with_oid(Oid(0)),
                    format: self.fmt,
                    value: buf,
                    row: None,
                })
            }
        }
    }

    fn find_type_info(
        &self,
        typ: &TypeInfo,
        oid: Oid,
    ) -> Result<Option<TypeInfo>, BoxDynError> {
        match typ.kind() {
            TypeKind::Simple if typ.0 == meta::Type::Record => Ok(TypeInfo::try_from_oid(oid)),
            TypeKind::Composite(fields) => {
                let ty = fields[self.ind].1.clone();
                if ty.0.oid() != oid {
                    return Err("unexpected mismatch of composite type information".into());
                }

                Ok(Some(ty))
            }
            TypeKind::Domain(domain) => self.find_type_info(domain, oid),
            _ => Err("unexpected custom type being decoded as a composite type".into()),
        }
    }
}
