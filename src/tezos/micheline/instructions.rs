use super::{
    prim::Prim,
    primitive::{Instruction, Primitive},
    MichelsonV1Expression,
};

pub fn prim(
    primitive: Instruction,
    args: Option<Vec<MichelsonV1Expression>>,
) -> MichelsonV1Expression {
    MichelsonV1Expression::Prim(Prim::new(Primitive::Instruction(primitive), args, None))
}

pub fn drop() -> MichelsonV1Expression {
    prim(Instruction::Drop, None)
}

pub fn nil(items_type: MichelsonV1Expression) -> MichelsonV1Expression {
    prim(Instruction::Nil, Some(vec![items_type]))
}

pub fn push(
    value_type: MichelsonV1Expression,
    value: MichelsonV1Expression,
) -> MichelsonV1Expression {
    prim(Instruction::Push, Some(vec![value_type, value]))
}

pub fn contract(parameter_type: MichelsonV1Expression) -> MichelsonV1Expression {
    prim(Instruction::Contract, Some(vec![parameter_type]))
}

pub fn if_none(
    branch1: MichelsonV1Expression,
    branch2: MichelsonV1Expression,
) -> MichelsonV1Expression {
    prim(Instruction::IfNone, Some(vec![branch1, branch2]))
}

pub fn unit() -> MichelsonV1Expression {
    prim(Instruction::Unit, None)
}

pub fn fail_with() -> MichelsonV1Expression {
    prim(Instruction::FailWith, None)
}

pub fn pair() -> MichelsonV1Expression {
    prim(Instruction::Pair, None)
}

pub fn cons() -> MichelsonV1Expression {
    prim(Instruction::Cons, None)
}

pub fn transfer_tokens() -> MichelsonV1Expression {
    prim(Instruction::TransferTokens, None)
}
