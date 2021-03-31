use serde::{Deserialize, Serialize};
use std::{convert::TryFrom, fmt::Display};

use super::{
    super::utils,
    primitive::{self, Instruction, Primitive, Type},
    HexDecodable, HexEncodable, MichelsonV1Expression, TzError,
};

#[derive(Debug, Serialize, Deserialize, PartialEq, Clone)]
pub struct Prim {
    pub prim: Primitive,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub args: Option<Vec<MichelsonV1Expression>>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub annots: Option<Vec<String>>,
}

impl Prim {
    pub fn new(
        prim: Primitive,
        args: Option<Vec<MichelsonV1Expression>>,
        annots: Option<Vec<String>>,
    ) -> Self {
        Prim { prim, args, annots }
    }

    pub fn type_info(
        &self,
    ) -> Result<
        (
            Type,
            Option<&Vec<MichelsonV1Expression>>,
            Option<&Vec<String>>,
        ),
        TzError,
    > {
        match self.prim {
            Primitive::Type(value) => Ok((value, self.args.as_ref(), self.annots.as_ref())),
            _ => Err(TzError::InvalidType),
        }
    }

    fn encoded_annots(&self) -> String {
        match &self.annots {
            Some(v) => {
                let encoded: Vec<String> = v
                    .iter()
                    .map(|annot| hex::encode(annot.as_bytes()))
                    .collect();

                let encoded_string = encoded.join("20");
                let length = encoded_string.len() / 2;

                format!(
                    "{}{}",
                    utils::num_to_padded_str(length, None, None),
                    encoded_string
                )
            }
            None => "".into(),
        }
    }

    pub fn has_annots(&self) -> bool {
        match &self.annots {
            Some(v) => !v.is_empty(),
            None => false,
        }
    }

    pub fn args_count(&self) -> usize {
        match &self.args {
            Some(v) => v.len(),
            None => 0,
        }
    }

    pub fn prepack_instruction(&self) -> Result<Prim, TzError> {
        match self.prim {
            Primitive::Instruction(instruction) => match instruction {
                Instruction::Dip => self.prepack_dip_instruction(),
                Instruction::If
                | Instruction::IfCons
                | Instruction::IfLeft
                | Instruction::IfNone => self.prepack_if_instructions(),
                Instruction::Lambda => self.prepack_lambda_instruction(),
                Instruction::Loop
                | Instruction::LoopLeft
                | Instruction::Map
                | Instruction::Iter => self.prepack_iteration_instructions(),
                Instruction::Push => self.prepack_push_instruction(),
                _ => Ok(self.clone()),
            },
            _ => Err(TzError::InvalidType),
        }
    }

    pub fn normalized(self) -> Self {
        match self.prim {
            Primitive::Data(primitive::Data::Pair) | Primitive::Type(primitive::Type::Pair) => {
                if self.args_count() > 2 {
                    let mut args = self.args.unwrap();
                    let first = args.remove(0);
                    let second = Prim::new(self.prim, Some(args), None).normalized();
                    Prim::new(
                        self.prim,
                        Some(vec![first, MichelsonV1Expression::Prim(second)]),
                        self.annots,
                    )
                } else {
                    self
                }
            }
            _ => self,
        }
    }

    fn prepack_dip_instruction(&self) -> Result<Prim, TzError> {
        match self.args_count() {
            1 => Ok(Prim::new(
                self.prim,
                Some(vec![self
                    .args
                    .as_ref()
                    .unwrap()
                    .first()
                    .unwrap()
                    .prepack_lambda()?]),
                self.annots.clone(),
            )),
            2 => Ok(Prim::new(
                self.prim,
                Some(vec![
                    self.args.as_ref().unwrap().first().unwrap().clone(),
                    self.args
                        .as_ref()
                        .unwrap()
                        .last()
                        .unwrap()
                        .prepack_lambda()?,
                ]),
                self.annots.clone(),
            )),
            _ => Err(TzError::InvalidType),
        }
    }

