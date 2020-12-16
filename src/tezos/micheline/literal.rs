use hex;
use serde::{Deserialize, Deserializer, Serialize, Serializer};

use super::{super::utils, super::utils::ConsumableHexStr, HexDecodable, HexEncodable, TzError};

#[derive(Serialize, Deserialize, PartialEq, Debug, Clone)]
#[serde(rename_all = "lowercase")]
pub enum Literal {
    String(String),

    #[serde(
        serialize_with = "literal_int_serializer",
        deserialize_with = "literal_int_deserializer"
    )]
    Int(i64),

    #[serde(
        serialize_with = "literal_bytes_serializer",
        deserialize_with = "literal_bytes_deserializer"
    )]
    Bytes(Vec<u8>),
}

fn literal_int_serializer<S>(int_value: &i64, s: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    s.serialize_str(&int_value.to_string())
}

fn literal_int_deserializer<'de, D>(d: D) -> Result<i64, D::Error>
where
    D: Deserializer<'de>,
{
    let int_value = String::deserialize(d)?;
    let int = int_value.parse::<i64>().map_err(|_error| {
        serde::de::Error::invalid_type(
            serde::de::Unexpected::Str(&int_value),
            &"a string representing a valid i64",
        )
    })?;

    Ok(int)
}

fn literal_bytes_serializer<S>(bytes: &[u8], s: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    let hex = hex::encode(bytes);
    s.serialize_str(&hex)
}

fn literal_bytes_deserializer<'de, D>(d: D) -> Result<Vec<u8>, D::Error>
where
    D: Deserializer<'de>,
{
    let hex = String::deserialize(d)?;
    let bytes = hex::decode(&hex).map_err(|_error| {
        serde::de::Error::invalid_type(serde::de::Unexpected::Str(&hex), &"a hex string")
    })?;

    Ok(bytes)
}

impl HexEncodable for Literal {
    fn to_hex_encoded(&self) -> Result<String, super::TzError> {
        match self {
            Literal::String(value) => Ok(Self::hex_encode_string(value)),
            Literal::Int(value) => Ok(Self::hex_encode_int(value)),
            Literal::Bytes(value) => Ok(Self::hex_encode_bytes(value)),
        }
    }
}

impl HexDecodable for Literal {
    fn from_hex(encoded: &mut ConsumableHexStr) -> Result<Self, TzError> {
        let prefix = MessagePrefix::from(encoded.read_bytes(1)?)?;
        match prefix {
            MessagePrefix::String => Literal::string_from_hex(encoded),
            MessagePrefix::Int => Literal::int_from_hex(encoded),
            MessagePrefix::Bytes => Literal::bytes_from_hex(encoded),
        }
    }
}

impl Literal {
    fn hex_encode_string(value: &str) -> String {
        let encoded = hex::encode(value.as_bytes());
        let length = encoded.len() / 2;

        format!(
            "{}{}{}",
            MessagePrefix::String.prefix(),
            utils::num_to_padded_str(length, None, None),
            encoded
        )
    }

    fn hex_encode_int(value: &i64) -> String {
        let mut absolute = value.abs();
        let mut bytes: Vec<u8> = vec![];

        let sign_mask: i64 = if value < &0 { 0b11000000 } else { 0b10000000 };

        bytes.push(((absolute & 0b00111111) | sign_mask) as u8);
        absolute >>= 6;

        while absolute != 0 {
            bytes.push(((absolute & 0b01111111) | 0b10000000) as u8);
            absolute >>= 7;
        }

        let length = bytes.len();

        bytes[length - 1] &= 0b01111111;

        bytes.iter().fold(
            String::from(MessagePrefix::Int.prefix()),
            |current, next| {
                let hex_value = utils::num_to_padded_str(*next, Some(2), None);
                format!("{}{}", current, hex_value)
            },
        )
    }

    fn hex_encode_bytes(value: &Vec<u8>) -> String {
        let length = utils::num_to_padded_str(value.len(), None, None);
        let hex_value = hex::encode(value);

        format!("{}{}{}", MessagePrefix::Bytes.prefix(), length, hex_value)
    }

    fn string_from_hex(encoded: &mut ConsumableHexStr) -> Result<Self, TzError> {
        encoded.consume_bytes(1)?; // consume prefix
        let value = encoded.consume_lengh_and_value(None)?;
        let bytes = hex::decode(value).map_err(|_error| TzError::InvalidType)?;
        let result = String::from_utf8(bytes).map_err(|_error| TzError::InvalidType)?;

        Ok(Literal::String(result))
    }

    fn int_from_hex(encoded: &mut ConsumableHexStr) -> Result<Self, TzError> {
        encoded.consume_bytes(1)?; // consume prefix
        let mut numbers: Vec<u8> = vec![];
        let mut current = encoded.consume_int(Some(1))? as u8;
        while (current & (1 << 7)) != 0 {
            numbers.push(current);
            current = encoded.consume_int(Some(1))? as u8;
        }

        numbers.push(current);
        let is_negative = (numbers[0] & (1 << 6)) != 0;
        numbers[0] &= 0b1111111;

        let mut binary_numbers: Vec<String> = numbers
            .iter()
            .enumerate()
            .map(|num| {
                let string_value = utils::num_to_padded_str(*num.1, Some(8), Some(2));
                let bit_length: usize = if num.0 == 0 { 6 } else { 7 };
                let start_index = std::cmp::max(string_value.len() - bit_length, 0);
                let slice = &string_value[start_index..];

                format!("{:0>width$}", slice, width = bit_length)
            })
            .collect();

        binary_numbers.reverse();
        let binary_string = binary_numbers.join("");
        let result =
            i64::from_str_radix(&binary_string, 2).map_err(|_error| TzError::InvalidType)?;

        let factor = if is_negative { -1 } else { 1 };

        Ok(Literal::Int(result * factor))
    }

