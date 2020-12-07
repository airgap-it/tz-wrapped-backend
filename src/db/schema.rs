table! {
    contracts (id) {
        id -> Uuid,
        created_at -> Timestamp,
        updated_at -> Timestamp,
        pkh -> Varchar,
        token_id -> Int4,
        mutisig_pkh -> Varchar,
        kind -> Int2,
        display_name -> Varchar,
    }
}

table! {
    gatekeepers (id) {
        id -> Uuid,
        created_at -> Timestamp,
        updated_at -> Timestamp,
        public_key -> Varchar,
        contract_id -> Uuid,
    }
}

table! {
    keyholders (id) {
        id -> Uuid,
        created_at -> Timestamp,
        updated_at -> Timestamp,
        public_key -> Varchar,
        contract_id -> Uuid,
    }
}

table! {
    operation_approvals (id) {
        id -> Uuid,
        created_at -> Timestamp,
        updated_at -> Timestamp,
        approver -> Uuid,
        request -> Uuid,
        kh_signature -> Varchar,
    }
}

table! {
    operation_requests (id) {
        id -> Uuid,
        created_at -> Timestamp,
        updated_at -> Timestamp,
        requester -> Uuid,
        destination -> Uuid,
        target_address -> Varchar,
        amount -> Int8,
        kind -> Int2,
        gk_signature -> Varchar,
        chain_id -> Varchar,
        nonce -> Int4,
        state -> Int2,
    }
}

joinable!(gatekeepers -> contracts (contract_id));
joinable!(keyholders -> contracts (contract_id));
joinable!(operation_approvals -> keyholders (approver));
joinable!(operation_approvals -> operation_requests (request));
joinable!(operation_requests -> contracts (destination));
joinable!(operation_requests -> gatekeepers (requester));

allow_tables_to_appear_in_same_query!(
    contracts,
    gatekeepers,
    keyholders,
    operation_approvals,
    operation_requests,
);