    fn prepack_if_instructions(&self) -> Result<Prim, TzError> {
        if self.args_count() != 2 {
            return Err(TzError::InvalidType);
        }

        Ok(Prim::new(
            self.prim,
            Some(vec![
                self.args
                    .as_ref()
                    .unwrap()
                    .first()
                    .unwrap()
                    .prepack_lambda()?,
                self.args
                    .as_ref()
                    .unwrap()
                    .last()
                    .unwrap()
                    .prepack_lambda()?,
            ]),
            self.annots.clone(),
        ))
    }

    fn prepack_lambda_instruction(&self) -> Result<Prim, TzError> {
        if self.args_count() != 3 {
            return Err(TzError::InvalidType);
        }

        Ok(Prim::new(
            self.prim,
            Some(vec![
                self.args.as_ref().unwrap().first().unwrap().clone(),
                self.args.as_ref().unwrap()[1].clone(),
                self.args
                    .as_ref()
                    .unwrap()
                    .last()
                    .unwrap()
                    .prepack_lambda()?,
            ]),
            self.annots.clone(),
        ))
    }

    fn prepack_iteration_instructions(&self) -> Result<Prim, TzError> {
        if self.args_count() != 1 {
            return Err(TzError::InvalidType);
        }

        Ok(Prim::new(
            self.prim,
            Some(vec![self
                .args
                .as_ref()
                .unwrap()
                .first()
                .unwrap()
                .prepack_lambda()?]),
            self.annots.clone(),
        ))
    }

    fn prepack_push_instruction(&self) -> Result<Prim, TzError> {
        if self.args_count() != 2 {
            return Err(TzError::InvalidType);
        }

        let schema = self.args.as_ref().unwrap().first().unwrap();
        let data = self.args.as_ref().unwrap().last().unwrap();

        Ok(Prim::new(
            self.prim,
            Some(vec![schema.clone(), data.prepack(schema)?]),
            self.annots.clone(),
        ))
    }
}

impl Display for Prim {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let to_michelson = |name: String| -> String {
            if let Some(args) = self.args.as_ref() {
                let mut michelson = format!("({} ", name.trim_matches('"'));
                for (index, arg) in args.iter().enumerate() {
                    michelson = format!(
                        "{}{}{}",
                        michelson,
                        arg,
                        if index == args.len() - 1 { ")" } else { " " }
                    )
                }

                michelson
            } else {
                name.trim_matches('"').to_owned()
            }
        };

        let value = match self.prim {
            Primitive::Data(data) => {
                let name = serde_json::json!(data).to_string();
                to_michelson(name)
            }
            Primitive::Type(type_) => {
                let name = serde_json::json!(type_).to_string();
                to_michelson(name)
            }
            Primitive::Instruction(instruction) => {
                let name = serde_json::json!(instruction)
                    .to_string()
                    .trim_matches('"')
                    .to_owned();
                if let Some(args) = self.args.as_ref() {
                    let mut michelson = format!("{} ", name);
                    for (index, arg) in args.iter().enumerate() {
                        michelson = format!(
                            "{}{}{}",
                            michelson,
                            arg,
                            if index != args.len() - 1 { " " } else { "" }
                        )
                    }

                    michelson
                } else {
                    name
                }
            }
        };

        write!(f, "{}", value)
    }
}

impl HexEncodable for Prim {
    fn to_hex_encoded(&self) -> Result<String, TzError> {
        let args_count = self.args_count();
        let has_annots = self.has_annots();
        let prefix = MessagePrefix::new(args_count, has_annots)?;
        let op = self.prim.op_code();
        let mut encoded_args: String = match &self.args {
            Some(vec) => vec.iter().map(|arg| arg.to_hex_encoded()).collect(),
            None => Ok("".into()),
        }?;
        if prefix == MessagePrefix::PrimNArgsAnnots {
            let args_length = encoded_args.len() / 2;
            encoded_args = format!(
                "{}{}",
                utils::num_to_padded_str(args_length, None, None),
                encoded_args
            );
        }
        let encoded_annots = self.encoded_annots();
        let result = format!(
            "{}{}{}{}",
            prefix.prefix(),
            op,
            encoded_args,
            encoded_annots
        );

        Ok(result)
    }
}

