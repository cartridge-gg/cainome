//! Support for string compatibility with Cairo `ByteArray`.
//! <https://github.com/starkware-libs/cairo/blob/a4de08fbd75fa1d58c69d054d6b3d99aaf318f90/corelib/src/byte_array.cairo>
//!
//! The basic concept of this `ByteArray` is relying on a string being
//! represented as an array of bytes packed by 31 bytes in a felt.
//! To support any string even if the length is not a multiple of 31,
//! the `ByteArray` struct has a `pending_word` field, which is the last
//! word that is always shorter than 31 bytes.
//!
//! In the data structure, everything is represented as a felt to be compatible
//! with the Cairo implementation.
use std::{
    str::{self},
    string::FromUtf8Error,
};

use starknet::core::types::FieldElement;

use crate::error::{Error, Result as CainomeResult};
use crate::CairoSerde;

const MAX_WORD_LEN: usize = 31;

pub const BYTES31_MAX: FieldElement = FieldElement::from_mont([
    18446744062762287141,
    20123647,
    18446744073709514624,
    576460566199927480,
]);

#[derive(Debug, Clone, Eq, PartialEq, Default, serde::Serialize, serde::Deserialize)]
pub struct Bytes31(FieldElement);

impl Bytes31 {
    pub fn new(felt: FieldElement) -> CainomeResult<Self> {
        if felt > BYTES31_MAX {
            Err(Error::Bytes31OutOfRange)
        } else {
            Ok(Self(felt))
        }
    }

    pub fn felt(&self) -> FieldElement {
        self.0
    }
}

impl From<Bytes31> for FieldElement {
    fn from(value: Bytes31) -> Self {
        value.felt()
    }
}

impl TryFrom<FieldElement> for Bytes31 {
    type Error = Error;

    fn try_from(value: FieldElement) -> Result<Self, Self::Error> {
        Self::new(value)
    }
}

impl CairoSerde for Bytes31 {
    type RustType = Self;

    fn cairo_serialize(rust: &Self::RustType) -> Vec<FieldElement> {
        vec![rust.felt()]
    }

    fn cairo_deserialize(felts: &[FieldElement], offset: usize) -> CainomeResult<Self::RustType> {
        Self::new(felts[offset])
    }
}

#[derive(Debug, Clone, Eq, PartialEq, Default, serde::Serialize, serde::Deserialize)]
pub struct ByteArray {
    pub data: Vec<Bytes31>,
    pub pending_word: FieldElement,
    pub pending_word_len: usize,
}

impl CairoSerde for ByteArray {
    type RustType = Self;

    const SERIALIZED_SIZE: Option<usize> = None;

    fn cairo_serialized_size(rust: &Self::RustType) -> usize {
        let mut size = 0;
        size += Vec::<Bytes31>::cairo_serialized_size(&rust.data);
        size += FieldElement::cairo_serialized_size(&rust.pending_word);
        size += u32::cairo_serialized_size(&(rust.pending_word_len as u32));
        size
    }

    fn cairo_serialize(rust: &Self::RustType) -> Vec<FieldElement> {
        let mut out: Vec<FieldElement> = vec![];
        out.extend(Vec::<Bytes31>::cairo_serialize(&rust.data));
        out.extend(FieldElement::cairo_serialize(&rust.pending_word));
        out.extend(u32::cairo_serialize(&(rust.pending_word_len as u32)));
        out
    }

    fn cairo_deserialize(felts: &[FieldElement], offset: usize) -> CainomeResult<Self::RustType> {
        let mut offset = offset;
        let data = Vec::<Bytes31>::cairo_deserialize(felts, offset)?;
        offset += Vec::<Bytes31>::cairo_serialized_size(&data);
        let pending_word = FieldElement::cairo_deserialize(felts, offset)?;
        offset += FieldElement::cairo_serialized_size(&pending_word);
        let pending_word_len = u32::cairo_deserialize(felts, offset)?;

        Ok(ByteArray {
            data,
            pending_word,
            pending_word_len: pending_word_len as usize,
        })
    }
}

impl ByteArray {
    /// Converts a `String` into a `ByteArray`.
    /// The rust type `String` implies UTF-8 encoding,
    /// event if this function is not directly bound to this encoding.
    ///
    /// # Arguments
    ///
    /// * `string` - The always valid UTF-8 string to convert.
    pub fn from_string(string: &str) -> CainomeResult<Self> {
        let bytes = string.as_bytes();
        let chunks: Vec<_> = bytes.chunks(MAX_WORD_LEN).collect();

        let remainder = if bytes.len() % MAX_WORD_LEN != 0 {
            chunks.last().copied().map(|last| last.to_vec())
        } else {
            None
        };

        let full_chunks = if remainder.is_some() {
            &chunks[..chunks.len() - 1]
        } else {
            &chunks[..]
        };

        let (pending_word, pending_word_len) = if let Some(r) = remainder {
            let len = r.len();
            (
                // Safe to unwrap as pending word always fit in a felt.
                FieldElement::from_byte_slice_be(&r).unwrap(),
                len,
            )
        } else {
            (FieldElement::ZERO, 0)
        };

        let mut data = Vec::new();
        for chunk in full_chunks {
            // Safe to unwrap as full chunks are 31 bytes long, always fit in a felt.
            data.push(Bytes31::new(
                FieldElement::from_byte_slice_be(chunk).unwrap(),
            )?)
        }

        Ok(Self {
            data,
            pending_word,
            pending_word_len,
        })
    }

