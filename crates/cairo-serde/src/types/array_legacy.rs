//! Dedicated struct for cairo 0 arrays, where len is not prefixed.
use crate::{CairoSerde, Error, Result};
use starknet::core::types::FieldElement;

#[derive(Debug, Clone, PartialEq, PartialOrd)]
pub struct CairoArrayLegacy<T>(pub Vec<T>);

impl<T: std::clone::Clone> CairoArrayLegacy<T> {
    pub fn from_slice(slice: &[T]) -> Self {
        Self(slice.to_vec())
    }

    pub fn len(&self) -> usize {
        self.0.len()
    }

    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
}

impl<T> From<Vec<T>> for CairoArrayLegacy<T> {
    fn from(value: Vec<T>) -> Self {
        Self(value)
    }
}

impl<T, RT> CairoSerde for CairoArrayLegacy<T>
where
    T: CairoSerde<RustType = RT>,
{
    type RustType = CairoArrayLegacy<RT>;

    const SERIALIZED_SIZE: Option<usize> = None;

    #[inline]
    fn cairo_serialized_size(rust: &Self::RustType) -> usize {
        let data = &rust.0;
        // In cairo 0, the length is always passed as an argument.
        data.iter().map(T::cairo_serialized_size).sum::<usize>()
    }

    fn cairo_serialize(rust: &Self::RustType) -> Vec<FieldElement> {
        let mut out: Vec<FieldElement> = vec![];
        rust.0
            .iter()
            .for_each(|r| out.extend(T::cairo_serialize(r)));
        out
    }

    fn cairo_deserialize(felts: &[FieldElement], offset: usize) -> Result<Self::RustType> {
        if offset >= felts.len() {
            // As the length of cairo 0 arrays is not included in the serialized form of the array,
            // we don't have much choice here to return an empty array instead of an error.
            return Ok(CairoArrayLegacy(vec![]));
        }

        let mut out: Vec<RT> = vec![];
        let mut offset = offset;
        let len = felts[offset - 1];

        if FieldElement::from(offset) + len > FieldElement::from(felts.len()) {
            return Err(Error::Deserialize(format!(
                "Buffer too short to deserialize an array of length {}: offset ({}) : buffer {:?}",
                len, offset, felts,
            )));
        }

        loop {
            if FieldElement::from(out.len()) == len {
                break;
            }

            let rust: RT = T::cairo_deserialize(felts, offset)?;
            offset += T::cairo_serialized_size(&rust);
            out.push(rust);
        }

        Ok(CairoArrayLegacy(out))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use starknet::macros::felt;

    #[test]
    fn array_offset_len_ok() {
        let serialized = vec![felt!("4"), felt!("1"), felt!("2"), felt!("3"), felt!("4")];
        let a = CairoArrayLegacy::<FieldElement>::cairo_deserialize(&serialized, 1).unwrap();
        assert_eq!(a.len(), 4);
        assert_eq!(a.0[0], felt!("1"));
        assert_eq!(a.0[1], felt!("2"));
        assert_eq!(a.0[2], felt!("3"));
        assert_eq!(a.0[3], felt!("4"));
    }

    #[test]
    fn empty_array() {
        // Array with only the length that is 0 (an other field as we're in cairo 0).
        // So the deserialization of the array starts at index 1.
        let serialized = vec![FieldElement::ZERO];
        let _a = CairoArrayLegacy::<FieldElement>::cairo_deserialize(&serialized, 1).unwrap();
    }
}
