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
        target_address -> Nullable<Varchar>,
        amount -> Int8,
        kind -> Int2,
        gk_signature -> Varchar,
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

joinable!(operation_approvals -> operation_requests (request));
joinable!(operation_approvals -> users (approver));
joinable!(operation_requests -> contracts (destination));
joinable!(operation_requests -> users (requester));
joinable!(users -> contracts (contract_id));

allow_tables_to_appear_in_same_query!(
    contracts,
    operation_approvals,
    operation_requests,
    users,
);
