use crate::CairoSerde;
use num::{bigint::ParseBigIntError, BigUint, Num};
use serde_with::{DeserializeAs, DisplayFromStr, SerializeAs};
use starknet::core::types::{Felt, U256 as StarknetU256};
use std::{
    cmp::Ordering,
    fmt::Display,
    ops::{Add, BitOr, Mul, Sub},
    str::FromStr,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct U256 {
    pub low: u128,
    pub high: u128,
}

impl U256 {
    pub const ZERO: U256 = Self::from_u128(0);
    pub const ONE: U256 = Self::from_u128(1);
    pub const TWO: U256 = Self::from_u128(2);

    const fn from_u128(value: u128) -> Self {
        U256 {
            low: value,
            high: 0,
        }
    }
}

macro_rules! impl_from_for_u256 {
    ($($t:ty),*) => {
        $(
            impl From<$t> for U256 {
                fn from(value: $t) -> Self {
                    Self::from_u128(value as u128)
                }
            }
        )*
    };
}

impl_from_for_u256!(u8, u16, u32, u64, u128, usize);

impl From<StarknetU256> for U256 {
    fn from(value: StarknetU256) -> Self {
        Self {
            low: value.low(),
            high: value.high(),
        }
    }
}

impl From<U256> for StarknetU256 {
    fn from(value: U256) -> Self {
        StarknetU256::from_words(value.low, value.high)
    }
}

impl PartialOrd for U256 {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(match self.high.cmp(&other.high) {
            Ordering::Equal => self.low.cmp(&other.low),
            ordering => ordering,
        })
    }
}

impl Add for U256 {
    type Output = Self;
    fn add(mut self, other: Self) -> Self {
        let (low, overflow_low) = self.low.overflowing_add(other.low);
        if overflow_low {
            self.high += 1;
        }
        let (high, _overflow_high) = self.high.overflowing_add(other.high);
        U256 { low, high }
    }
}

impl Sub for U256 {
    type Output = Self;
    fn sub(self, other: Self) -> Self {
        let (low, overflow_low) = self.low.overflowing_sub(other.low);
        let (high, overflow_high) = self.high.overflowing_sub(other.high);
        if overflow_high {
            panic!("High underflow");
        }
        let final_high = if overflow_low {
            if high == 0 {
                panic!("High underflow");
            }
            high.wrapping_sub(1)
        } else {
            high
        };
        U256 {
            low,
            high: final_high,
        }
    }
}

impl Mul for U256 {
    type Output = Self;

    fn mul(self, rhs: Self) -> Self::Output {
        (StarknetU256::from(self) * StarknetU256::from(rhs)).into()
    }
}

impl BitOr for U256 {
    type Output = Self;

    fn bitor(self, other: Self) -> Self {
        U256 {
            low: self.low | other.low,
            high: self.high | other.high,
        }
    }
}

impl Display for U256 {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut num = BigUint::from(0u128);
        num += BigUint::from(self.high);
        num <<= 128;
        num += BigUint::from(self.low);
        write!(f, "0x{}", num.to_str_radix(16))
    }
}

impl FromStr for U256 {
    type Err = ParseBigIntError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let num = if s.len() >= 2 && &s[0..2] == "0x" {
            BigUint::from_str_radix(&s[2..], 16)
        } else {
            BigUint::from_str(s)
        }
        .unwrap();
        let mask = (BigUint::from(1u128) << 128u32) - BigUint::from(1u128);
        let b_low: BigUint = (num.clone() >> 0) & mask.clone();
        let b_high: BigUint = (num.clone() >> 128) & mask.clone();

        let mut low = 0;
        let mut high = 0;

        for (i, digit) in b_low.to_u64_digits().iter().take(2).enumerate() {
            low |= (*digit as u128) << (i * 64);
        }

        for (i, digit) in b_high.to_u64_digits().iter().take(2).enumerate() {
            high |= (*digit as u128) << (i * 64);
        }

        Ok(U256 { low, high })
    }
}

impl serde::Serialize for U256 {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        DisplayFromStr::serialize_as(self, serializer)
    }
}

impl<'de> serde::Deserialize<'de> for U256 {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        DisplayFromStr::deserialize_as(deserializer)
    }
}

impl CairoSerde for U256 {
    type RustType = Self;

    const SERIALIZED_SIZE: Option<usize> = Some(2);
    const DYNAMIC: bool = false;

    #[inline]
    fn cairo_serialized_size(this: &U256) -> usize {
        u128::cairo_serialized_size(&this.low) + u128::cairo_serialized_size(&this.high)
    }
    fn cairo_serialize(this: &U256) -> Vec<Felt> {
        [
            u128::cairo_serialize(&this.low),
            u128::cairo_serialize(&this.high),
        ]
        .concat()
    }
    fn cairo_deserialize(felts: &[Felt], offset: usize) -> Result<U256, crate::Error> {
        let low = u128::cairo_deserialize(felts, offset)?;
        let high = u128::cairo_deserialize(felts, offset + u128::cairo_serialized_size(&low))?;
        Ok(U256 { low, high })
    }
}

