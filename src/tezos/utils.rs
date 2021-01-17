use num_bigint::BigUint;
use num_traits::ToPrimitive;
use radix_fmt;
use std::fmt::Display;

use super::TzError;

pub fn num_to_padded_str<T>(value: T, length: Option<usize>, radix: Option<u8>) -> String
where
    radix_fmt::Radix<T>: Display,
{
    let pad_length = length.unwrap_or(8);
    let output_radix = radix.unwrap_or(16);
    let value_to_pad = radix_fmt::radix(value, output_radix);

    format!("{:0>width$}", &value_to_pad.to_string(), width = pad_length)
}

pub struct ConsumableHexStr<'a> {
    str: &'a str,
    position: usize,
}

impl<'a> ConsumableHexStr<'a> {
    pub fn new(str: &'a str) -> Self {
        ConsumableHexStr {
            str: str,
            position: 0,
        }
    }

    pub fn consume_bytes(&mut self, size: usize) -> Result<&'a str, TzError> {
        let end_index = self.position + (size * 2);
        let result = self.read_to(end_index)?;
        self.position = end_index;

        Ok(result)
    }

    pub fn consume_lengh_and_value(&mut self, bytes: Option<usize>) -> Result<&'a str, TzError> {
        let length = self.consume_int(bytes)?;
        self.consume_bytes(length as usize)
    }

    pub fn consume_int(&mut self, bytes: Option<usize>) -> Result<i64, TzError> {
        let int_hex = self.consume_bytes(bytes.unwrap_or(4))?;
        let int = i64::from_str_radix(int_hex, 16).map_err(|_error| TzError::InvalidType)?;

        Ok(int)
    }

    pub fn read_bytes(&self, size: usize) -> Result<&'a str, TzError> {
        let end_index = self.position + (size * 2);
        self.read_to(end_index)
    }

    pub fn has_more(&self) -> bool {
        self.position < self.str.len()
    }

    fn read_to(&self, index: usize) -> Result<&'a str, TzError> {
        if index > self.str.len() {
            return Err(TzError::InvalidIndex);
        }

        Ok(&self.str[self.position..index])
    }
}

pub fn biguint_to_u8(a: &BigUint) -> u8 {
    let mask = BigUint::from(u8::MAX);
    (a & mask).to_u8().unwrap()
}

#[cfg(test)]
mod test {
    use super::num_to_padded_str;

    #[test]
    fn test_padding() -> () {
        let padded = num_to_padded_str(255, None, None);

        assert_eq!("000000ff", padded);
    }
}
