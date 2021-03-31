use std::{
    convert::{self, TryFrom, TryInto},
    fmt::Display,
};

use num_bigint::BigInt;
use primitive::Primitive;
use serde::{Deserialize, Serialize};

use super::utils;
use super::utils::ConsumableHexStr;
use super::{coding, TzError};

pub mod data;
pub mod instructions;
pub mod literal;
pub mod prim;
pub mod primitive;
pub mod types;

pub fn string(value: String) -> MichelsonV1Expression {
    MichelsonV1Expression::Literal(literal::Literal::String(value))
}

// pub fn int(value: BigInt) -> MichelsonV1Expression {
//     MichelsonV1Expression::Literal(literal::Literal::Int(value))
// }

pub fn int<T>(value: T) -> MichelsonV1Expression
where
    T: convert::Into<BigInt>,
{
    MichelsonV1Expression::Literal(literal::Literal::Int(value.into()))
}

pub fn bytes(value: Vec<u8>) -> MichelsonV1Expression {
    MichelsonV1Expression::Literal(literal::Literal::Bytes(value))
}

pub fn sequence(items: Vec<MichelsonV1Expression>) -> MichelsonV1Expression {
    MichelsonV1Expression::Sequence(items)
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Clone)]
#[serde(untagged)]
pub enum MichelsonV1Expression {
    Prim(prim::Prim),
    Literal(literal::Literal),
    Sequence(Vec<MichelsonV1Expression>),
}

static PACK_PREFIX: &str = "05";

impl MichelsonV1Expression {
    pub fn pack(&self, schema: Option<&MichelsonV1Expression>) -> Result<String, TzError> {
        let encoded: String;
        if let Some(schema) = schema {
            let normalized_schema = schema.clone().normalized();
            let packed = self.prepack(&normalized_schema)?;
            encoded = packed.to_hex_encoded()?;
        } else {
            encoded = self.to_hex_encoded()?;
        }

        Ok(format!("{}{}", PACK_PREFIX, encoded))
    }

    pub fn prepack(
        &self,
        schema: &MichelsonV1Expression,
    ) -> Result<MichelsonV1Expression, TzError> {
        use primitive::Type;
        let (type_, args, _) = schema.type_info()?;
        let string_value =
            if let MichelsonV1Expression::Literal(literal::Literal::String(value)) = self {
                Some(value)
            } else {
                None
            };
        Ok(match type_ {
            Type::List | Type::Set => self.prepack_sequence(args)?,
            Type::Map | Type::BigMap => self.prepack_map(args)?,
            Type::Lambda => self.prepack_lambda()?,
            Type::Pair => self.prepack_pair(args)?,
            Type::Option => {
                if let Some(prepacked) = self.prepack_option(args) {
                    prepacked?
                } else {
                    self.clone()
                }
            }
            Type::Or => self.prepack_or(args)?,
            Type::ChainID => {
                if let Some(value) = string_value {
                    bytes(coding::encode_chain_id(value)?)
                } else {
                    self.clone()
                }
            }
            Type::Signature => {
                if let Some(value) = string_value {
                    bytes(coding::encode_signature(value)?)
                } else {
                    self.clone()
                }
            }
            Type::KeyHash => {
                if let Some(value) = string_value {
                    bytes(coding::encode_address(value, true)?)
                } else {
                    self.clone()
                }
            }
            Type::Key => {
                if let Some(value) = string_value {
                    bytes(coding::encode_public_key(value)?)
                } else {
                    self.clone()
                }
            }
            Type::Address | Type::Contract => {
                if let Some(value) = string_value {
                    bytes(coding::encode_contract(value)?)
                } else {
                    self.clone()
                }
            }
            Type::Timestamp => {
                if let Some(value) = string_value {
                    int(coding::encode_timestamp(value)?)
                } else {
                    self.clone()
                }
            }
            _ => self.clone(),
        })
    }

    fn normalized(self) -> Self {
        match self {
            MichelsonV1Expression::Prim(prim) => MichelsonV1Expression::Prim(prim.normalized()),
            _ => self,
        }
    }

    fn prepack_sequence(
        &self,
        args: Option<&Vec<MichelsonV1Expression>>,
    ) -> Result<MichelsonV1Expression, TzError> {
        let sequence_types = args.ok_or(TzError::InvalidType)?;
        if let MichelsonV1Expression::Sequence(value) = self {
            if sequence_types.len() != 1 {
                return Err(TzError::InvalidType);
            }
            let prepacked: Vec<MichelsonV1Expression> = value
                .iter()
                .map(|item| item.prepack(&sequence_types[0]))
                .collect::<Result<Vec<MichelsonV1Expression>, TzError>>()?;

            Ok(MichelsonV1Expression::Sequence(prepacked))
        } else {
            Err(TzError::InvalidType)
        }
    }

