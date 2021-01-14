use num_derive::FromPrimitive;
use num_traits::FromPrimitive;
use serde::{Deserialize, Serialize};

use super::{super::utils, TzError};

#[derive(Serialize, Deserialize, PartialEq, Debug, Copy, Clone)]
#[serde(untagged)]
pub enum Primitive {
    Data(Data),
    Type(Type),
    Instruction(Instruction),
}

impl Primitive {
    pub fn from(hex_value: &str) -> Result<Self, TzError> {
        let int_value = u8::from_str_radix(hex_value, 16).map_err(|_error| TzError::InvalidType)?;

        let data: Option<Data> = FromPrimitive::from_u8(int_value);
        if let Some(value) = data {
            return Ok(Primitive::Data(value));
        }

        let type_: Option<Type> = FromPrimitive::from_u8(int_value);
        if let Some(value) = type_ {
            return Ok(Primitive::Type(value));
        }

        let instruction: Option<Instruction> = FromPrimitive::from_u8(int_value);
        if let Some(value) = instruction {
            return Ok(Primitive::Instruction(value));
        }

        Err(TzError::InvalidType)
    }
}

#[derive(Serialize, Deserialize, PartialEq, Debug, Copy, Clone, FromPrimitive)]
pub enum Data {
    False = 0x03,
    Elt = 0x04,
    Left = 0x05,
    None = 0x06,
    Pair = 0x07,
    Right = 0x08,
    Some = 0x09,
    True = 0x0a,
    Unit = 0x0b,
}

#[derive(Serialize, Deserialize, PartialEq, Debug, Copy, Clone, FromPrimitive)]
#[serde(rename_all = "snake_case")]
pub enum Type {
    Parameter = 0x00,
    Storage = 0x01,
    Code = 0x02,
    Bool = 0x59,
    Contract = 0x5a,
    Int = 0x5b,
    Key = 0x5c,
    KeyHash = 0x5d,
    Lambda = 0x5e,
    List = 0x5f,
    Map = 0x60,
    BigMap = 0x61,
    Nat = 0x62,
    Option = 0x63,
    Or = 0x64,
    Pair = 0x65,
    Set = 0x66,
    Signature = 0x67,
    String = 0x68,
    Bytes = 0x69,
    Mutez = 0x6a,
    Timestamp = 0x6b,
    Unit = 0x6c,
    Operation = 0x6d,
    Address = 0x6e,
    #[serde(rename = "chain_id")]
    ChainID = 0x74,
}

#[derive(Serialize, Deserialize, PartialEq, Debug, Copy, Clone, FromPrimitive)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum Instruction {
    Pack = 0x0c,
    Unpack = 0x0d,
    Blake2b = 0x0e,
    Sha256 = 0x0f,
    Sha512 = 0x10,
    Abs = 0x11,
    Add = 0x12,
    Amount = 0x13,
    And = 0x14,
    Balance = 0x15,
    Car = 0x16,
    Cdr = 0x17,
    CheckSignature = 0x18,
    Compare = 0x19,
    Concat = 0x1a,
    Cons = 0x1b,
    CreateAccount = 0x1c,
    CreateContract = 0x1d,
    ImplicitAccount = 0x1e,
    Dip = 0x1f,
    Drop = 0x20,
    Dup = 0x21,
    Ediv = 0x22,
    EmptyMap = 0x23,
    EmptySet = 0x24,
    Eq = 0x25,
    Exec = 0x26,
    #[serde(rename = "FAILWITH")]
    FailWith = 0x27,
    Ge = 0x28,
    Get = 0x29,
    Gt = 0x2a,
    HashKey = 0x2b,
    If = 0x2c,
    IfCons = 0x2d,
    IfLeft = 0x2e,
    IfNone = 0x2f,
    Int = 0x30,
    Lambda = 0x31,
    Le = 0x32,
    Left = 0x33,
    Loop = 0x34,
    Lsl = 0x35,
    Lsr = 0x36,
    Lt = 0x37,
    Map = 0x38,
    Mem = 0x39,
    Mul = 0x3a,
    Neg = 0x3b,
    Neq = 0x3c,
    Nil = 0x3d,
    None = 0x3e,
    Not = 0x3f,
    Now = 0x40,
    Or = 0x41,
    Pair = 0x42,
    Push = 0x43,
    Right = 0x44,
    Size = 0x45,
    Some = 0x46,
    Source = 0x47,
    Sender = 0x48,
    #[serde(rename = "SELF")]
    Self_ = 0x49,
    StepsToQuota = 0x4a,
    Sub = 0x4b,
    Swap = 0x4c,
    TransferTokens = 0x4d,
    SetDelegate = 0x4e,
    Unit = 0x4f,
    Update = 0x50,
    Xor = 0x51,
    Iter = 0x52,
    LoopLeft = 0x53,
    Address = 0x54,
    Contract = 0x55,
    Isnat = 0x56,
    Cast = 0x57,
    Rename = 0x58,
    Slice = 0x6f,
    Dig = 0x70,
    Dug = 0x71,
    EmptyBigMap = 0x72,
    Apply = 0x73,
    #[serde(rename = "CHAIN_ID")]
    ChainID = 0x75,
}