    /// Converts `ByteArray` instance into a UTF-8 encoded string on success.
    /// Returns error if the `ByteArray` contains an invalid UTF-8 string.
    pub fn to_string(&self) -> Result<String, FromUtf8Error> {
        let mut s = String::new();

        for d in &self.data {
            // Chunks are always 31 bytes long (MAX_WORD_LEN).
            s.push_str(&felt_to_utf8(&d.felt(), MAX_WORD_LEN)?);
        }

        if self.pending_word_len > 0 {
            s.push_str(&felt_to_utf8(&self.pending_word, self.pending_word_len)?);
        }

        Ok(s)
    }
}

/// Converts a felt into a UTF-8 string.
/// Returns an error if the felt contains an invalid UTF-8 string.
///
/// # Arguments
///
/// * `felt` - The `FieldElement` to convert. In the context of `ByteArray` this
///            felt always contains at most 31 bytes.
/// * `len` - The number of bytes in the felt, at most 31. In the context
///           of `ByteArray`, we don't need to check `len` as the `MAX_WORD_LEN`
///           already protect against that.
fn felt_to_utf8(felt: &FieldElement, len: usize) -> Result<String, FromUtf8Error> {
    let mut buffer = Vec::new();

    // ByteArray always enforce to have the first byte equal to 0.
    // That's why we start to 1.
    for byte in felt.to_bytes_be()[1 + MAX_WORD_LEN - len..].iter() {
        buffer.push(*byte)
    }

    String::from_utf8(buffer)
}

impl TryFrom<String> for ByteArray {
    type Error = Error;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        ByteArray::from_string(&value)
    }
}

impl TryFrom<&str> for ByteArray {
    type Error = Error;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        ByteArray::from_string(value)
    }
}

#[cfg(test)]
mod tests {
    use super::ByteArray;
    use starknet::core::types::FieldElement;

    #[test]
    fn test_from_string_empty_string_default() {
        let b = ByteArray::from_string("").unwrap();
        assert_eq!(b, ByteArray::default());
    }

    #[test]
    fn test_from_string_only_pending_word() {
        let b = ByteArray::from_string("ABCD").unwrap();
        assert_eq!(
            b,
            ByteArray {
                data: vec![],
                pending_word: FieldElement::from_hex_be(
                    "0x0000000000000000000000000000000000000000000000000000000041424344"
                )
                .unwrap(),
                pending_word_len: 4,
            }
        );
    }

    #[test]
    fn test_from_string_max_pending_word_len() {
        // pending word is at most 30 bytes long.
        let b = ByteArray::from_string("ABCDEFGHIJKLMNOPQRSTUVWXYZ1234").unwrap();

        assert_eq!(
            b,
            ByteArray {
                data: vec![],
                pending_word: FieldElement::from_hex_be(
                    "0x00004142434445464748494a4b4c4d4e4f505152535455565758595a31323334"
                )
                .unwrap(),
                pending_word_len: 30,
            }
        );
    }

    #[test]
    fn test_from_string_data_only() {
        let b = ByteArray::from_string("ABCDEFGHIJKLMNOPQRSTUVWXYZ12345").unwrap();

        assert_eq!(
            b,
            ByteArray {
                data: vec![FieldElement::from_hex_be(
                    "0x004142434445464748494a4b4c4d4e4f505152535455565758595a3132333435"
                )
                .unwrap()
                .try_into()
                .unwrap()],
                pending_word: FieldElement::ZERO,
                pending_word_len: 0,
            }
        );
    }

    #[test]
    fn test_from_string_data_only_multiple() {
        let b = ByteArray::from_string(
            "ABCDEFGHIJKLMNOPQRSTUVWXYZ12345ABCDEFGHIJKLMNOPQRSTUVWXYZ12345",
        )
        .unwrap();

        assert_eq!(
            b,
            ByteArray {
                data: vec![
                    FieldElement::from_hex_be(
                        "0x004142434445464748494a4b4c4d4e4f505152535455565758595a3132333435"
                    )
                    .unwrap()
                    .try_into()
                    .unwrap(),
                    FieldElement::from_hex_be(
                        "0x004142434445464748494a4b4c4d4e4f505152535455565758595a3132333435"
                    )
                    .unwrap()
                    .try_into()
                    .unwrap(),
                ],
                pending_word: FieldElement::ZERO,
                pending_word_len: 0,
            }
        );
    }