    fn prepack_map(
        &self,
        args: Option<&Vec<MichelsonV1Expression>>,
    ) -> Result<MichelsonV1Expression, TzError> {
        use primitive::Data;

        if let MichelsonV1Expression::Sequence(value) = self {
            let prepacked: Result<Vec<MichelsonV1Expression>, TzError> = value
                .iter()
                .map(|item| {
                    if let MichelsonV1Expression::Prim(elt) = item {
                        let map_types = args.ok_or(TzError::InvalidType)?;

                        if elt.prim != Primitive::Data(Data::Elt)
                            || elt.args_count() != map_types.len()
                        {
                            return Err(TzError::InvalidType);
                        }

                        let arguments: Option<Vec<MichelsonV1Expression>> = elt
                            .args
                            .as_ref()
                            .and_then(|args| {
                                Some(
                                    args.iter()
                                        .enumerate()
                                        .map(|(index, argument)| {
                                            argument.prepack(&map_types[index])
                                        })
                                        .collect::<Result<Vec<MichelsonV1Expression>, TzError>>(),
                                )
                            })
                            .map_or(Ok(None), |r| r.map(Some))?;

                        Ok(data::prim(Data::Elt, arguments))
                    } else {
                        Err(TzError::InvalidType)
                    }
                })
                .collect();

            Ok(sequence(prepacked?))
        } else {
            Err(TzError::InvalidType)
        }
    }

    fn prepack_lambda(&self) -> Result<MichelsonV1Expression, TzError> {
        if let MichelsonV1Expression::Sequence(values) = self {
            let packed = values
                .iter()
                .map(|value| match value {
                    MichelsonV1Expression::Prim(prim) => {
                        Ok(MichelsonV1Expression::Prim(prim.prepack_instruction()?))
                    }
                    MichelsonV1Expression::Literal(_) => Err(TzError::InvalidType),
                    MichelsonV1Expression::Sequence(_) => value.prepack_lambda(),
                })
                .collect::<Result<Vec<MichelsonV1Expression>, TzError>>();

            Ok(sequence(packed?))
        } else {
            Err(TzError::InvalidType)
        }
    }

    fn prepack_pair(
        &self,
        args: Option<&Vec<MichelsonV1Expression>>,
    ) -> Result<MichelsonV1Expression, TzError> {
        use primitive::Data;

        match self {
            MichelsonV1Expression::Prim(value) => {
                let pair_types = args.ok_or(TzError::InvalidType)?;

                let value = value.clone().normalized();

                if value.prim != Primitive::Data(Data::Pair)
                    || value.args_count() != pair_types.len()
                {
                    return Err(TzError::InvalidType);
                }

                let arguments: Option<Vec<MichelsonV1Expression>> = value
                    .args
                    .as_ref()
                    .and_then(|args| {
                        Some(
                            args.iter()
                                .enumerate()
                                .map(|(index, argument)| argument.prepack(&pair_types[index]))
                                .collect::<Result<Vec<MichelsonV1Expression>, TzError>>(),
                        )
                    })
                    .map_or(Ok(None), |r| r.map(Some))?;

                Ok(data::prim(Data::Pair, arguments))
            }
            MichelsonV1Expression::Sequence(values) => {
                let pair = MichelsonV1Expression::Prim(prim::Prim::new(
                    primitive::Primitive::Data(Data::Pair),
                    Some(values.clone()),
                    None,
                ))
                .normalized();
                pair.prepack_pair(args)
            }
            MichelsonV1Expression::Literal(_) => Err(TzError::InvalidType),
        }
    }

    fn prepack_option(
        &self,
        args: Option<&Vec<MichelsonV1Expression>>,
    ) -> Option<Result<MichelsonV1Expression, TzError>> {
        use primitive::Data;

        if let MichelsonV1Expression::Prim(value) = self {
            if value.prim != Primitive::Data(Data::Some) {
                return None;
            }

            if let None = args {
                return Some(Err(TzError::InvalidType));
            }

            let option_types = args.unwrap();

            if value.args_count() != option_types.len() && option_types.len() == 1 {
                return Some(Err(TzError::InvalidType));
            }

            let arguments: Result<Option<Vec<MichelsonV1Expression>>, TzError> = value
                .args
                .as_ref()
                .and_then(|args| {
                    Some(
                        args.iter()
                            .enumerate()
                            .map(|(index, argument)| argument.prepack(&option_types[index]))
                            .collect::<Result<Vec<MichelsonV1Expression>, TzError>>(),
                    )
                })
                .map_or(Ok(None), |r| r.map(Some));

            if let Err(error) = arguments {
                return Some(Err(error));
            }

            Some(Ok(data::prim(Data::Some, arguments.unwrap())))
        } else {
            Some(Err(TzError::InvalidType))
        }
    }

