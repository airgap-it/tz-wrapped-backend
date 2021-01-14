use crate::tezos::micheline::{instructions, int, sequence, string, types, MichelsonV1Expression};

pub fn mint_call_micheline(
    address: String,
    contract_address: String,
    amount: i64,
    token_id: i64,
) -> MichelsonV1Expression {
    let contract_entrypoint = string(format!("{}%mint", contract_address))
        .prepack(&types::address())
        .unwrap();
    let address_prepacked = string(address).prepack(&types::address()).unwrap();
    sequence(vec![
        instructions::drop(),
        instructions::nil(types::operation()),
        instructions::push(types::address(), contract_entrypoint),
        instructions::contract(types::list(types::pair(
            types::address(),
            types::pair(types::nat(), types::nat()),
        ))),
        sequence(vec![instructions::if_none(
            sequence(vec![instructions::unit(), instructions::fail_with()]),
            sequence(vec![]),
        )]),
        instructions::push(types::mutez(), int(0)),
        instructions::nil(types::pair(
            types::address(),
            types::pair(types::nat(), types::nat()),
        )),
        instructions::push(types::nat(), int(amount)),
        instructions::push(types::nat(), int(token_id)),
        instructions::pair(),
        instructions::push(types::address(), address_prepacked),
        instructions::pair(),
        instructions::cons(),
        instructions::transfer_tokens(),
        instructions::cons(),
    ])
    // data::left(data::right(data::right(sequence(vec![data::pair(
    //     string(address),
    //     data::pair(int(token_id), int(amount)),
    // )]))))
}

pub fn burn_call_micheline(
    contract_address: String,
    amount: i64,
    token_id: i64,
) -> MichelsonV1Expression {
    let contract_entrypoint = string(format!("{}%burn", contract_address))
        .prepack(&types::address())
        .unwrap();
    sequence(vec![
        instructions::drop(),
        instructions::nil(types::operation()),
        instructions::push(types::address(), contract_entrypoint),
        instructions::contract(types::list(types::pair(types::nat(), types::nat()))),
        sequence(vec![instructions::if_none(
            sequence(vec![instructions::unit(), instructions::fail_with()]),
            sequence(vec![]),
        )]),
        instructions::push(types::mutez(), int(0)),
        instructions::nil(types::pair(types::nat(), types::nat())),
        instructions::push(types::nat(), int(amount)),
        instructions::push(types::nat(), int(token_id)),
        instructions::pair(),
        instructions::cons(),
        instructions::transfer_tokens(),
        instructions::cons(),
    ])
    // data::left(data::right(data::left(sequence(vec![data::pair(
    //     int(token_id),
    //     int(amount),
    // )]))))
}
