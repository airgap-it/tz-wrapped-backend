use derive_more::{Display, Error};
use serde::{Deserialize, Serialize};

use super::coding;
use super::utils;
use super::utils::ConsumableHexStr;

pub mod literal;
pub mod prim;
pub mod primitive;

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
            let packed = self.prepack(schema)?;
            encoded = packed.to_hex_encoded()?;
        } else {
            encoded = self.to_hex_encoded()?;
        }

        Ok(format!("{}{}", PACK_PREFIX, encoded))
    }

    fn prepack(&self, schema: &MichelsonV1Expression) -> Result<MichelsonV1Expression, TzError> {
        use primitive::Type;

        let (type_, args) = schema.type_info()?;
        let string_value =
            if let MichelsonV1Expression::Literal(literal::Literal::String(value)) = self {
                Some(value)
            } else {
                None
            };
        Ok(match type_ {
            Type::List | Type::Set => self.prepack_sequence(args)?,
            Type::Map | Type::BigMap => self.prepack_map(args)?,
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
                    MichelsonV1Expression::Literal(literal::Literal::Bytes(
                        coding::encode_chain_id(value)?,
                    ))
                } else {
                    self.clone()
                }
            }
            Type::Signature => {
                if let Some(value) = string_value {
                    MichelsonV1Expression::Literal(literal::Literal::Bytes(
                        coding::encode_signature(value)?,
                    ))
                } else {
                    self.clone()
                }
            }
            Type::KeyHash => {
                if let Some(value) = string_value {
                    MichelsonV1Expression::Literal(literal::Literal::Bytes(coding::encode_address(
                        value, true,
                    )?))
                } else {
                    self.clone()
                }
            }
            Type::Key => {
                if let Some(value) = string_value {
                    MichelsonV1Expression::Literal(literal::Literal::Bytes(
                        coding::encode_public_key(value)?,
                    ))
                } else {
                    self.clone()
                }
            }
            Type::Address | Type::Contract => {
                if let Some(value) = string_value {
                    MichelsonV1Expression::Literal(literal::Literal::Bytes(
                        coding::encode_contract(value)?,
                    ))
                } else {
                    self.clone()
                }
            }
            Type::Timestamp => {
                if let Some(value) = string_value {
                    MichelsonV1Expression::Literal(literal::Literal::Int(coding::encode_timestamp(
                        value,
                    )?))
                } else {
                    self.clone()
                }
            }
            _ => self.clone(),
        })
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
        use prim::Prim;
        use primitive::{Data, Primitive};

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

                        Ok(MichelsonV1Expression::Prim(Prim::new(
                            Primitive::Data(Data::Elt),
                            arguments,
                            None,
                        )))
                    } else {
                        Err(TzError::InvalidType)
                    }
                })
                .collect();

            Ok(MichelsonV1Expression::Sequence(prepacked?))
        } else {
            Err(TzError::InvalidType)
        }
    }

    fn prepack_pair(
        &self,
        args: Option<&Vec<MichelsonV1Expression>>,
    ) -> Result<MichelsonV1Expression, TzError> {
        use prim::Prim;
        use primitive::{Data, Primitive};

        if let MichelsonV1Expression::Prim(value) = self {
            let pair_types = args.ok_or(TzError::InvalidType)?;

            if value.prim != Primitive::Data(Data::Pair) || value.args_count() != pair_types.len() {
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

            Ok(MichelsonV1Expression::Prim(Prim::new(
                Primitive::Data(Data::Pair),
                arguments,
                None,
            )))
        } else {
            Err(TzError::InvalidType)
        }
    }

    fn prepack_option(
        &self,
        args: Option<&Vec<MichelsonV1Expression>>,
    ) -> Option<Result<MichelsonV1Expression, TzError>> {
        use prim::Prim;
        use primitive::{Data, Primitive};

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

            Some(Ok(MichelsonV1Expression::Prim(Prim::new(
                Primitive::Data(Data::Some),
                arguments.unwrap(),
                None,
            ))))
        } else {
            Some(Err(TzError::InvalidType))
        }
    }

    fn prepack_or(
        &self,
        args: Option<&Vec<MichelsonV1Expression>>,
    ) -> Result<MichelsonV1Expression, TzError> {
        use prim::Prim;
        use primitive::{Data, Primitive};

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

    fn type_info(&self) -> Result<(primitive::Type, Option<&Vec<MichelsonV1Expression>>), TzError> {
        match self {
            MichelsonV1Expression::Prim(value) => value.type_info(),
            _ => Err(TzError::InvalidType),
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
        let prefix = MessagePrefix::from(encoded.read_bytes(1)?)?;

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
        let initial: Result<String, TzError> = Ok(String::from(""));
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
    fn from(value: &str) -> Result<Self, TzError> {
        prim::MessagePrefix::from(value)
            .map(|mp| MessagePrefix::Prim(mp))
            .or(literal::MessagePrefix::from(value)
                .map(|mp| MessagePrefix::Literal(mp))
                .or(match value {
                    "02" => Ok(MessagePrefix::Sequence),
                    _ => Err(TzError::InvalidType),
                }))
    }

    fn prefix(&self) -> &str {
        match self {
            MessagePrefix::Prim(value) => value.prefix(),
            MessagePrefix::Literal(value) => value.prefix(),
            MessagePrefix::Sequence => "02",
        }
    }
}

pub fn extract_prim(value: &MichelsonV1Expression) -> Result<&prim::Prim, TzError> {
    if let MichelsonV1Expression::Prim(value) = value {
        Ok(value)
    } else {
        Err(TzError::InvalidType)
    }
}

pub fn extract_int(value: &MichelsonV1Expression) -> Result<&i64, TzError> {
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

pub fn extract_sequence(
    value: &MichelsonV1Expression,
) -> Result<&Vec<MichelsonV1Expression>, TzError> {
    if let MichelsonV1Expression::Sequence(value) = value {
        Ok(value)
    } else {
        Err(TzError::InvalidType)
    }
}

#[derive(Error, Display, Debug)]
pub enum TzError {
    InvalidIndex,
    InvalidType,
    InvalidArgument,
    NetworkFailure,
    ParsingFailure,
}

trait HexEncodable {
    fn to_hex_encoded(&self) -> Result<String, TzError>;
}

trait HexDecodable {
    fn from_hex(encoded: &mut ConsumableHexStr) -> Result<Self, TzError>
    where
        Self: Sized;
}

#[cfg(test)]
mod test {
    use super::literal::Literal;
    use super::prim::Prim;
    use super::primitive::Data;
    use super::primitive::Primitive;
    use super::primitive::Type;
    use super::*;

    #[test]
    fn test_micheline_encoding_1() -> Result<(), TzError> {
        let micheline = MichelsonV1Expression::Prim(Prim::new(
            Primitive::Data(Data::Pair),
            Some(vec![
                MichelsonV1Expression::Literal(Literal::Int(1)),
                MichelsonV1Expression::Literal(Literal::String(String::from("test"))),
            ]),
            None,
        ));

        let encoded = micheline.to_hex_encoded()?;

        assert_eq!(encoded, "07070001010000000474657374");

        Ok(())
    }

    #[test]
    fn test_micheline_decoding_1() -> Result<(), TzError> {
        let micheline = MichelsonV1Expression::Prim(Prim::new(
            Primitive::Data(Data::Pair),
            Some(vec![
                MichelsonV1Expression::Literal(Literal::Int(1)),
                MichelsonV1Expression::Literal(Literal::String(String::from("test"))),
            ]),
            None,
        ));

        let mut encoded = ConsumableHexStr::new("07070001010000000474657374");
        let decoded_micheline = MichelsonV1Expression::from_hex(&mut encoded)?;

        assert_eq!(decoded_micheline, micheline);

        Ok(())
    }

    #[test]
    fn test_micheline_serialize_1() -> Result<(), TzError> {
        let micheline = MichelsonV1Expression::Prim(Prim::new(
            Primitive::Data(Data::Pair),
            Some(vec![
                MichelsonV1Expression::Literal(Literal::Int(1)),
                MichelsonV1Expression::Literal(Literal::String(String::from("test"))),
            ]),
            None,
        ));

        let json = serde_json::json!(micheline).to_string();

        assert_eq!(
            json,
            r#"{"prim":"Pair","args":[{"int":"1"},{"string":"test"}]}"#
        );

        Ok(())
    }

    #[test]
    fn test_micheline_deserialize_1() -> Result<(), serde_json::Error> {
        let micheline = MichelsonV1Expression::Prim(Prim::new(
            Primitive::Data(Data::Pair),
            Some(vec![
                MichelsonV1Expression::Literal(Literal::Int(1)),
                MichelsonV1Expression::Literal(Literal::String(String::from("test"))),
            ]),
            None,
        ));

        let json = serde_json::json!({"prim":"Pair","args":[{"int":"1"},{"string":"test"}]});
        let decoded_micheline: MichelsonV1Expression = serde_json::from_value(json)?;

        assert_eq!(decoded_micheline, micheline);

        Ok(())
    }

    #[test]
    fn test_micheline_encoding_2() -> Result<(), TzError> {
        let micheline = MichelsonV1Expression::Prim(Prim::new(
            Primitive::Data(Data::Pair),
            Some(vec![
                MichelsonV1Expression::Literal(Literal::Int(999999)),
                MichelsonV1Expression::Prim(Prim::new(
                    Primitive::Data(Data::Pair),
                    Some(vec![
                        MichelsonV1Expression::Literal(Literal::String(String::from("test"))),
                        MichelsonV1Expression::Literal(Literal::Int(-299)),
                    ]),
                    None,
                )),
            ]),
            None,
        ));

        let encoded = micheline.to_hex_encoded()?;

        assert_eq!(encoded, "070700bf887a070701000000047465737400eb04");

        Ok(())
    }

    #[test]
    fn test_micheline_decoding_2() -> Result<(), TzError> {
        let micheline = MichelsonV1Expression::Prim(Prim::new(
            Primitive::Data(Data::Pair),
            Some(vec![
                MichelsonV1Expression::Literal(Literal::Int(999999)),
                MichelsonV1Expression::Prim(Prim::new(
                    Primitive::Data(Data::Pair),
                    Some(vec![
                        MichelsonV1Expression::Literal(Literal::String(String::from("test"))),
                        MichelsonV1Expression::Literal(Literal::Int(-299)),
                    ]),
                    None,
                )),
            ]),
            None,
        ));

        let mut encoded = ConsumableHexStr::new("070700bf887a070701000000047465737400eb04");
        let decoded_micheline = MichelsonV1Expression::from_hex(&mut encoded)?;

        assert_eq!(decoded_micheline, micheline);

        Ok(())
    }

    #[test]
    fn test_micheline_serialize_2() -> Result<(), TzError> {
        let micheline = MichelsonV1Expression::Prim(Prim::new(
            Primitive::Data(Data::Pair),
            Some(vec![
                MichelsonV1Expression::Literal(Literal::Int(999999)),
                MichelsonV1Expression::Prim(Prim::new(
                    Primitive::Data(Data::Pair),
                    Some(vec![
                        MichelsonV1Expression::Literal(Literal::String(String::from("test"))),
                        MichelsonV1Expression::Literal(Literal::Int(-299)),
                    ]),
                    None,
                )),
            ]),
            None,
        ));

        let json = serde_json::json!(micheline).to_string();

        assert_eq!(
            json,
            r#"{"prim":"Pair","args":[{"int":"999999"},{"prim":"Pair","args":[{"string":"test"},{"int":"-299"}]}]}"#
        );

        Ok(())
    }

    #[test]
    fn test_micheline_deserialize_2() -> Result<(), serde_json::Error> {
        let micheline = MichelsonV1Expression::Prim(Prim::new(
            Primitive::Data(Data::Pair),
            Some(vec![
                MichelsonV1Expression::Literal(Literal::Int(999999)),
                MichelsonV1Expression::Prim(Prim::new(
                    Primitive::Data(Data::Pair),
                    Some(vec![
                        MichelsonV1Expression::Literal(Literal::String(String::from("test"))),
                        MichelsonV1Expression::Literal(Literal::Int(-299)),
                    ]),
                    None,
                )),
            ]),
            None,
        ));

        let json = serde_json::json!({"prim":"Pair","args":[{"int":"999999"},{"prim":"Pair","args":[{"string":"test"},{"int":"-299"}]}]});
        let decoded_micheline: MichelsonV1Expression = serde_json::from_value(json)?;

        assert_eq!(decoded_micheline, micheline);

        Ok(())
    }

    #[test]
    fn test_micheline_encoding_3() -> Result<(), TzError> {
        let mut micheline = MichelsonV1Expression::Prim(Prim::new(
            Primitive::Data(Data::Some),
            Some(vec![MichelsonV1Expression::Literal(Literal::String(
                String::from(":)"),
            ))]),
            None,
        ));

        let mut encoded = micheline.to_hex_encoded()?;

        assert_eq!(encoded, "050901000000023a29");

        micheline = MichelsonV1Expression::Prim(Prim::new(Primitive::Data(Data::None), None, None));

        encoded = micheline.to_hex_encoded()?;

        assert_eq!(encoded, "0306");

        Ok(())
    }

    #[test]
    fn test_micheline_decoding_3() -> Result<(), TzError> {
        let mut micheline = MichelsonV1Expression::Prim(Prim::new(
            Primitive::Data(Data::Some),
            Some(vec![MichelsonV1Expression::Literal(Literal::String(
                String::from(":)"),
            ))]),
            None,
        ));

        let mut encoded = ConsumableHexStr::new("050901000000023a29");
        let mut decoded_micheline = MichelsonV1Expression::from_hex(&mut encoded)?;

        assert_eq!(decoded_micheline, micheline);

        micheline = MichelsonV1Expression::Prim(Prim::new(Primitive::Data(Data::None), None, None));

        encoded = ConsumableHexStr::new("0306");
        decoded_micheline = MichelsonV1Expression::from_hex(&mut encoded)?;

        assert_eq!(decoded_micheline, micheline);

        Ok(())
    }

    #[test]
    fn test_micheline_serialize_3() -> Result<(), TzError> {
        let mut micheline = MichelsonV1Expression::Prim(Prim::new(
            Primitive::Data(Data::Some),
            Some(vec![MichelsonV1Expression::Literal(Literal::String(
                String::from(":)"),
            ))]),
            None,
        ));

        let mut json = serde_json::json!(micheline).to_string();

        assert_eq!(json, r#"{"prim":"Some","args":[{"string":":)"}]}"#);

        micheline = MichelsonV1Expression::Prim(Prim::new(Primitive::Data(Data::None), None, None));

        json = serde_json::json!(micheline).to_string();

        assert_eq!(json, r#"{"prim":"None"}"#);

        Ok(())
    }

    #[test]
    fn test_micheline_deserialize_3() -> Result<(), serde_json::Error> {
        let mut micheline = MichelsonV1Expression::Prim(Prim::new(
            Primitive::Data(Data::Some),
            Some(vec![MichelsonV1Expression::Literal(Literal::String(
                String::from(":)"),
            ))]),
            None,
        ));

        let mut json = serde_json::json!({"prim":"Some","args":[{"string":":)"}]});
        let mut decoded_micheline: MichelsonV1Expression = serde_json::from_value(json)?;

        assert_eq!(decoded_micheline, micheline);

        micheline = MichelsonV1Expression::Prim(Prim::new(Primitive::Data(Data::None), None, None));

        json = serde_json::json!({"prim":"None"});
        decoded_micheline = serde_json::from_value(json)?;

        assert_eq!(decoded_micheline, micheline);

        Ok(())
    }

    #[test]
    fn test_micheline_encoding_4() -> Result<(), TzError> {
        let mut micheline =
            MichelsonV1Expression::Prim(Prim::new(Primitive::Data(Data::True), None, None));

        let mut encoded = micheline.to_hex_encoded()?;

        assert_eq!(encoded, "030a");

        micheline =
            MichelsonV1Expression::Prim(Prim::new(Primitive::Data(Data::False), None, None));

        encoded = micheline.to_hex_encoded()?;

        assert_eq!(encoded, "0303");

        Ok(())
    }

    #[test]
    fn test_micheline_decoding_4() -> Result<(), TzError> {
        let mut micheline =
            MichelsonV1Expression::Prim(Prim::new(Primitive::Data(Data::True), None, None));

        let mut encoded = ConsumableHexStr::new("030a");
        let mut decoded_micheline = MichelsonV1Expression::from_hex(&mut encoded)?;

        assert_eq!(decoded_micheline, micheline);

        micheline =
            MichelsonV1Expression::Prim(Prim::new(Primitive::Data(Data::False), None, None));

        encoded = ConsumableHexStr::new("0303");
        decoded_micheline = MichelsonV1Expression::from_hex(&mut encoded)?;

        assert_eq!(decoded_micheline, micheline);

        Ok(())
    }

    #[test]
    fn test_micheline_serialize_4() -> Result<(), TzError> {
        let mut micheline =
            MichelsonV1Expression::Prim(Prim::new(Primitive::Data(Data::True), None, None));

        let mut json = serde_json::json!(micheline).to_string();

        assert_eq!(json, r#"{"prim":"True"}"#);

        micheline =
            MichelsonV1Expression::Prim(Prim::new(Primitive::Data(Data::False), None, None));

        json = serde_json::json!(micheline).to_string();

        assert_eq!(json, r#"{"prim":"False"}"#);

        Ok(())
    }

    #[test]
    fn test_micheline_deserialize_4() -> Result<(), serde_json::Error> {
        let mut micheline =
            MichelsonV1Expression::Prim(Prim::new(Primitive::Data(Data::True), None, None));

        let mut json = serde_json::json!({"prim":"True"});
        let mut decoded_micheline: MichelsonV1Expression = serde_json::from_value(json)?;

        assert_eq!(decoded_micheline, micheline);

        micheline =
            MichelsonV1Expression::Prim(Prim::new(Primitive::Data(Data::False), None, None));

        json = serde_json::json!({"prim":"False"});
        decoded_micheline = serde_json::from_value(json)?;

        assert_eq!(decoded_micheline, micheline);

        Ok(())
    }

    #[test]
    fn test_micheline_encoding_5() -> Result<(), TzError> {
        let mut micheline = MichelsonV1Expression::Prim(Prim::new(
            Primitive::Data(Data::Left),
            Some(vec![MichelsonV1Expression::Literal(Literal::String(
                String::from("test"),
            ))]),
            None,
        ));

        let mut encoded = micheline.to_hex_encoded()?;

        assert_eq!(encoded, "0505010000000474657374");

        micheline = MichelsonV1Expression::Prim(Prim::new(
            Primitive::Data(Data::Right),
            Some(vec![MichelsonV1Expression::Literal(Literal::Int(1024))]),
            None,
        ));

        encoded = micheline.to_hex_encoded()?;

        assert_eq!(encoded, "0508008010");

        Ok(())
    }

    #[test]
    fn test_micheline_decoding_5() -> Result<(), TzError> {
        let mut micheline = MichelsonV1Expression::Prim(Prim::new(
            Primitive::Data(Data::Left),
            Some(vec![MichelsonV1Expression::Literal(Literal::String(
                String::from("test"),
            ))]),
            None,
        ));

        let mut encoded = ConsumableHexStr::new("0505010000000474657374");
        let mut decoded_micheline = MichelsonV1Expression::from_hex(&mut encoded)?;

        assert_eq!(decoded_micheline, micheline);

        micheline = MichelsonV1Expression::Prim(Prim::new(
            Primitive::Data(Data::Right),
            Some(vec![MichelsonV1Expression::Literal(Literal::Int(1024))]),
            None,
        ));

        encoded = ConsumableHexStr::new("0508008010");
        decoded_micheline = MichelsonV1Expression::from_hex(&mut encoded)?;

        assert_eq!(decoded_micheline, micheline);

        Ok(())
    }

    #[test]
    fn test_micheline_serialize_5() -> Result<(), TzError> {
        let mut micheline = MichelsonV1Expression::Prim(Prim::new(
            Primitive::Data(Data::Left),
            Some(vec![MichelsonV1Expression::Literal(Literal::String(
                String::from("test"),
            ))]),
            None,
        ));

        let mut json = serde_json::json!(micheline).to_string();

        assert_eq!(json, r#"{"prim":"Left","args":[{"string":"test"}]}"#);

        micheline = MichelsonV1Expression::Prim(Prim::new(
            Primitive::Data(Data::Right),
            Some(vec![MichelsonV1Expression::Literal(Literal::Int(1024))]),
            None,
        ));

        json = serde_json::json!(micheline).to_string();

        assert_eq!(json, r#"{"prim":"Right","args":[{"int":"1024"}]}"#);

        Ok(())
    }

    #[test]
    fn test_micheline_deserialize_5() -> Result<(), serde_json::Error> {
        let mut micheline = MichelsonV1Expression::Prim(Prim::new(
            Primitive::Data(Data::Left),
            Some(vec![MichelsonV1Expression::Literal(Literal::String(
                String::from("test"),
            ))]),
            None,
        ));

        let mut json = serde_json::json!({"prim":"Left","args":[{"string":"test"}]});
        let mut decoded_micheline: MichelsonV1Expression = serde_json::from_value(json)?;

        assert_eq!(decoded_micheline, micheline);

        micheline = MichelsonV1Expression::Prim(Prim::new(
            Primitive::Data(Data::Right),
            Some(vec![MichelsonV1Expression::Literal(Literal::Int(1024))]),
            None,
        ));

        json = serde_json::json!({"prim":"Right","args":[{"int":"1024"}]});
        decoded_micheline = serde_json::from_value(json)?;

        assert_eq!(decoded_micheline, micheline);

        Ok(())
    }

    #[test]
    fn test_micheline_encoding_6() -> Result<(), TzError> {
        let micheline =
            MichelsonV1Expression::Prim(Prim::new(Primitive::Data(Data::Unit), None, None));

        let encoded = micheline.to_hex_encoded()?;

        assert_eq!(encoded, "030b");

        Ok(())
    }

    #[test]
    fn test_micheline_decoding_6() -> Result<(), TzError> {
        let micheline =
            MichelsonV1Expression::Prim(Prim::new(Primitive::Data(Data::Unit), None, None));

        let mut encoded = ConsumableHexStr::new("030b");
        let decoded_micheline = MichelsonV1Expression::from_hex(&mut encoded)?;

        assert_eq!(decoded_micheline, micheline);

        Ok(())
    }

    #[test]
    fn test_micheline_serialize_6() -> Result<(), TzError> {
        let micheline =
            MichelsonV1Expression::Prim(Prim::new(Primitive::Data(Data::Unit), None, None));

        let json = serde_json::json!(micheline).to_string();

        assert_eq!(json, r#"{"prim":"Unit"}"#);

        Ok(())
    }

    #[test]
    fn test_micheline_deserialize_6() -> Result<(), serde_json::Error> {
        let micheline =
            MichelsonV1Expression::Prim(Prim::new(Primitive::Data(Data::Unit), None, None));

        let json = serde_json::json!({"prim":"Unit"});
        let decoded_micheline: MichelsonV1Expression = serde_json::from_value(json)?;

        assert_eq!(decoded_micheline, micheline);

        Ok(())
    }

    #[test]
    fn test_micheline_encoding_7() -> Result<(), TzError> {
        let micheline = MichelsonV1Expression::Sequence(vec![
            MichelsonV1Expression::Literal(Literal::String(String::from("test1"))),
            MichelsonV1Expression::Literal(Literal::String(String::from("test2"))),
            MichelsonV1Expression::Literal(Literal::String(String::from("test3"))),
        ]);

        let encoded = micheline.to_hex_encoded()?;

        assert_eq!(
            encoded,
            "020000001e010000000574657374310100000005746573743201000000057465737433"
        );

        Ok(())
    }

    #[test]
    fn test_micheline_decoding_7() -> Result<(), TzError> {
        let micheline = MichelsonV1Expression::Sequence(vec![
            MichelsonV1Expression::Literal(Literal::String(String::from("test1"))),
            MichelsonV1Expression::Literal(Literal::String(String::from("test2"))),
            MichelsonV1Expression::Literal(Literal::String(String::from("test3"))),
        ]);

        let mut encoded = ConsumableHexStr::new(
            "020000001e010000000574657374310100000005746573743201000000057465737433",
        );
        let decoded_micheline = MichelsonV1Expression::from_hex(&mut encoded)?;

        assert_eq!(decoded_micheline, micheline);

        Ok(())
    }

    #[test]
    fn test_micheline_serialize_7() -> Result<(), TzError> {
        let micheline = MichelsonV1Expression::Sequence(vec![
            MichelsonV1Expression::Literal(Literal::String(String::from("test1"))),
            MichelsonV1Expression::Literal(Literal::String(String::from("test2"))),
            MichelsonV1Expression::Literal(Literal::String(String::from("test3"))),
        ]);

        let json = serde_json::json!(micheline).to_string();

        assert_eq!(
            json,
            r#"[{"string":"test1"},{"string":"test2"},{"string":"test3"}]"#
        );

        Ok(())
    }

    #[test]
    fn test_micheline_deserialize_7() -> Result<(), serde_json::Error> {
        let micheline = MichelsonV1Expression::Sequence(vec![
            MichelsonV1Expression::Literal(Literal::String(String::from("test1"))),
            MichelsonV1Expression::Literal(Literal::String(String::from("test2"))),
            MichelsonV1Expression::Literal(Literal::String(String::from("test3"))),
        ]);

        let json = serde_json::json!([{"string":"test1"},{"string":"test2"},{"string":"test3"}]);
        let decoded_micheline: MichelsonV1Expression = serde_json::from_value(json)?;

        assert_eq!(decoded_micheline, micheline);

        Ok(())
    }

    #[test]
    fn test_micheline_encoding_8() -> Result<(), TzError> {
        let bytes = hex::decode("0a039f").expect("valid bytes");
        let micheline = MichelsonV1Expression::Literal(Literal::Bytes(bytes));

        let encoded = micheline.to_hex_encoded()?;

        assert_eq!(encoded, "0a000000030a039f");

        Ok(())
    }

    #[test]
    fn test_micheline_decoding_8() -> Result<(), TzError> {
        let bytes = hex::decode("0a039f").expect("valid bytes");
        let micheline = MichelsonV1Expression::Literal(Literal::Bytes(bytes));

        let mut encoded = ConsumableHexStr::new("0a000000030a039f");
        let decoded_micheline = MichelsonV1Expression::from_hex(&mut encoded)?;

        assert_eq!(decoded_micheline, micheline);

        Ok(())
    }

    #[test]
    fn test_micheline_serialize_8() -> Result<(), TzError> {
        let bytes = hex::decode("0a039f").expect("valid bytes");
        let micheline = MichelsonV1Expression::Literal(Literal::Bytes(bytes));

        let json = serde_json::json!(micheline).to_string();

        assert_eq!(json, r#"{"bytes":"0a039f"}"#);

        Ok(())
    }

    #[test]
    fn test_micheline_deserialize_8() -> Result<(), serde_json::Error> {
        let bytes = hex::decode("0a039f").expect("valid bytes");
        let micheline = MichelsonV1Expression::Literal(Literal::Bytes(bytes));

        let json = serde_json::json!({"bytes":"0a039f"});
        let decoded_micheline: MichelsonV1Expression = serde_json::from_value(json)?;

        assert_eq!(decoded_micheline, micheline);

        Ok(())
    }

    #[test]
    fn test_micheline_pack_1() -> Result<(), TzError> {
        let micheline = MichelsonV1Expression::Prim(Prim::new(
            Primitive::Data(Data::Some),
            Some(vec![MichelsonV1Expression::Sequence(vec![
                MichelsonV1Expression::Literal(Literal::String(String::from("test1"))),
                MichelsonV1Expression::Literal(Literal::String(String::from("test2"))),
            ])]),
            None,
        ));
        let schema = MichelsonV1Expression::Prim(Prim::new(
            Primitive::Type(Type::Option),
            Some(vec![MichelsonV1Expression::Prim(Prim::new(
                Primitive::Type(Type::List),
                Some(vec![MichelsonV1Expression::Prim(Prim::new(
                    Primitive::Type(Type::String),
                    None,
                    None,
                ))]),
                None,
            ))]),
            None,
        ));

        let packed = micheline.pack(Some(&schema))?;

        assert_eq!(
            packed,
            "05050902000000140100000005746573743101000000057465737432"
        );

        Ok(())
    }

    #[test]
    fn test_micheline_pack_2() -> Result<(), TzError> {
        let micheline = MichelsonV1Expression::Prim(Prim::new(
            Primitive::Data(Data::Some),
            Some(vec![MichelsonV1Expression::Sequence(vec![
                MichelsonV1Expression::Prim(Prim::new(
                    Primitive::Data(Data::Elt),
                    Some(vec![
                        MichelsonV1Expression::Literal(Literal::String(String::from("testKey1"))),
                        MichelsonV1Expression::Literal(Literal::Int(100)),
                    ]),
                    None,
                )),
                MichelsonV1Expression::Prim(Prim::new(
                    Primitive::Data(Data::Elt),
                    Some(vec![
                        MichelsonV1Expression::Literal(Literal::String(String::from("testKey2"))),
                        MichelsonV1Expression::Literal(Literal::Int(200)),
                    ]),
                    None,
                )),
            ])]),
            None,
        ));
        let schema = MichelsonV1Expression::Prim(Prim::new(
            Primitive::Type(Type::Option),
            Some(vec![MichelsonV1Expression::Prim(Prim::new(
                Primitive::Type(Type::Map),
                Some(vec![
                    MichelsonV1Expression::Prim(Prim::new(
                        Primitive::Type(Type::String),
                        None,
                        None,
                    )),
                    MichelsonV1Expression::Prim(Prim::new(Primitive::Type(Type::Int), None, None)),
                ]),
                None,
            ))]),
            None,
        ));

        let packed = micheline.pack(Some(&schema))?;

        assert_eq!(packed, "050509020000002407040100000008746573744b65793100a40107040100000008746573744b657932008803");

        Ok(())
    }

    #[test]
    fn test_micheline_pack_3() -> Result<(), TzError> {
        let micheline = MichelsonV1Expression::Prim(Prim::new(
            Primitive::Data(Data::Pair),
            Some(vec![
                MichelsonV1Expression::Literal(Literal::String(String::from(
                    "tz1Ts3m2dXTXB66XN7cg5ALiAvzZY6AxrFd9",
                ))),
                MichelsonV1Expression::Literal(Literal::Int(100)),
            ]),
            None,
        ));
        let schema = MichelsonV1Expression::Prim(Prim::new(
            Primitive::Type(Type::Pair),
            Some(vec![
                MichelsonV1Expression::Prim(Prim::new(Primitive::Type(Type::Address), None, None)),
                MichelsonV1Expression::Prim(Prim::new(Primitive::Type(Type::Int), None, None)),
            ]),
            None,
        ));

        let packed = micheline.pack(Some(&schema))?;

        assert_eq!(
            packed,
            "0507070a0000001600005a374e077b2e539f222af1e61964d7487c8b95fe00a401"
        );

        Ok(())
    }

    #[test]
    fn test_micheline_pack_4() -> Result<(), TzError> {
        let micheline = MichelsonV1Expression::Prim(Prim::new(
            Primitive::Data(Data::Some),
            Some(vec![MichelsonV1Expression::Literal(Literal::String(
                String::from("tz1Ts3m2dXTXB66XN7cg5ALiAvzZY6AxrFd9"),
            ))]),
            None,
        ));
        let schema = MichelsonV1Expression::Prim(Prim::new(
            Primitive::Type(Type::Option),
            Some(vec![MichelsonV1Expression::Prim(Prim::new(
                Primitive::Type(Type::Address),
                None,
                None,
            ))]),
            None,
        ));

        let packed = micheline.pack(Some(&schema))?;

        assert_eq!(
            packed,
            "0505090a0000001600005a374e077b2e539f222af1e61964d7487c8b95fe"
        );

        Ok(())
    }

    #[test]
    fn test_micheline_pack_5() -> Result<(), TzError> {
        let mut micheline = MichelsonV1Expression::Prim(Prim::new(
            Primitive::Data(Data::Left),
            Some(vec![MichelsonV1Expression::Prim(Prim::new(
                Primitive::Data(Data::Left),
                Some(vec![MichelsonV1Expression::Literal(Literal::String(
                    String::from("test"),
                ))]),
                None,
            ))]),
            None,
        ));
        let schema = MichelsonV1Expression::Prim(Prim::new(
            Primitive::Type(Type::Or),
            Some(vec![
                MichelsonV1Expression::Prim(Prim::new(
                    Primitive::Type(Type::Or),
                    Some(vec![
                        MichelsonV1Expression::Prim(Prim::new(
                            Primitive::Type(Type::String),
                            None,
                            None,
                        )),
                        MichelsonV1Expression::Prim(Prim::new(
                            Primitive::Type(Type::Int),
                            None,
                            None,
                        )),
                    ]),
                    None,
                )),
                MichelsonV1Expression::Prim(Prim::new(Primitive::Type(Type::Int), None, None)),
            ]),
            None,
        ));

        let mut packed = micheline.pack(Some(&schema))?;
        assert_eq!(packed, "0505050505010000000474657374");

        micheline = MichelsonV1Expression::Prim(Prim::new(
            Primitive::Data(Data::Left),
            Some(vec![MichelsonV1Expression::Prim(Prim::new(
                Primitive::Data(Data::Right),
                Some(vec![MichelsonV1Expression::Literal(Literal::Int(100))]),
                None,
            ))]),
            None,
        ));

        packed = micheline.pack(Some(&schema))?;
        assert_eq!(packed, "050505050800a401");

        micheline = MichelsonV1Expression::Prim(Prim::new(
            Primitive::Data(Data::Right),
            Some(vec![MichelsonV1Expression::Literal(Literal::Int(100))]),
            None,
        ));

        packed = micheline.pack(Some(&schema))?;
        assert_eq!(packed, "05050800a401");

        Ok(())
    }

    #[test]
    fn test_micheline_pack_6() -> Result<(), TzError> {
        let micheline =
            MichelsonV1Expression::Literal(Literal::String(String::from("NetXdQprcVkpaWU")));
        let schema =
            MichelsonV1Expression::Prim(Prim::new(Primitive::Type(Type::ChainID), None, None));

        let packed = micheline.pack(Some(&schema))?;

        assert_eq!(packed, "050a000000047a06a770");

        Ok(())
    }

    #[test]
    fn test_micheline_pack_7() -> Result<(), TzError> {
        let micheline = MichelsonV1Expression::Literal(Literal::String(String::from("sigNw8i6ihAGn8iwcbgfdA5HNdmBRFVRBGoUPnvmPidnHyqD2HoLq6ZbAxiov9i7FrFgjvuU2Mu6NfxEg9onxQH8PSPsXpPT")));
        let schema =
            MichelsonV1Expression::Prim(Prim::new(Primitive::Type(Type::Signature), None, None));

        let packed = micheline.pack(Some(&schema))?;

        assert_eq!(packed, "050a00000040073a1c8aff3edfb9b5d4dcc02f4ecea06617a267d67d9ae9293d23676b3e17ea0b6d643e4b85c3f0d6e2d47f670f4ab4e826753a799494123d75d56a29d0c105");

        Ok(())
    }

    #[test]
    fn test_micheline_pack_8() -> Result<(), TzError> {
        let micheline = MichelsonV1Expression::Literal(Literal::String(String::from(
            "tz1Ts3m2dXTXB66XN7cg5ALiAvzZY6AxrFd9",
        )));
        let schema =
            MichelsonV1Expression::Prim(Prim::new(Primitive::Type(Type::KeyHash), None, None));

        let packed = micheline.pack(Some(&schema))?;

        assert_eq!(
            packed,
            "050a00000015005a374e077b2e539f222af1e61964d7487c8b95fe"
        );

        Ok(())
    }

    #[test]
    fn test_micheline_pack_9() -> Result<(), TzError> {
        let micheline = MichelsonV1Expression::Literal(Literal::String(String::from(
            "edpkuAJhbFLfJ4zWbQQWTZNGDg7hrcG1m1CBSWVB3iDHChjuzeaZB6",
        )));
        let schema = MichelsonV1Expression::Prim(Prim::new(Primitive::Type(Type::Key), None, None));

        let packed = micheline.pack(Some(&schema))?;

        assert_eq!(
            packed,
            "050a0000002100444e1f4ab90c304a5ac003d367747aab63815f583ff2330ce159d12c1ecceba1"
        );

        Ok(())
    }

    #[test]
    fn test_micheline_pack_10() -> Result<(), TzError> {
        let micheline = MichelsonV1Expression::Literal(Literal::String(String::from(
            "KT1JKNrzC57FtUe3dmYXmm12ucmjDmzbkKrc%transfer",
        )));
        let schema =
            MichelsonV1Expression::Prim(Prim::new(Primitive::Type(Type::Contract), None, None));

        let packed = micheline.pack(Some(&schema))?;

        assert_eq!(
            packed,
            "050a0000001e016ac8111c23353817d663fe21ff7037f9de36a8c4007472616e73666572"
        );

        Ok(())
    }

    #[test]
    fn test_micheline_pack_11() -> Result<(), TzError> {
        let micheline =
            MichelsonV1Expression::Literal(Literal::String(String::from("2020-11-10T07:49:28Z")));
        let schema =
            MichelsonV1Expression::Prim(Prim::new(Primitive::Type(Type::Timestamp), None, None));

        let packed = micheline.pack(Some(&schema))?;

        assert_eq!(packed, "05008898d2fa0b");

        Ok(())
    }
}
