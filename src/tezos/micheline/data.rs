use super::{
    prim::Prim,
    primitive::{Data, Primitive},
    MichelsonV1Expression,
};

pub fn prim(primitive: Data, args: Option<Vec<MichelsonV1Expression>>) -> MichelsonV1Expression {
    MichelsonV1Expression::Prim(Prim::new(Primitive::Data(primitive), args, None))
}

pub fn pair(arg1: MichelsonV1Expression, arg2: MichelsonV1Expression) -> MichelsonV1Expression {
    prim(Data::Pair, Some(vec![arg1, arg2]))
}

pub fn left(value: MichelsonV1Expression) -> MichelsonV1Expression {
    prim(Data::Left, Some(vec![value]))
}

pub fn right(value: MichelsonV1Expression) -> MichelsonV1Expression {
    prim(Data::Right, Some(vec![value]))
}

pub fn some(value: MichelsonV1Expression) -> MichelsonV1Expression {
    prim(Data::Some, Some(vec![value]))
}

pub fn none() -> MichelsonV1Expression {
    prim(Data::None, None)
}

pub fn elt(key: MichelsonV1Expression, value: MichelsonV1Expression) -> MichelsonV1Expression {
    prim(Data::Elt, Some(vec![key, value]))
}

pub fn false_() -> MichelsonV1Expression {
    prim(Data::False, None)
}

pub fn true_() -> MichelsonV1Expression {
    prim(Data::True, None)
}

pub fn unit() -> MichelsonV1Expression {
    prim(Data::Unit, None)
}
