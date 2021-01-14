use super::{
    prim::Prim,
    primitive::{Primitive, Type},
    MichelsonV1Expression,
};

pub fn prim(primitive: Type, args: Option<Vec<MichelsonV1Expression>>) -> MichelsonV1Expression {
    MichelsonV1Expression::Prim(Prim::new(Primitive::Type(primitive), args, None))
}

pub fn pair(type1: MichelsonV1Expression, type2: MichelsonV1Expression) -> MichelsonV1Expression {
    prim(Type::Pair, Some(vec![type1, type2]))
}

pub fn chain_id() -> MichelsonV1Expression {
    prim(Type::ChainID, None)
}

pub fn contract() -> MichelsonV1Expression {
    prim(Type::Contract, None)
}

pub fn string() -> MichelsonV1Expression {
    prim(Type::String, None)
}

pub fn address() -> MichelsonV1Expression {
    prim(Type::Address, None)
}

pub fn signature() -> MichelsonV1Expression {
    prim(Type::Signature, None)
}

pub fn key_hash() -> MichelsonV1Expression {
    prim(Type::KeyHash, None)
}

pub fn key() -> MichelsonV1Expression {
    prim(Type::Key, None)
}

pub fn int() -> MichelsonV1Expression {
    prim(Type::Int, None)
}

pub fn nat() -> MichelsonV1Expression {
    prim(Type::Nat, None)
}

pub fn timestamp() -> MichelsonV1Expression {
    prim(Type::Timestamp, None)
}

pub fn list(value: MichelsonV1Expression) -> MichelsonV1Expression {
    prim(Type::List, Some(vec![value]))
}

pub fn or(left_t: MichelsonV1Expression, right_t: MichelsonV1Expression) -> MichelsonV1Expression {
    prim(Type::Or, Some(vec![left_t, right_t]))
}

pub fn option(value: MichelsonV1Expression) -> MichelsonV1Expression {
    prim(Type::Option, Some(vec![value]))
}

pub fn map(key_t: MichelsonV1Expression, value_t: MichelsonV1Expression) -> MichelsonV1Expression {
    prim(Type::Map, Some(vec![key_t, value_t]))
}

pub fn operation() -> MichelsonV1Expression {
    prim(Type::Operation, None)
}

pub fn mutez() -> MichelsonV1Expression {
    prim(Type::Mutez, None)
}

pub fn lambda(
    parameter: MichelsonV1Expression,
    return_type: MichelsonV1Expression,
) -> MichelsonV1Expression {
    prim(Type::Lambda, Some(vec![parameter, return_type]))
}

pub fn unit() -> MichelsonV1Expression {
    prim(Type::Unit, None)
}