    fn prepack_or(
        &self,
        args: Option<&Vec<MichelsonV1Expression>>,
    ) -> Result<MichelsonV1Expression, TzError> {
        use prim::Prim;
        use primitive::Data;

        if let MichelsonV1Expression::Prim(value) = self {
            let or_types = args.ok_or(TzError::InvalidType)?;

            if (value.prim != Primitive::Data(Data::Left)
                && value.prim != Primitive::Data(Data::Right))
                || value.args_count() != 1
                || or_types.len() != 2
            {
                return Err(TzError::InvalidType);
            }

            let index: usize = if value.prim == Primitive::Data(Data::Left) {
                0
            } else {
                1
            };
            let argument = value
                .args
                .as_ref()
                .unwrap()
                .first()
                .unwrap()
                .prepack(&or_types[index])?;

            Ok(MichelsonV1Expression::Prim(Prim::new(
                value.prim,
                Some(vec![argument]),
                None,
            )))
        } else {
            Err(TzError::InvalidType)
        }
    }

    // pub fn from_packed(packed: &str, schema: Option<MichelsonV1Expression>) -> Result<Self, TzError> {
    //     let mut encoded = ConsumableHexStr::new(packed);
    //     let prefix = encoded.consume_bytes(1)?;
    //     if prefix != PACK_PREFIX {
    //         return Err(TzError::InvalidType);
    //     }
    //     let result = MichelsonV1Expression::from_hex(&mut encoded)?;

    //     if let Some(schema) = schema {
    //         Self::postunpack(result, schema)
    //     } else {
    //         Ok(result)
    //     }
    // }

    // fn postunpack(value: MichelsonV1Expression, schema: MichelsonV1Expression) -> Result<MichelsonV1Expression, TzError> {
    //     todo!()
    // }

    fn type_info<'a>(
        &'a self,
    ) -> Result<
        (
            primitive::Type,
            Option<&'a Vec<MichelsonV1Expression>>,
            Option<&'a Vec<String>>,
        ),
        TzError,
    > {
        match self {
            MichelsonV1Expression::Prim(value) => value.type_info(),
            _ => Err(TzError::InvalidType),
        }
    }
}

impl Display for MichelsonV1Expression {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            MichelsonV1Expression::Prim(value) => write!(f, "{}", value),
            MichelsonV1Expression::Literal(value) => write!(f, "{}", value),
            MichelsonV1Expression::Sequence(values) => {
                let mut current: String = "{ ".into();
                for (index, value) in values.iter().enumerate() {
                    current = format!(
                        "{}{}{}",
                        current,
                        value,
                        if index != values.len() - 1 { "; " } else { "" }
                    )
                }
                current = format!("{}}}", current);
                write!(f, "{}", current)
            }
        }
    }
}

impl HexEncodable for MichelsonV1Expression {
    fn to_hex_encoded(&self) -> Result<String, TzError> {
        match self {
            MichelsonV1Expression::Prim(value) => value.to_hex_encoded(),
            MichelsonV1Expression::Literal(value) => value.to_hex_encoded(),
            MichelsonV1Expression::Sequence(value) => value.to_hex_encoded(),
        }
    }
}

impl HexDecodable for MichelsonV1Expression {
    fn from_hex(encoded: &mut ConsumableHexStr) -> Result<Self, TzError>
    where
        Self: Sized,
    {
        let prefix: MessagePrefix = encoded.read_bytes(1)?.try_into()?;

        Ok(match prefix {
            MessagePrefix::Prim(_) => MichelsonV1Expression::Prim(prim::Prim::from_hex(encoded)?),
            MessagePrefix::Literal(_) => {
                MichelsonV1Expression::Literal(literal::Literal::from_hex(encoded)?)
            }
            MessagePrefix::Sequence => {
                MichelsonV1Expression::Sequence(Vec::<MichelsonV1Expression>::from_hex(encoded)?)
            }
        })
    }
}

impl HexEncodable for Vec<MichelsonV1Expression> {
    fn to_hex_encoded(&self) -> Result<String, TzError> {
        let initial: Result<String, TzError> = Ok("".into());
        let encoded = self.iter().fold(initial, |current, next| {
            let encoded_item = next.to_hex_encoded()?;
            let result = format!("{}{}", current?, encoded_item);

            Ok(result)
        })?;

        let length = encoded.len() / 2;
        let result = format!(
            "{}{}{}",
            MessagePrefix::Sequence.prefix(),
            utils::num_to_padded_str(length, None, None),
            encoded
        );

        Ok(result)
    }
}

