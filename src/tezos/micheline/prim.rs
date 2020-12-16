use serde::{Deserialize, Serialize};

use super::{
    super::utils,
    primitive::{Primitive, Type},
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

    pub fn type_info(&self) -> Result<(Type, Option<&Vec<MichelsonV1Expression>>), TzError> {
        match self.prim {
            Primitive::Type(value) => Ok((value, self.args.as_ref())),
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
            None => String::from(""),
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
}

impl HexEncodable for Prim {
    fn to_hex_encoded(&self) -> Result<String, TzError> {
        let args_count = self.args_count();
        let has_annots = self.has_annots();
        let prefix = MessagePrefix::new(args_count, has_annots)?;
        let op = self.prim.op_code();
        let mut encoded_args = match &self.args {
            Some(vec) => vec.iter().map(|arg| arg.to_hex_encoded()).collect(),
            None => Ok(String::from("")),
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
        let prefix = MessagePrefix::from(encoded.consume_bytes(1)?)?;
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
            let annots_list: Vec<String> = encoded_annots
                .split("20")
                .map(|a| String::from(a))
                .collect();

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

        Self::from(&value_string)
    }

    pub fn from_int(value: u8) -> Result<Self, TzError> {
        let value_string = utils::num_to_padded_str(value, Some(2), None);

        Self::from(&value_string)
    }

    pub fn from(value: &str) -> Result<Self, TzError> {
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
