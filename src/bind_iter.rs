use crate::{ArgumentBuffer, HasArrayType, TypeInfo};
use core::cell::Cell;
use crate::{
    encode::{Encode, IsNull},
    error::BoxDynError,
    codec::Type,
};

// not exported but pub because it is used in the extension trait
pub struct BindIter<I>(Cell<Option<I>>);

/// Iterator extension trait enabling iterators to encode arrays in Postgres.
///
/// Because of the blanket impl of `HasArrayType` for all references
/// we can borrow instead of needing to clone or copy in the iterators
/// and it still works
///
/// Previously, 3 separate arrays would be needed in this example which
/// requires iterating 3 times to collect items into the array and then
/// iterating over them again to encode.
///
/// This now requires only iterating over the array once for each field
/// while using less memory giving both speed and memory usage improvements
/// along with allowing much more flexibility in the underlying collection.
pub trait BindIterExt: Iterator + Sized {
    fn bind_iter(self) -> BindIter<Self>;
}

impl<I: Iterator + Sized> BindIterExt for I {
    fn bind_iter(self) -> BindIter<I> {
        BindIter(Cell::new(Some(self)))
    }
}

impl<I> Type for BindIter<I>
where
    I: Iterator,
    <I as Iterator>::Item: Type + HasArrayType,
{
    fn type_info() -> crate::TypeInfo {
        <I as Iterator>::Item::array_type_info()
    }
    fn compatible(ty: &TypeInfo) -> bool {
        <I as Iterator>::Item::array_compatible(ty)
    }
}

impl<'q, I> BindIter<I>
where
    I: Iterator,
    <I as Iterator>::Item: Type + Encode<'q>,
{
    fn encode_inner(
        // need ownership to iterate
        mut iter: I,
        buf: &mut ArgumentBuffer,
    ) -> Result<IsNull, BoxDynError> {
        let lower_size_hint = iter.size_hint().0;
        let first = iter.next();
        let type_info = first
            .as_ref()
            .and_then(Encode::produces)
            .unwrap_or_else(<I as Iterator>::Item::type_info);

        buf.extend(&1_i32.to_be_bytes()); // number of dimensions
        buf.extend(&0_i32.to_be_bytes()); // flags

        if let Some(oid) = type_info.oid() {
            buf.extend(oid.0.to_be_bytes());
        } else {
            buf.push_hole(type_info);
        }

        let len_start = buf.len();
        buf.extend(0_i32.to_be_bytes()); // len (unknown so far)
        buf.extend(1_i32.to_be_bytes()); // lower bound

        match first {
            Some(first) => buf.encode(first)?,
            None => return Ok(IsNull::No),
        }

        let mut count = 1_i32;
        const MAX: usize = i32::MAX as usize - 1;

        for value in (&mut iter).take(MAX) {
            buf.encode(value)?;
            count += 1;
        }

        const OVERFLOW: usize = i32::MAX as usize + 1;
        if iter.next().is_some() {
            let iter_size = std::cmp::max(lower_size_hint, OVERFLOW);
            return Err(format!("encoded iterator is too large for Postgres: {iter_size}").into());
        }

        // set the length now that we know what it is.
        buf[len_start..(len_start + 4)].copy_from_slice(&count.to_be_bytes());

        Ok(IsNull::No)
    }
}

impl<'q, I> Encode<'q> for BindIter<I>
where
    I: Iterator,
    <I as Iterator>::Item: Type + Encode<'q>,
{
    fn encode_by_ref(&self, buf: &mut ArgumentBuffer) -> Result<IsNull, BoxDynError> {
        Self::encode_inner(self.0.take().expect("BindIter is only used once"), buf)
    }
    fn encode(self, buf: &mut ArgumentBuffer) -> Result<IsNull, BoxDynError>
    where
        Self: Sized,
    {
        Self::encode_inner(
            self.0.into_inner().expect("BindIter is only used once"),
            buf,
        )
    }
}
