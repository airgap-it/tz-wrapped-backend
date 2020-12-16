use crate::tezos::micheline::{
    literal::Literal, prim::Prim, primitive::Data, primitive::Primitive, MichelsonV1Expression,
};

pub fn mint_call_micheline(address: String, amount: i64) -> MichelsonV1Expression {
    MichelsonV1Expression::Prim(Prim::new(
        Primitive::Data(Data::Right),
        Some(vec![MichelsonV1Expression::Prim(Prim::new(
            Primitive::Data(Data::Left),
            Some(vec![MichelsonV1Expression::Prim(Prim::new(
                Primitive::Data(Data::Left),
                Some(vec![MichelsonV1Expression::Prim(Prim::new(
                    Primitive::Data(Data::Left),
                    Some(vec![MichelsonV1Expression::Prim(Prim::new(
                        Primitive::Data(Data::Pair),
                        Some(vec![
                            MichelsonV1Expression::Literal(Literal::String(address)),
                            MichelsonV1Expression::Literal(Literal::Int(amount)),
                        ]),
                        None,
                    ))]),
                    None,
                ))]),
                None,
            ))]),
            None,
        ))]),
        None,
    ))
}

pub fn burn_call_micheline(amount: i64) -> MichelsonV1Expression {
    MichelsonV1Expression::Prim(Prim::new(
        Primitive::Data(Data::Right),
        Some(vec![MichelsonV1Expression::Prim(Prim::new(
            Primitive::Data(Data::Left),
            Some(vec![MichelsonV1Expression::Prim(Prim::new(
                Primitive::Data(Data::Left),
                Some(vec![MichelsonV1Expression::Prim(Prim::new(
                    Primitive::Data(Data::Right),
                    Some(vec![MichelsonV1Expression::Literal(Literal::Int(amount))]),
                    None,
                ))]),
                None,
            ))]),
            None,
        ))]),
        None,
    ))
}