impl HexDecodable for Vec<MichelsonV1Expression> {
    fn from_hex(encoded: &mut ConsumableHexStr) -> Result<Self, TzError>
    where
        Self: Sized,
    {
        encoded.consume_bytes(1)?; // consume prefix
        let value = encoded.consume_lengh_and_value(None)?;
        let mut consumable = ConsumableHexStr::new(value);
        let mut sequence = Vec::<MichelsonV1Expression>::new();
        while consumable.has_more() {
            sequence.push(MichelsonV1Expression::from_hex(&mut consumable)?)
        }

        Ok(sequence)
    }
}

enum MessagePrefix {
    Prim(prim::MessagePrefix),
    Literal(literal::MessagePrefix),
    Sequence,
}

impl MessagePrefix {
    fn prefix(&self) -> &str {
        match self {
            MessagePrefix::Prim(value) => value.prefix(),
            MessagePrefix::Literal(value) => value.prefix(),
            MessagePrefix::Sequence => "02",
        }
    }
}

impl TryFrom<&str> for MessagePrefix {
    type Error = TzError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        prim::MessagePrefix::try_from(value)
            .map(|mp| MessagePrefix::Prim(mp))
            .or(literal::MessagePrefix::from(value)
                .map(|mp| MessagePrefix::Literal(mp))
                .or(match value {
                    "02" => Ok(MessagePrefix::Sequence),
                    _ => Err(TzError::InvalidType),
                }))
    }
}

pub fn extract_prim(value: &MichelsonV1Expression) -> Result<&prim::Prim, TzError> {
    if let MichelsonV1Expression::Prim(value) = value {
        Ok(value)
    } else {
        Err(TzError::InvalidType)
    }
}

pub fn extract_int(value: &MichelsonV1Expression) -> Result<&BigInt, TzError> {
    if let MichelsonV1Expression::Literal(literal::Literal::Int(value)) = value {
        Ok(value)
    } else {
        Err(TzError::InvalidType)
    }
}

pub fn extract_string(value: &MichelsonV1Expression) -> Result<&String, TzError> {
    if let MichelsonV1Expression::Literal(literal::Literal::String(value)) = value {
        Ok(value)
    } else {
        Err(TzError::InvalidType)
    }
}

pub fn extract_bytes(value: &MichelsonV1Expression) -> Result<&Vec<u8>, TzError> {
    if let MichelsonV1Expression::Literal(literal::Literal::Bytes(value)) = value {
        Ok(value)
    } else {
        Err(TzError::InvalidType)
    }
}

pub fn extract_key(value: &MichelsonV1Expression) -> Result<&MichelsonV1Expression, TzError> {
    let elt = extract_prim(value)?;
    match elt.prim {
        Primitive::Data(primitive::Data::Elt) => {
            if elt.args_count() == 2 {
                Ok(elt.args.as_ref().unwrap().first().as_ref().unwrap())
            } else {
                Err(TzError::InvalidType)
            }
        }
        _ => Err(TzError::InvalidType),
    }
}

pub fn extract_sequence(
    value: &MichelsonV1Expression,
) -> Result<&Vec<MichelsonV1Expression>, TzError> {
    if let MichelsonV1Expression::Sequence(value) = value {
        Ok(value)
    } else {
        Err(TzError::InvalidType)
    }
}

trait HexEncodable {
    fn to_hex_encoded(&self) -> Result<String, TzError>;
}

trait HexDecodable {
    fn from_hex(encoded: &mut ConsumableHexStr) -> Result<Self, TzError>
    where
        Self: Sized;
}

#[derive(Debug, Clone)]
pub struct ArgsBinding<'a> {
    pub label: Option<&'a str>,
    pub value: &'a Args<'a>,
}

#[derive(Debug, Clone)]
pub enum Args<'a> {
    Expr(MichelsonV1Expression),
    Array(&'a Vec<ArgsBinding<'a>>),
}

impl Args<'_> {
    fn is_string(&self) -> bool {
        match self {
            Args::Expr(value) => match value {
                MichelsonV1Expression::Literal(literal) => match literal {
                    literal::Literal::String(_) => true,
                    _ => false,
                },
                _ => false,
            },
            Args::Array(_) => false,
        }
    }

    fn is_int(&self) -> bool {
        match self {
            Args::Expr(value) => match value {
                MichelsonV1Expression::Literal(literal) => match literal {
                    literal::Literal::Int(_) => true,
                    _ => false,
                },
                _ => false,
            },
            Args::Array(_) => false,
        }
    }

    fn is_bytes(&self) -> bool {
        match self {
            Args::Expr(value) => match value {
                MichelsonV1Expression::Literal(literal) => match literal {
                    literal::Literal::Bytes(_) => true,
                    _ => false,
                },
                _ => false,
            },
            Args::Array(_) => false,
        }
    }

    fn is_bool(&self) -> bool {
        match self {
            Args::Expr(value) => match value {
                MichelsonV1Expression::Prim(prim) => match prim.prim {
                    Primitive::Data(data) => match data {
                        primitive::Data::False | primitive::Data::True => true,
                        _ => false,
                    },
                    _ => false,
                },
                _ => false,
            },
            Args::Array(_) => false,
        }
    }
}

