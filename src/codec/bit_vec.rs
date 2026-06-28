use crate::arguments::value_size_int4_checked;
use crate::{
    decode::Decode,
    encode::{Encode, IsNull},
    error::BoxDynError,
    types::Type,
    ArgumentBuffer, HasArrayType, TypeInfo, ValueFormat, ValueRef,
};
use bit_vec::BitVec;
use crate::bytes::Buf;
use std::{io, mem};

impl Type for BitVec {
    fn type_info() -> TypeInfo {
        TypeInfo::VARBIT
    }

    fn compatible(ty: &TypeInfo) -> bool {
        *ty == TypeInfo::BIT || *ty == TypeInfo::VARBIT
    }
}

impl HasArrayType for BitVec {
    fn array_type_info() -> TypeInfo {
        TypeInfo::VARBIT_ARRAY
    }

    fn array_compatible(ty: &TypeInfo) -> bool {
        *ty == TypeInfo::BIT_ARRAY || *ty == TypeInfo::VARBIT_ARRAY
    }
}

impl Encode<'_> for BitVec {
    fn encode_by_ref(&self, buf: &mut ArgumentBuffer) -> Result<IsNull, BoxDynError> {
        let len = value_size_int4_checked(self.len())?;

        buf.extend(len.to_be_bytes());
        buf.extend(self.to_bytes());

        Ok(IsNull::No)
    }

    fn size_hint(&self) -> usize {
        mem::size_of::<i32>() + self.len()
    }
}

impl Decode<'_> for BitVec {
    fn decode(value: ValueRef<'_>) -> Result<Self, BoxDynError> {
        match value.format() {
            ValueFormat::Binary => {
                let mut bytes = value.as_bytes()?;
                let len = bytes.get_i32();

                let len = usize::try_from(len).map_err(|_| format!("invalid VARBIT len: {len}"))?;

                // The smallest amount of data we can read is one byte
                let bytes_len = len.div_ceil(8);

                if bytes.remaining() != bytes_len {
                    Err(io::Error::new(
                        io::ErrorKind::InvalidData,
                        "VARBIT length mismatch.",
                    ))?;
                }

                let mut bitvec = BitVec::from_bytes(bytes);

                // Chop off zeroes from the back. We get bits in bytes, so if
                // our bitvec is not in full bytes, extra zeroes are added to
                // the end.
                while bitvec.len() > len {
                    bitvec.pop();
                }

                Ok(bitvec)
            }
            ValueFormat::Text => {
                let s = value.as_str()?;
                let mut bit_vec = BitVec::with_capacity(s.len());

                for c in s.chars() {
                    match c {
                        '0' => bit_vec.push(false),
                        '1' => bit_vec.push(true),
                        _ => {
                            Err(io::Error::new(
                                io::ErrorKind::InvalidData,
                                "VARBIT data contains other characters than 1 or 0.",
                            ))?;
                        }
                    }
                }

                Ok(bit_vec)
            }
        }
    }
}