    fn bytes_from_hex(encoded: &mut ConsumableHexStr) -> Result<Self, TzError> {
        encoded.consume_bytes(1)?; // consume prefix

        let value = encoded.consume_lengh_and_value(None)?;
        let bytes = hex::decode(value).map_err(|_error| TzError::InvalidType)?;

        Ok(Literal::Bytes(bytes))
    }
}

pub enum MessagePrefix {
    String,
    Int,
    Bytes,
}

impl MessagePrefix {
    pub fn from(value: &str) -> Result<Self, TzError> {
        match value {
            "01" => Ok(Self::String),
            "00" => Ok(Self::Int),
            "0a" => Ok(Self::Bytes),
            _ => Err(TzError::InvalidType),
        }
    }

    pub fn prefix(&self) -> &str {
        match self {
            Self::String => "01",
            Self::Int => "00",
            Self::Bytes => "0a",
        }
    }
}

#[cfg(test)]
mod test {
    use crate::tezos::micheline::TzError;

    use super::*;

    #[test]
    fn test_serialization() -> () {
        let string = Literal::String(String::from("Test"));
        let string_json = serde_json::json!(string).to_string();
        assert_eq!(string_json, r#"{"string":"Test"}"#);

        let int = Literal::Int(100);
        let int_json = serde_json::json!(int).to_string();
        assert_eq!(int_json, r#"{"int":"100"}"#);

        let bytes = Literal::Bytes(vec![10, 255, 5]);
        let bytes_json = serde_json::json!(bytes).to_string();
        assert_eq!(bytes_json, r#"{"bytes":"0aff05"}"#)
    }

    #[test]
    fn test_deserialization() -> Result<(), serde_json::Error> {
        let string_json = serde_json::json!({
            "string": "Test"
        });
        let string: Literal = serde_json::from_value(string_json)?;
        assert_eq!(string, Literal::String(String::from("Test")));

        let int_json = serde_json::json!({
            "int": "100"
        });
        let int: Literal = serde_json::from_value(int_json)?;
        assert_eq!(int, Literal::Int(100));

        let bytes_json = serde_json::json!({
            "bytes": "0aff05"
        });
        let bytes: Literal = serde_json::from_value(bytes_json)?;
        assert_eq!(bytes, Literal::Bytes(vec![10, 255, 5]));

        Ok(())
    }

    #[test]
    fn test_string_hex_encoding() -> Result<(), TzError> {
        let string = Literal::String(String::from("Test"));
        let hex_value = string.to_hex_encoded()?;
        assert_eq!(hex_value, "010000000454657374");

        Ok(())
    }

    #[test]
    fn test_int_hex_encoding_1() -> Result<(), TzError> {
        let int = Literal::Int(100);
        let hex_value = int.to_hex_encoded()?;
        assert_eq!(hex_value, "00a401");

        Ok(())
    }

    #[test]
    fn test_int_hex_encoding_2() -> Result<(), TzError> {
        let int = Literal::Int(100000);
        let hex_value = int.to_hex_encoded()?;
        assert_eq!(hex_value, "00a09a0c");

        Ok(())
    }

    #[test]
    fn test_bytes_hex_encoding() -> Result<(), TzError> {
        let bytes = Literal::Bytes(vec![0, 255, 100, 50, 1]);
        let hex_value = bytes.to_hex_encoded()?;
        assert_eq!(hex_value, "0a0000000500ff643201");

        Ok(())
    }

    #[test]
    fn test_string_hex_decoding() -> Result<(), TzError> {
        let mut encoded = ConsumableHexStr::new("010000000454657374");
        let value = Literal::from_hex(&mut encoded)?;
        assert_eq!(value, Literal::String(String::from("Test")));

        Ok(())
    }

    #[test]
    fn test_int_hex_decoding_1() -> Result<(), TzError> {
        let mut encoded = ConsumableHexStr::new("00a401");
        let value = Literal::from_hex(&mut encoded)?;

        assert_eq!(value, Literal::Int(100));

        Ok(())
    }

    #[test]
    fn test_int_hex_decoding_2() -> Result<(), TzError> {
        let mut encoded = ConsumableHexStr::new("00a09a0c");
        let value = Literal::from_hex(&mut encoded)?;

        assert_eq!(value, Literal::Int(100000));

        Ok(())
    }

    #[test]
    fn test_bytes_hex_decoding() -> Result<(), TzError> {
        let mut encoded = ConsumableHexStr::new("0a0000000500ff643201");
        let value = Literal::from_hex(&mut encoded)?;

        assert_eq!(value, Literal::Bytes(vec![0, 255, 100, 50, 1]));

        Ok(())
    }
}