#[cfg(test)]
mod test {
    use super::prim::Prim;
    use super::primitive::Data;
    use super::primitive::Primitive;
    use super::*;

    #[test]
    fn test_micheline_coding_1() -> Result<(), TzError> {
        let micheline = data::pair(int(1), string("test".into()));

        let encoded = micheline.to_hex_encoded()?;
        let encoded_str = "07070001010000000474657374";
        assert_eq!(encoded, encoded_str);

        let mut consumable_str = ConsumableHexStr::new(encoded_str);
        let mut decoded_micheline = MichelsonV1Expression::from_hex(&mut consumable_str)?;
        assert_eq!(decoded_micheline, micheline);

        let json = serde_json::json!(micheline);
        assert_eq!(
            json.to_string(),
            r#"{"prim":"Pair","args":[{"int":"1"},{"string":"test"}]}"#
        );

        decoded_micheline = serde_json::from_value(json)?;
        assert_eq!(decoded_micheline, micheline);

        Ok(())
    }

    #[test]
    fn test_micheline_coding_2() -> Result<(), TzError> {
        let micheline = data::pair(
            int(999999),
            data::pair(string("test".to_owned()), int(-299)),
        );

        let encoded = micheline.to_hex_encoded()?;
        let encoded_str = "070700bf887a070701000000047465737400eb04";
        assert_eq!(encoded, encoded_str);

        let mut consumable_str = ConsumableHexStr::new(encoded_str);
        let mut decoded_micheline = MichelsonV1Expression::from_hex(&mut consumable_str)?;
        assert_eq!(decoded_micheline, micheline);

        let json = serde_json::json!(micheline);
        assert_eq!(
            json.to_string(),
            r#"{"prim":"Pair","args":[{"int":"999999"},{"prim":"Pair","args":[{"string":"test"},{"int":"-299"}]}]}"#
        );

        decoded_micheline = serde_json::from_value(json)?;
        assert_eq!(decoded_micheline, micheline);

        Ok(())
    }