impl Primitive {
    pub fn op_code(&self) -> String {
        match self {
            Primitive::Data(value) => {
                let int_value = *value as u8;

                utils::num_to_padded_str(int_value, Some(2), None)
            }
            Primitive::Type(value) => {
                let int_value = *value as u8;

                utils::num_to_padded_str(int_value, Some(2), None)
            }
            Primitive::Instruction(value) => {
                let int_value = *value as u8;

                utils::num_to_padded_str(int_value, Some(2), None)
            }
        }
    }
}

#[cfg(test)]
mod test {
    use super::{Data, Instruction, Primitive, Type};

    #[test]
    fn test_primitive_serialization() -> () {
        let self_ = Primitive::Instruction(Instruction::Self_);
        let self_json = serde_json::json!(self_).to_string();
        assert_eq!(self_json, r#""SELF""#);

        let chain_id = Primitive::Instruction(Instruction::ChainID);
        let chain_id_json = serde_json::json!(chain_id).to_string();
        assert_eq!(chain_id_json, r#""CHAIN_ID""#);

        let steps_to_quota = Primitive::Instruction(Instruction::StepsToQuota);
        let steps_to_quota_json = serde_json::json!(steps_to_quota).to_string();
        assert_eq!(steps_to_quota_json, r#""STEPS_TO_QUOTA""#);

        let big_map = Primitive::Type(Type::BigMap);
        let big_map_json = serde_json::json!(big_map).to_string();
        assert_eq!(big_map_json, r#""big_map""#);

        let some = Primitive::Data(Data::Some);
        let some_json = serde_json::json!(some).to_string();
        assert_eq!(some_json, r#""Some""#);
    }

    #[test]
    fn test_primitive_deserialization() -> Result<(), serde_json::Error> {
        let self_value = serde_json::json!("SELF");
        let self_: Primitive = serde_json::from_value(self_value)?;
        assert_eq!(self_, Primitive::Instruction(Instruction::Self_));

        let chain_id_value = serde_json::json!("CHAIN_ID");
        let chain_id: Primitive = serde_json::from_value(chain_id_value)?;
        assert_eq!(chain_id, Primitive::Instruction(Instruction::ChainID));

        let steps_to_quota_value = serde_json::json!("STEPS_TO_QUOTA");
        let steps_to_quota: Primitive = serde_json::from_value(steps_to_quota_value)?;
        assert_eq!(
            steps_to_quota,
            Primitive::Instruction(Instruction::StepsToQuota)
        );

        let big_map_value = serde_json::json!("big_map");
        let big_map: Primitive = serde_json::from_value(big_map_value)?;
        assert_eq!(big_map, Primitive::Type(Type::BigMap));

        let some_value = serde_json::json!("Some");
        let some: Primitive = serde_json::from_value(some_value)?;
        assert_eq!(some, Primitive::Data(Data::Some));

        Ok(())
    }
}
