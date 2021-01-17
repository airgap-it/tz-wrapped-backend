use num_bigint::BigInt;

use crate::tezos::micheline::{data, int, string, MichelsonV1Expression};

pub fn mint_call_micheline(address: String, amount: BigInt) -> MichelsonV1Expression {
    data::right(data::left(data::left(data::left(data::pair(
        string(address),
        int(amount),
    )))))
}

pub fn burn_call_micheline(amount: BigInt) -> MichelsonV1Expression {
    data::right(data::left(data::left(data::right(int(amount)))))
}
