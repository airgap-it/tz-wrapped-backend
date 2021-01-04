use crate::tezos::micheline::{data, int, string, MichelsonV1Expression};

pub fn mint_call_micheline(address: String, amount: i64) -> MichelsonV1Expression {
    data::right(data::left(data::left(data::left(data::pair(
        string(address),
        int(amount),
    )))))
}

pub fn burn_call_micheline(amount: i64) -> MichelsonV1Expression {
    data::right(data::left(data::left(data::right(int(amount)))))
}
