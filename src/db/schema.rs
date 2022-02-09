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
    capabilities (id) {
        id -> Uuid,
        created_at -> Timestamp,
        contract_id -> Uuid,
        operation_request_kind -> Int2,
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
        symbol -> Varchar,
        decimals -> Int4,
    }
}

table! {
    node_endpoints (id) {
        id -> Uuid,
        created_at -> Timestamp,
        updated_at -> Timestamp,
        name -> Varchar,
        url -> Varchar,
        network -> Varchar,
        selected -> Bool,
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
        user_id -> Uuid,
        contract_id -> Uuid,
        target_address -> Nullable<Varchar>,
        amount -> Nullable<Numeric>,
        threshold -> Nullable<Int8>,
        kind -> Int2,
        chain_id -> Varchar,
        nonce -> Int8,
        state -> Int2,
        operation_hash -> Nullable<Varchar>,
    }
}

table! {
    proposed_users (id) {
        id -> Uuid,
        created_at -> Timestamp,
        user_id -> Uuid,
        operation_request_id -> Uuid,
        position -> Int4,
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

joinable!(capabilities -> contracts (contract_id));
joinable!(operation_approvals -> operation_requests (operation_request_id));
joinable!(operation_approvals -> users (keyholder_id));
joinable!(operation_requests -> contracts (contract_id));
joinable!(operation_requests -> users (user_id));
joinable!(proposed_users -> operation_requests (operation_request_id));
joinable!(proposed_users -> users (user_id));
joinable!(users -> contracts (contract_id));

allow_tables_to_appear_in_same_query!(
    authentication_challenges,
    capabilities,
    contracts,
    node_endpoints,
    operation_approvals,
    operation_requests,
    proposed_users,
    users,
);
