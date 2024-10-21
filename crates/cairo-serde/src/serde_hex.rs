use serde::ser::SerializeSeq;
use std::num::ParseIntError;

pub trait FromStrHexOrDec: Sized {
    fn from_str_hex_or_dec(s: &str) -> Result<Self, ParseIntError>;
}

impl FromStrHexOrDec for u64 {
    fn from_str_hex_or_dec(s: &str) -> Result<Self, ParseIntError> {
        if s.starts_with("0x") || s.starts_with("0X") {
            u64::from_str_radix(&s[2..], 16)
        } else {
            s.parse::<u64>()
        }
    }
}

impl FromStrHexOrDec for u128 {
    fn from_str_hex_or_dec(s: &str) -> Result<Self, ParseIntError> {
        if s.starts_with("0x") || s.starts_with("0X") {
            u128::from_str_radix(&s[2..], 16)
        } else {
            s.parse::<u128>()
        }
    }
}

impl FromStrHexOrDec for i64 {
    fn from_str_hex_or_dec(s: &str) -> Result<Self, ParseIntError> {
        u64::from_str_hex_or_dec(s).map(|v| v as i64)
    }
}

impl FromStrHexOrDec for i128 {
    fn from_str_hex_or_dec(s: &str) -> Result<Self, ParseIntError> {
        u128::from_str_hex_or_dec(s).map(|v| v as i128)
    }
}

/// Serialize a value as a hex string.
pub fn serialize_as_hex<S, T>(value: &T, serializer: S) -> std::result::Result<S::Ok, S::Error>
where
    S: serde::Serializer,
    T: serde::Serialize + std::fmt::LowerHex,
{
    serializer.serialize_str(&format!("{:#x}", value))
}

/// Serialize a vector of values as a hex string.
pub fn serialize_as_hex_vec<S, T>(
    value: &Vec<T>,
    serializer: S,
) -> std::result::Result<S::Ok, S::Error>
where
    S: serde::Serializer,
    T: serde::Serialize + std::fmt::LowerHex,
{
    let mut seq = serializer.serialize_seq(Some(value.len()))?;
    for v in value {
        seq.serialize_element(&format!("{:#x}", v))?;
    }
    seq.end()
}

/// Serialize a tuple of two values as a hex string.
pub fn serialize_as_hex_t2<S, T1, T2>(
    value: &(T1, T2),
    serializer: S,
) -> std::result::Result<S::Ok, S::Error>
where
    S: serde::Serializer,
    T1: serde::Serialize + std::fmt::LowerHex,
    T2: serde::Serialize + std::fmt::LowerHex,
{
    let mut seq = serializer.serialize_seq(Some(2))?;
    seq.serialize_element(&format!("{:#x}", value.0))?;
    seq.serialize_element(&format!("{:#x}", value.1))?;
    seq.end()
}

/// Serialize a tuple of three values as a hex string.
pub fn serialize_as_hex_t3<S, T1, T2, T3>(
    value: &(T1, T2, T3),
    serializer: S,
) -> std::result::Result<S::Ok, S::Error>
where
    S: serde::Serializer,
    T1: serde::Serialize + std::fmt::LowerHex,
    T2: serde::Serialize + std::fmt::LowerHex,
    T3: serde::Serialize + std::fmt::LowerHex,
{
    let mut seq = serializer.serialize_seq(Some(2))?;
    seq.serialize_element(&format!("{:#x}", value.0))?;
    seq.serialize_element(&format!("{:#x}", value.1))?;
    seq.serialize_element(&format!("{:#x}", value.2))?;
    seq.end()
}

/// Deserialize a single hex string into a value.
pub fn deserialize_from_hex<'de, D, T>(deserializer: D) -> std::result::Result<T, D::Error>
where
    D: serde::Deserializer<'de>,
    T: serde::Deserialize<'de> + FromStrHexOrDec,
{
    let hex_string: String = serde::Deserialize::deserialize(deserializer)?;
    T::from_str_hex_or_dec(&hex_string).map_err(serde::de::Error::custom)
}

/// Deserialize a vector of hex strings into values.
pub fn deserialize_from_hex_vec<'de, D, T>(deserializer: D) -> std::result::Result<Vec<T>, D::Error>
where
    D: serde::Deserializer<'de>,
    T: serde::Deserialize<'de> + FromStrHexOrDec,
{
    let hex_strings: Vec<String> = serde::Deserialize::deserialize(deserializer)?;
    hex_strings
        .into_iter()
        .map(|s| T::from_str_hex_or_dec(&s).map_err(serde::de::Error::custom))
        .collect()
}

/// Deserialize a string into a value, trying first to use `from_str` default.
/// If it fails, tries to parse as a hex string.
macro_rules! deserialize_hex {
    ($hex_string:expr, $type:ty) => {
        <$type>::from_str($hex_string).or_else(|_| {
            let hex_string = $hex_string.trim_start_matches("0x");
            u128::from_str_radix(hex_string, 16)
                .map(|num| num.to_string())
                .or_else(|_| Ok(hex_string.to_string()))
                .and_then(|dec_string| {
                    <$type>::from_str(&dec_string).map_err(serde::de::Error::custom)
                })
        })
    };
}

/// Deserialize a tuple of two hex strings into values.
/// For tuples, we can't enforce all the elements to implement `FromStrHexOrDec`
/// in this naive implementation.
pub fn deserialize_from_hex_t2<'de, D, T1, T2>(
    deserializer: D,
) -> std::result::Result<(T1, T2), D::Error>
where
    D: serde::Deserializer<'de>,
    T1: serde::Deserialize<'de> + std::str::FromStr,
    T2: serde::Deserialize<'de> + std::str::FromStr,
    <T1 as std::str::FromStr>::Err: std::fmt::Display,
    <T2 as std::str::FromStr>::Err: std::fmt::Display,
{
    let hex_strings: (String, String) = serde::Deserialize::deserialize(deserializer)?;

    let v1 = deserialize_hex!(&hex_strings.0, T1)?;
    let v2 = deserialize_hex!(&hex_strings.1, T2)?;

    Ok((v1, v2))
}

/// Deserialize a tuple of three hex strings into values.
/// For tuples, we can't enforce all the elements to implement `FromStrHexOrDec`
/// in this naive implementation.
pub fn deserialize_from_hex_t3<'de, D, T1, T2, T3>(
    deserializer: D,
) -> std::result::Result<(T1, T2, T3), D::Error>
where
    D: serde::Deserializer<'de>,
    T1: serde::Deserialize<'de> + std::str::FromStr,
    T2: serde::Deserialize<'de> + std::str::FromStr,
    T3: serde::Deserialize<'de> + std::str::FromStr,
    <T1 as std::str::FromStr>::Err: std::fmt::Display,
    <T2 as std::str::FromStr>::Err: std::fmt::Display,
    <T3 as std::str::FromStr>::Err: std::fmt::Display,
{
    let hex_strings: (String, String, String) = serde::Deserialize::deserialize(deserializer)?;

    let v1 = deserialize_hex!(&hex_strings.0, T1)?;
    let v2 = deserialize_hex!(&hex_strings.1, T2)?;
    let v3 = deserialize_hex!(&hex_strings.2, T3)?;
    Ok((v1, v2, v3))
}