impl HexDecodable for Prim {
    fn from_hex(encoded: &mut super::ConsumableHexStr) -> Result<Self, TzError>
    where
        Self: Sized,
    {
        let prefix = MessagePrefix::try_from(encoded.consume_bytes(1)?)?;
        let op_code = encoded.consume_bytes(1)?;
        let prim = Primitive::from(op_code)?;
        let args_count = prefix
            .args_count()
            .map(|count| Result::<usize, TzError>::Ok(count))
            .unwrap_or_else(|| encoded.consume_int(None).map(|int| int as usize))?;
        let args = if args_count > 0 {
            let decoded_args: Vec<MichelsonV1Expression> = (0..args_count)
                .map(|_i| MichelsonV1Expression::from_hex(encoded))
                .collect::<Result<Vec<_>, _>>()?;
            Some(decoded_args)
        } else {
            None
        };
        let annots = if prefix.has_annots() {
            let encoded_annots = encoded.consume_lengh_and_value(None)?;
            let annots_list: Vec<String> = encoded_annots.split("20").map(|a| a.into()).collect();

            Some(annots_list)
        } else {
            None
        };
        Ok(Prim { prim, args, annots })
    }
}

#[derive(Debug, PartialEq)]
pub enum MessagePrefix {
    Prim0Args,
    Prim0ArgsAnnots,
    Prim1Arg,
    Prim1ArgAnnots,
    Prim2Args,
    Prim2ArgsAnnots,
    PrimNArgsAnnots,
}

impl MessagePrefix {
    pub fn new(args_count: usize, has_annots: bool) -> Result<Self, TzError> {
        let value = std::cmp::min(2 * args_count + (if !has_annots { 0 } else { 1 }) + 3, 9);
        let value_string = utils::num_to_padded_str(value, Some(2), None);

        Self::try_from(value_string.as_ref())
    }

    pub fn prefix(&self) -> &str {
        match self {
            MessagePrefix::Prim0Args => "03",
            MessagePrefix::Prim0ArgsAnnots => "04",
            MessagePrefix::Prim1Arg => "05",
            MessagePrefix::Prim1ArgAnnots => "06",
            MessagePrefix::Prim2Args => "07",
            MessagePrefix::Prim2ArgsAnnots => "08",
            MessagePrefix::PrimNArgsAnnots => "09",
        }
    }

    pub fn has_annots(&self) -> bool {
        match self {
            MessagePrefix::Prim0ArgsAnnots
            | MessagePrefix::Prim1ArgAnnots
            | MessagePrefix::Prim2ArgsAnnots
            | MessagePrefix::PrimNArgsAnnots => true,
            MessagePrefix::Prim0Args | MessagePrefix::Prim1Arg | MessagePrefix::Prim2Args => false,
        }
    }

    pub fn args_count(&self) -> Option<usize> {
        match self {
            MessagePrefix::Prim0Args | MessagePrefix::Prim0ArgsAnnots => Some(0),
            MessagePrefix::Prim1Arg | MessagePrefix::Prim1ArgAnnots => Some(1),
            MessagePrefix::Prim2Args | MessagePrefix::Prim2ArgsAnnots => Some(2),
            MessagePrefix::PrimNArgsAnnots => None,
        }
    }
}

impl TryFrom<u8> for MessagePrefix {
    type Error = TzError;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        let value_string = utils::num_to_padded_str(value, Some(2), None);

        MessagePrefix::try_from(value_string.as_ref())
    }
}

impl TryFrom<&str> for MessagePrefix {
    type Error = TzError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        match value {
            "03" => Ok(Self::Prim0Args),
            "04" => Ok(Self::Prim0ArgsAnnots),
            "05" => Ok(Self::Prim1Arg),
            "06" => Ok(Self::Prim1ArgAnnots),
            "07" => Ok(Self::Prim2Args),
            "08" => Ok(Self::Prim2ArgsAnnots),
            "09" => Ok(Self::PrimNArgsAnnots),
            _ => Err(TzError::InvalidType),
        }
    }
}
