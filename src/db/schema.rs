table! {
    authentication_challenges (id) {
        id -> Uuid,
        created_at -> Timestamp,
        updated_at -> Timestamp,
        expires_at -> Timestamp,
        address -> Varchar,
        challenge -> Varchar,
        state -> Int2,
    }
}

table! {
    contracts (id) {
        id -> Uuid,
        created_at -> Timestamp,
        updated_at -> Timestamp,
        pkh -> Varchar,
        token_id -> Int4,
        multisig_pkh -> Varchar,
        kind -> Int2,
        display_name -> Varchar,
        min_approvals -> Int4,
        decimals -> Int4,
    }
}

table! {
    operation_approvals (id) {
        id -> Uuid,
        created_at -> Timestamp,
        updated_at -> Timestamp,
        keyholder_id -> Uuid,
        operation_request_id -> Uuid,
        signature -> Varchar,
    }
}

table! {
    operation_requests (id) {
        id -> Uuid,
        created_at -> Timestamp,
        updated_at -> Timestamp,
        gatekeeper_id -> Uuid,
        contract_id -> Uuid,
        target_address -> Nullable<Varchar>,
        amount -> Numeric,
        kind -> Int2,
        chain_id -> Varchar,
        nonce -> Int8,
        state -> Int2,
        operation_hash -> Nullable<Varchar>,
    }
}

table! {
    users (id) {
        id -> Uuid,
        created_at -> Timestamp,
        updated_at -> Timestamp,
        public_key -> Varchar,
        address -> Varchar,
        contract_id -> Uuid,
        kind -> Int2,
        state -> Int2,
        display_name -> Varchar,
        email -> Nullable<Varchar>,
    }
}

joinable!(operation_approvals -> operation_requests (operation_request_id));
joinable!(operation_approvals -> users (keyholder_id));
joinable!(operation_requests -> contracts (contract_id));
joinable!(operation_requests -> users (gatekeeper_id));
joinable!(users -> contracts (contract_id));

allow_tables_to_appear_in_same_query!(
    authentication_challenges,
    contracts,
    operation_approvals,
    operation_requests,
    users,
);