    #[test]
    fn test_micheline_coding_3() -> Result<(), TzError> {
        let mut micheline = data::some(string(":)".to_owned()));

        let mut encoded = micheline.to_hex_encoded()?;
        let mut encoded_str = "050901000000023a29";
        assert_eq!(encoded, encoded_str);

        let mut consumable_str = ConsumableHexStr::new(encoded_str);
        let mut decoded_micheline = MichelsonV1Expression::from_hex(&mut consumable_str)?;
        assert_eq!(decoded_micheline, micheline);

        let mut json = serde_json::json!(micheline);
        assert_eq!(
            json.to_string(),
            r#"{"prim":"Some","args":[{"string":":)"}]}"#
        );

        decoded_micheline = serde_json::from_value(json)?;
        assert_eq!(decoded_micheline, micheline);

        micheline = data::none();

        encoded = micheline.to_hex_encoded()?;
        encoded_str = "0306";
        assert_eq!(encoded, encoded_str);

        consumable_str = ConsumableHexStr::new(encoded_str);
        decoded_micheline = MichelsonV1Expression::from_hex(&mut consumable_str)?;
        assert_eq!(decoded_micheline, micheline);

        json = serde_json::json!(micheline);
        assert_eq!(json.to_string(), r#"{"prim":"None"}"#);

        decoded_micheline = serde_json::from_value(json)?;
        assert_eq!(decoded_micheline, micheline);

        Ok(())
    }

    #[test]
    fn test_micheline_coding_4() -> Result<(), TzError> {
        let mut micheline =
            MichelsonV1Expression::Prim(Prim::new(Primitive::Data(Data::True), None, None));

        let mut encoded = micheline.to_hex_encoded()?;
        let mut encoded_str = "030a";
        assert_eq!(encoded, encoded_str);

        let mut consumable_str = ConsumableHexStr::new(encoded_str);
        let mut decoded_micheline = MichelsonV1Expression::from_hex(&mut consumable_str)?;
        assert_eq!(decoded_micheline, micheline);

        let mut json = serde_json::json!(micheline);
        assert_eq!(json.to_string(), r#"{"prim":"True"}"#);

        decoded_micheline = serde_json::from_value(json)?;
        assert_eq!(decoded_micheline, micheline);

        micheline =
            MichelsonV1Expression::Prim(Prim::new(Primitive::Data(Data::False), None, None));

        encoded = micheline.to_hex_encoded()?;
        encoded_str = "0303";
        assert_eq!(encoded, encoded_str);

        consumable_str = ConsumableHexStr::new(encoded_str);
        decoded_micheline = MichelsonV1Expression::from_hex(&mut consumable_str)?;
        assert_eq!(decoded_micheline, micheline);

        json = serde_json::json!(micheline);
        assert_eq!(json.to_string(), r#"{"prim":"False"}"#);

        decoded_micheline = serde_json::from_value(json)?;
        assert_eq!(decoded_micheline, micheline);

        Ok(())
    }

    #[test]
    fn test_micheline_coding_5() -> Result<(), TzError> {
        let mut micheline = data::left(string("test".to_owned()));

        let mut encoded = micheline.to_hex_encoded()?;
        let mut encoded_str = "0505010000000474657374";
        assert_eq!(encoded, encoded_str);

        let mut consumable_str = ConsumableHexStr::new(encoded_str);
        let mut decoded_micheline = MichelsonV1Expression::from_hex(&mut consumable_str)?;
        assert_eq!(decoded_micheline, micheline);

        let mut json = serde_json::json!(micheline);
        assert_eq!(
            json.to_string(),
            r#"{"prim":"Left","args":[{"string":"test"}]}"#
        );

        decoded_micheline = serde_json::from_value(json)?;
        assert_eq!(decoded_micheline, micheline);

        micheline = data::right(int(1024));

        encoded = micheline.to_hex_encoded()?;
        encoded_str = "0508008010";
        assert_eq!(encoded, encoded_str);

        consumable_str = ConsumableHexStr::new(encoded_str);
        decoded_micheline = MichelsonV1Expression::from_hex(&mut consumable_str)?;
        assert_eq!(decoded_micheline, micheline);

        json = serde_json::json!(micheline);
        assert_eq!(
            json.to_string(),
            r#"{"prim":"Right","args":[{"int":"1024"}]}"#
        );

        decoded_micheline = serde_json::from_value(json)?;
        assert_eq!(decoded_micheline, micheline);

        Ok(())
    }

    #[test]
    fn test_micheline_coding_6() -> Result<(), TzError> {
        let micheline =
            MichelsonV1Expression::Prim(Prim::new(Primitive::Data(Data::Unit), None, None));

        let encoded = micheline.to_hex_encoded()?;
        let encoded_str = "030b";
        assert_eq!(encoded, encoded_str);

        let mut consumable_str = ConsumableHexStr::new(encoded_str);
        let mut decoded_micheline = MichelsonV1Expression::from_hex(&mut consumable_str)?;
        assert_eq!(decoded_micheline, micheline);

        let json = serde_json::json!(micheline);
        assert_eq!(json.to_string(), r#"{"prim":"Unit"}"#);

        decoded_micheline = serde_json::from_value(json)?;
        assert_eq!(decoded_micheline, micheline);

        Ok(())
    }

    #[test]
    fn test_micheline_coding_7() -> Result<(), TzError> {
        let micheline = sequence(vec![
            string("test1".to_owned()),
            string("test2".to_owned()),
            string("test3".to_owned()),
        ]);

        let encoded = micheline.to_hex_encoded()?;
        let encoded_str = "020000001e010000000574657374310100000005746573743201000000057465737433";
        assert_eq!(encoded, encoded_str);

        let mut consumable_str = ConsumableHexStr::new(encoded_str);
        let mut decoded_micheline = MichelsonV1Expression::from_hex(&mut consumable_str)?;
        assert_eq!(decoded_micheline, micheline);

        let json = serde_json::json!(micheline);
        assert_eq!(
            json.to_string(),
            r#"[{"string":"test1"},{"string":"test2"},{"string":"test3"}]"#
        );

        decoded_micheline = serde_json::from_value(json)?;
        assert_eq!(decoded_micheline, micheline);

        Ok(())
    }

    #[test]
    fn test_micheline_coding_8() -> Result<(), TzError> {
        let micheline = bytes(hex::decode("0a039f").expect("valid bytes"));

        let encoded = micheline.to_hex_encoded()?;
        let encoded_str = "0a000000030a039f";
        assert_eq!(encoded, encoded_str);

        let mut consumable_str = ConsumableHexStr::new(encoded_str);
        let mut decoded_micheline = MichelsonV1Expression::from_hex(&mut consumable_str)?;
        assert_eq!(decoded_micheline, micheline);

        let json = serde_json::json!(micheline);
        assert_eq!(json.to_string(), r#"{"bytes":"0a039f"}"#);

        decoded_micheline = serde_json::from_value(json)?;
        assert_eq!(decoded_micheline, micheline);

        Ok(())
    }

    #[test]
    fn test_micheline_pack_1() -> Result<(), TzError> {
        let micheline = data::some(sequence(vec![
            string("test1".into()),
            string("test2".into()),
        ]));
        let schema = types::option(types::list(types::string()));
        let packed = micheline.pack(Some(&schema))?;
        assert_eq!(
            packed,
            "05050902000000140100000005746573743101000000057465737432"
        );

        Ok(())
    }

    #[test]
    fn test_micheline_pack_2() -> Result<(), TzError> {
        let micheline = data::some(sequence(vec![
            data::elt(string("testKey1".into()), int(100)),
            data::elt(string("testKey2".into()), int(200)),
        ]));
        let schema = types::option(types::map(types::string(), types::int()));
        let packed = micheline.pack(Some(&schema))?;
        assert_eq!(packed, "050509020000002407040100000008746573744b65793100a40107040100000008746573744b657932008803");

        Ok(())
    }

    #[test]
    fn test_micheline_pack_3() -> Result<(), TzError> {
        let micheline = data::pair(
            string("tz1Ts3m2dXTXB66XN7cg5ALiAvzZY6AxrFd9".into()),
            int(100),
        );
        let schema = types::pair(types::address(), types::int());
        let packed = micheline.pack(Some(&schema))?;
        assert_eq!(
            packed,
            "0507070a0000001600005a374e077b2e539f222af1e61964d7487c8b95fe00a401"
        );

        Ok(())
    }

    #[test]
    fn test_micheline_pack_4() -> Result<(), TzError> {
        let micheline = data::some(string("tz1Ts3m2dXTXB66XN7cg5ALiAvzZY6AxrFd9".into()));
        let schema = types::option(types::address());
        let packed = micheline.pack(Some(&schema))?;
        assert_eq!(
            packed,
            "0505090a0000001600005a374e077b2e539f222af1e61964d7487c8b95fe"
        );

        Ok(())
    }

    #[test]
    fn test_micheline_pack_5() -> Result<(), TzError> {
        let mut micheline = data::left(data::left(string("test".into())));
        let schema = types::or(types::or(types::string(), types::int()), types::int());

        let mut packed = micheline.pack(Some(&schema))?;
        assert_eq!(packed, "0505050505010000000474657374");

        micheline = data::left(data::right(int(100)));

        packed = micheline.pack(Some(&schema))?;
        assert_eq!(packed, "050505050800a401");

        micheline = data::right(int(100));

        packed = micheline.pack(Some(&schema))?;
        assert_eq!(packed, "05050800a401");

        Ok(())
    }

    #[test]
    fn test_micheline_pack_6() -> Result<(), TzError> {
        let micheline = string("NetXdQprcVkpaWU".into());
        let schema = types::chain_id();

        let packed = micheline.pack(Some(&schema))?;
        assert_eq!(packed, "050a000000047a06a770");

        Ok(())
    }

    #[test]
    fn test_micheline_pack_7() -> Result<(), TzError> {
        let micheline = string("sigNw8i6ihAGn8iwcbgfdA5HNdmBRFVRBGoUPnvmPidnHyqD2HoLq6ZbAxiov9i7FrFgjvuU2Mu6NfxEg9onxQH8PSPsXpPT".into());
        let schema = types::signature();

        let packed = micheline.pack(Some(&schema))?;
        assert_eq!(packed, "050a00000040073a1c8aff3edfb9b5d4dcc02f4ecea06617a267d67d9ae9293d23676b3e17ea0b6d643e4b85c3f0d6e2d47f670f4ab4e826753a799494123d75d56a29d0c105");

        Ok(())
    }

    #[test]
    fn test_micheline_pack_8() -> Result<(), TzError> {
        let micheline = string("tz1Ts3m2dXTXB66XN7cg5ALiAvzZY6AxrFd9".into());
        let schema = types::key_hash();

        let packed = micheline.pack(Some(&schema))?;
        assert_eq!(
            packed,
            "050a00000015005a374e077b2e539f222af1e61964d7487c8b95fe"
        );

        Ok(())
    }

    #[test]
    fn test_micheline_pack_9() -> Result<(), TzError> {
        let micheline = string("edpkuAJhbFLfJ4zWbQQWTZNGDg7hrcG1m1CBSWVB3iDHChjuzeaZB6".into());
        let schema = types::key();

        let packed = micheline.pack(Some(&schema))?;
        assert_eq!(
            packed,
            "050a0000002100444e1f4ab90c304a5ac003d367747aab63815f583ff2330ce159d12c1ecceba1"
        );

        Ok(())
    }

    #[test]
    fn test_micheline_pack_10() -> Result<(), TzError> {
        let micheline = string("KT1JKNrzC57FtUe3dmYXmm12ucmjDmzbkKrc%transfer".into());
        let schema = types::contract();

        let packed = micheline.pack(Some(&schema))?;
        assert_eq!(
            packed,
            "050a0000001e016ac8111c23353817d663fe21ff7037f9de36a8c4007472616e73666572"
        );

        Ok(())
    }

    #[test]
    fn test_micheline_pack_11() -> Result<(), TzError> {
        let micheline = string("2020-11-10T07:49:28Z".into());
        let schema = types::timestamp();

        let packed = micheline.pack(Some(&schema))?;
        assert_eq!(packed, "05008898d2fa0b");

        Ok(())
    }

    fn test_micheline_pack_12() -> Result<(), TzError> {
        let call = sequence(vec![
            instructions::drop(),
            instructions::nil(types::operation()),
            instructions::push(
                types::address(),
                string("KT1S4QXjASmM3ei7a6raRLkr4TbnKu7RuC9X%mint".into()),
            ),
            instructions::contract(types::list(types::pair(
                types::address(),
                types::pair(types::nat(), types::nat()),
            ))),
            sequence(vec![instructions::if_none(
                sequence(vec![instructions::unit(), instructions::fail_with()]),
                sequence(vec![]),
            )]),
            instructions::push(types::mutez(), int(0)),
            instructions::nil(types::pair(
                types::address(),
                types::pair(types::nat(), types::nat()),
            )),
            instructions::push(types::nat(), int(1)),
            instructions::push(types::nat(), int(0)),
            instructions::pair(),
            instructions::push(
                types::address(),
                string("tz1Mj7RzPmMAqDUNFBn5t5VbXmWW4cSUAdtT".into()),
            ),
            instructions::pair(),
            instructions::cons(),
            instructions::transfer_tokens(),
            instructions::cons(),
        ]);

        let micheline = data::pair(string("NetXm8tYqnMWky1".into()), data::pair(int(2), call));
        let schema = types::pair(
            types::chain_id(),
            types::pair(
                types::nat(),
                types::lambda(types::unit(), types::list(types::operation())),
            ),
        );

        let packed = micheline.pack(Some(&schema))?;
        assert_eq!(packed, "0507070a00000004a83650210707000202000000930320053d036d0743036e0a0000001a01bfb439dbd450df69f25257e901c568377527cb11006d696e740555055f0765036e0765036203620200000010072f0200000004034f032702000000000743036a0000053d0765036e07650362036207430362000107430362000003420743036e0a00000016000016e64994c2ddbd293695b63e4cade029d3c8b5e30342031b034d031b");

        Ok(())
    }

    #[test]
    fn test_display_michelson() -> Result<(), TzError> {
        let call = sequence(vec![
            instructions::drop(),
            instructions::nil(types::operation()),
            instructions::push(
                types::address(),
                string("KT1S4QXjASmM3ei7a6raRLkr4TbnKu7RuC9X%mint".into()),
            ),
            instructions::contract(types::list(types::pair(
                types::address(),
                types::pair(types::nat(), types::nat()),
            ))),
            sequence(vec![instructions::if_none(
                sequence(vec![instructions::unit(), instructions::fail_with()]),
                sequence(vec![]),
            )]),
            instructions::push(types::mutez(), int(0)),
            instructions::nil(types::pair(
                types::address(),
                types::pair(types::nat(), types::nat()),
            )),
            instructions::push(types::nat(), int(1)),
            instructions::push(types::nat(), int(0)),
            instructions::pair(),
            instructions::push(
                types::address(),
                string("tz1Mj7RzPmMAqDUNFBn5t5VbXmWW4cSUAdtT".into()),
            ),
            instructions::pair(),
            instructions::cons(),
            instructions::transfer_tokens(),
            instructions::cons(),
        ]);

        let micheline = data::pair(string("NetXm8tYqnMWky1".into()), data::pair(int(2), call));
        let schema = types::pair(
            types::chain_id(),
            types::pair(
                types::nat(),
                types::lambda(types::unit(), types::list(types::operation())),
            ),
        );

        assert_eq!(format!("{}", micheline), "(Pair \"NetXm8tYqnMWky1\" (Pair 2 { DROP; NIL operation; PUSH address \"KT1S4QXjASmM3ei7a6raRLkr4TbnKu7RuC9X%mint\"; CONTRACT (list (pair address (pair nat nat))); { IF_NONE { UNIT; FAILWITH} { }}; PUSH mutez 0; NIL (pair address (pair nat nat)); PUSH nat 1; PUSH nat 0; PAIR; PUSH address \"tz1Mj7RzPmMAqDUNFBn5t5VbXmWW4cSUAdtT\"; PAIR; CONS; TRANSFER_TOKENS; CONS}))");
        assert_eq!(
            format!("{}", schema),
            "(pair chain_id (pair nat (lambda unit (list operation))))"
        );

        Ok(())
    }
}