    #[test]
    fn test_from_string_data_and_pending_word() {
        let b = ByteArray::from_string(
            "ABCDEFGHIJKLMNOPQRSTUVWXYZ12345ABCDEFGHIJKLMNOPQRSTUVWXYZ12345ABCD",
        )
        .unwrap();

        assert_eq!(
            b,
            ByteArray {
                data: vec![
                    FieldElement::from_hex_be(
                        "0x004142434445464748494a4b4c4d4e4f505152535455565758595a3132333435"
                    )
                    .unwrap()
                    .try_into()
                    .unwrap(),
                    FieldElement::from_hex_be(
                        "0x004142434445464748494a4b4c4d4e4f505152535455565758595a3132333435"
                    )
                    .unwrap()
                    .try_into()
                    .unwrap(),
                ],
                pending_word: FieldElement::from_hex_be(
                    "0x0000000000000000000000000000000000000000000000000000000041424344"
                )
                .unwrap(),
                pending_word_len: 4,
            }
        );
    }

    #[test]
    fn test_to_string_empty_string_default() {
        let b = ByteArray::default();
        assert_eq!(b.to_string().unwrap(), "");
    }

    #[test]
    fn test_to_string_only_pending_word() {
        let b = ByteArray {
            data: vec![],
            pending_word: FieldElement::from_hex_be(
                "0x0000000000000000000000000000000000000000000000000000000041424344",
            )
            .unwrap(),
            pending_word_len: 4,
        };

        assert_eq!(b.to_string().unwrap(), "ABCD");
    }

    #[test]
    fn test_to_string_max_pending_word_len() {
        let b = ByteArray {
            data: vec![],
            pending_word: FieldElement::from_hex_be(
                "0x00004142434445464748494a4b4c4d4e4f505152535455565758595a31323334",
            )
            .unwrap(),
            pending_word_len: 30,
        };

        assert_eq!(b.to_string().unwrap(), "ABCDEFGHIJKLMNOPQRSTUVWXYZ1234");
    }

    #[test]
    fn test_to_string_data_only() {
        let b = ByteArray {
            data: vec![FieldElement::from_hex_be(
                "0x004142434445464748494a4b4c4d4e4f505152535455565758595a3132333435",
            )
            .unwrap()
            .try_into()
            .unwrap()],
            pending_word: FieldElement::ZERO,
            pending_word_len: 0,
        };

        assert_eq!(b.to_string().unwrap(), "ABCDEFGHIJKLMNOPQRSTUVWXYZ12345");
    }

    #[test]
    fn test_to_string_data_only_multiple() {
        let b = ByteArray {
            data: vec![
                FieldElement::from_hex_be(
                    "0x004142434445464748494a4b4c4d4e4f505152535455565758595a3132333435",
                )
                .unwrap()
                .try_into()
                .unwrap(),
                FieldElement::from_hex_be(
                    "0x004142434445464748494a4b4c4d4e4f505152535455565758595a3132333435",
                )
                .unwrap()
                .try_into()
                .unwrap(),
            ],
            pending_word: FieldElement::ZERO,
            pending_word_len: 0,
        };

        assert_eq!(
            b.to_string().unwrap(),
            "ABCDEFGHIJKLMNOPQRSTUVWXYZ12345ABCDEFGHIJKLMNOPQRSTUVWXYZ12345"
        );
    }

    #[test]
    fn test_to_string_data_and_pending_word() {
        let b = ByteArray {
            data: vec![
                FieldElement::from_hex_be(
                    "0x004142434445464748494a4b4c4d4e4f505152535455565758595a3132333435",
                )
                .unwrap()
                .try_into()
                .unwrap(),
                FieldElement::from_hex_be(
                    "0x004142434445464748494a4b4c4d4e4f505152535455565758595a3132333435",
                )
                .unwrap()
                .try_into()
                .unwrap(),
            ],
            pending_word: FieldElement::from_hex_be(
                "0x0000000000000000000000000000000000000000000000000000000041424344",
            )
            .unwrap(),
            pending_word_len: 4,
        };

        assert_eq!(
            b.to_string().unwrap(),
            "ABCDEFGHIJKLMNOPQRSTUVWXYZ12345ABCDEFGHIJKLMNOPQRSTUVWXYZ12345ABCD"
        );
    }

    #[test]
    #[should_panic]
    fn test_to_string_invalid_utf8() {
        let b = ByteArray {
            data: vec![],
            pending_word: FieldElement::from_hex_be(
                "0x00000000000000000000000000000000000000000000000000000000ffffffff",
            )
            .unwrap(),
            pending_word_len: 4,
        };

        b.to_string().unwrap();
    }

    #[test]
    fn test_from_utf8() {
        let b: ByteArray = "🦀🌟".try_into().unwrap();

        assert_eq!(
            b,
            ByteArray {
                data: vec![],
                pending_word: FieldElement::from_hex_be(
                    "0x000000000000000000000000000000000000000000000000f09fa680f09f8c9f",
                )
                .unwrap(),
                pending_word_len: 8,
            }
        );
    }
}