#[derive(Debug, thiserror::Error)]
#[error("Value out of range")]
pub struct ValueOutOfRangeError;

/// Felt to U256 conversion as if the tuple was a cairo serialized U256
impl TryFrom<(Felt, Felt)> for U256 {
    type Error = ValueOutOfRangeError;
    fn try_from((a, b): (Felt, Felt)) -> Result<U256, Self::Error> {
        let U256 {
            low: a_low,
            high: a_high,
        } = U256::from_bytes_be(&a.to_bytes_be());
        let U256 {
            low: b_low,
            high: b_high,
        } = U256::from_bytes_be(&b.to_bytes_be());
        if b_high != 0 || a_high != 0 {
            return Err(ValueOutOfRangeError);
        }
        Ok(U256 {
            low: a_low,
            high: b_low,
        })
    }
}

impl U256 {
    pub fn to_bytes_be(&self) -> [u8; 32] {
        let mut bytes = [0; 32];
        bytes[0..16].copy_from_slice(&self.high.to_be_bytes());
        bytes[16..32].copy_from_slice(&self.low.to_be_bytes());
        bytes
    }
    pub fn to_bytes_le(&self) -> [u8; 32] {
        let mut bytes = [0; 32];
        bytes[0..16].copy_from_slice(&self.low.to_le_bytes());
        bytes[16..32].copy_from_slice(&self.high.to_le_bytes());
        bytes
    }
    pub fn from_bytes_be(bytes: &[u8; 32]) -> Self {
        let high = u128::from_be_bytes(bytes[0..16].try_into().unwrap());
        let low = u128::from_be_bytes(bytes[16..32].try_into().unwrap());
        U256 { low, high }
    }
    pub fn from_bytes_le(bytes: &[u8; 32]) -> Self {
        let low = u128::from_le_bytes(bytes[0..16].try_into().unwrap());
        let high = u128::from_le_bytes(bytes[16..32].try_into().unwrap());
        U256 { low, high }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_serialize_u256() {
        let low = 9_u128;
        let high = 8_u128;
        let felts = U256::cairo_serialize(&U256 { low, high });
        assert_eq!(felts.len(), 2);
        assert_eq!(felts[0], Felt::from(9_u128));
        assert_eq!(felts[1], Felt::from(8_u128));
    }

    #[test]
    fn test_add_u256_low_overflow() {
        let u256_1 = U256 {
            low: u128::MAX,
            high: 1_u128,
        };
        let u256_2 = U256 {
            low: 1_u128,
            high: 2_u128,
        };
        let u256_3 = u256_1 + u256_2;
        assert_eq!(u256_3.low, 0_u128);
        assert_eq!(u256_3.high, 4_u128);
    }

    #[test]
    fn test_add_u256_high_overflow() {
        let u256_1 = U256 {
            low: 0_u128,
            high: u128::MAX,
        };
        let u256_2 = U256 {
            low: 0_u128,
            high: 1_u128,
        };

        let u256_3 = u256_1 + u256_2;

        assert_eq!(u256_3.low, 0_u128);
        assert_eq!(u256_3.high, 0_u128);
    }

    #[test]
    fn test_sub_u256() {
        let u256_1 = U256 {
            low: 1_u128,
            high: 2_u128,
        };
        let u256_2 = U256 {
            low: 0_u128,
            high: 1_u128,
        };
        let u256_3 = u256_1 - u256_2;
        assert_eq!(u256_3.low, 1_u128);
        assert_eq!(u256_3.high, 1_u128);
    }

    #[test]
    fn test_sub_u256_underflow_low() {
        let u256_1 = U256 {
            low: 0_u128,
            high: 1_u128,
        };
        let u256_2 = U256 {
            low: 2_u128,
            high: 0_u128,
        };
        let u256_3 = u256_1 - u256_2;
        assert_eq!(u256_3.low, u128::MAX - 1);
        assert_eq!(u256_3.high, 0_u128);
    }

    #[test]
    #[should_panic]
    fn test_sub_u256_underflow_high() {
        let u256_1 = U256 {
            low: 0_u128,
            high: 1_u128,
        };
        let u256_2 = U256 {
            low: 0_u128,
            high: 2_u128,
        };
        let _u256_3 = u256_1 - u256_2;
    }

    #[test]
    #[should_panic]
    fn test_sub_u256_underflow_high_2() {
        let u256_1 = U256 {
            low: 10_u128,
            high: 2_u128,
        };
        let u256_2 = U256 {
            low: 11_u128,
            high: 2_u128,
        };
        let _u256_3 = u256_1 - u256_2;
    }

    #[test]
    fn test_bit_or_u256() {
        let u256_1 = U256 {
            low: 0b1010_u128,
            high: 0b1100_u128,
        };
        let u256_2 = U256 {
            low: 0b0110_u128,
            high: 0b0011_u128,
        };
        let u256_3 = u256_1 | u256_2;
        assert_eq!(u256_3.low, 0b1110_u128);
        assert_eq!(u256_3.high, 0b1111_u128)
    }

    #[test]
    fn test_serialize_u256_max() {
        let low = u128::MAX;
        let high = u128::MAX;
        let felts = U256::cairo_serialize(&U256 { low, high });
        assert_eq!(felts.len(), 2);
        assert_eq!(felts[0], Felt::from(u128::MAX));
        assert_eq!(felts[1], Felt::from(u128::MAX));
    }

    #[test]
    fn test_serialize_u256_min() {
        let low = u128::MIN;
        let high = u128::MIN;
        let felts = U256::cairo_serialize(&U256 { low, high });
        assert_eq!(felts.len(), 2);
        assert_eq!(felts[0], Felt::from(u128::MIN));
        assert_eq!(felts[1], Felt::from(u128::MIN));
    }

    #[test]
    fn test_display_u256() {
        let u256 = U256 {
            low: 12_u128,
            high: 0_u128,
        };
        println!("{}", u256);
        assert_eq!(format!("{}", u256), "0xc");
    }

    #[test]
    fn test_from_str() {
        let u256 = U256::from_str("18446744073709551616").unwrap();
        assert_eq!(u256.low, 18446744073709551616_u128);
        assert_eq!(u256.high, 0_u128);
    }

    #[test]
    fn test_deserialize_u256() {
        let felts = vec![Felt::from(9_u128), Felt::from(8_u128)];
        let num_u256 = U256::cairo_deserialize(&felts, 0).unwrap();
        assert_eq!(num_u256.low, 9_u128);
        assert_eq!(num_u256.high, 8_u128);
    }

    #[test]
    fn test_serialized_size_u256() {
        let u256 = U256 {
            low: 9_u128,
            high: 8_u128,
        };
        assert_eq!(U256::cairo_serialized_size(&u256), 2);
    }

    #[test]
    fn test_to_bytes_be() {
        let u256 = U256 {
            low: 9_u128,
            high: 8_u128,
        };
        let bytes = u256.to_bytes_be();
        let expected_bytes: [u8; 32] = [
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 8, // high
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 9, // low
        ];
        assert_eq!(bytes, expected_bytes);
    }

    #[test]
    fn test_to_bytes_le() {
        let u256 = U256 {
            low: 9_u128,
            high: 8_u128,
        };
        let bytes = u256.to_bytes_le();
        let expected_bytes: [u8; 32] = [
            9, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, // low
            8, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, // high
        ];
        assert_eq!(bytes, expected_bytes);
    }

    #[test]
    fn test_from_bytes_be() {
        let bytes: [u8; 32] = [
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 8, // high
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 9, // low
        ];
        let u256 = U256::from_bytes_be(&bytes);
        assert_eq!(u256.low, 9_u128);
        assert_eq!(u256.high, 8_u128);
    }

    #[test]
    fn test_from_bytes_le() {
        let bytes: [u8; 32] = [
            9, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, // low
            8, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, // high
        ];
        let u256 = U256::from_bytes_le(&bytes);
        assert_eq!(u256.low, 9_u128);
        assert_eq!(u256.high, 8_u128);
    }

    #[test]
    fn test_from_field_element() {
        let felts = (Felt::from(9_u128), Felt::from(8_u128));
        let u256 = U256::try_from(felts).unwrap();
        assert_eq!(u256.low, 9_u128);
        assert_eq!(u256.high, 8_u128);
    }

    #[test]
    fn test_ordering_1() {
        let u256_1 = U256 {
            low: 9_u128,
            high: 8_u128,
        };
        let u256_2 = U256 {
            low: 0_u128,
            high: 9_u128,
        };
        assert!(u256_1 < u256_2);
    }

    #[test]
    fn test_ordering_2() {
        let u256_1 = U256 {
            low: 9_u128,
            high: 8_u128,
        };
        let u256_2 = U256 {
            low: 9_u128,
            high: 8_u128,
        };
        assert!(u256_1 == u256_2);
    }

    #[test]
    fn test_ordering_3() {
        let u256_1 = U256 {
            low: 8_u128,
            high: 9_u128,
        };
        let u256_2 = U256 {
            low: 9_u128,
            high: 9_u128,
        };
        assert!(u256_1 < u256_2);
    }
}
