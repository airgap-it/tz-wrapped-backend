-- Your SQL goes here
CREATE EXTENSION IF NOT EXISTS "uuid-ossp";

CREATE TABLE IF NOT EXISTS contracts (
    id                  uuid NOT NULL DEFAULT uuid_generate_v4() PRIMARY KEY,
    created_at          TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at          TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    pkh                 VARCHAR NOT NULL,
    token_id            INTEGER NOT NULL DEFAULT 0,
    multisig_pkh        VARCHAR NOT NULL,
    kind                SMALLINT NOT NULL,
    display_name        VARCHAR NOT NULL,
    min_approvals       INTEGER NOT NULL,
    symbol              VARCHAR NOT NULL,
    decimals            INTEGER NOT NULL DEFAULT 0,

    UNIQUE(pkh, multisig_pkh, token_id)
);

SELECT diesel_manage_updated_at('contracts');

CREATE TABLE IF NOT EXISTS users (
    id                  uuid NOT NULL DEFAULT uuid_generate_v4() PRIMARY KEY,
    created_at          TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at          TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    public_key          VARCHAR NOT NULL,
    address             VARCHAR NOT NULL,
    contract_id         uuid NOT NULL,
    kind                SMALLINT NOT NULL,
    state               SMALLINT NOT NULL DEFAULT 0,
    display_name        VARCHAR NOT NULL,
    email               VARCHAR DEFAULT NULL,

    UNIQUE(public_key, contract_id, kind),
    FOREIGN KEY(contract_id) REFERENCES contracts(id) ON DELETE CASCADE
);

SELECT diesel_manage_updated_at('users');

CREATE TABLE IF NOT EXISTS operation_requests (
    id                  uuid NOT NULL DEFAULT uuid_generate_v4() PRIMARY KEY,
    created_at          TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at          TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    user_id             uuid NOT NULL,
    contract_id         uuid NOT NULL,
    target_address      VARCHAR DEFAULT NULL,
    amount              NUMERIC(1000, 0) DEFAULT NULL,
    threshold           BIGINT DEFAULT NULL,
    kind                SMALLINT NOT NULL,
    chain_id            VARCHAR NOT NULL,
    nonce               BIGINT NOT NULL,
    state               SMALLINT NOT NULL DEFAULT 0,
    operation_hash      VARCHAR DEFAULT NULL,

    UNIQUE(contract_id, nonce),
    FOREIGN KEY(user_id) REFERENCES users(id) ON DELETE CASCADE,
    FOREIGN KEY(contract_id) REFERENCES contracts(id) ON DELETE CASCADE
);

SELECT diesel_manage_updated_at('operation_requests');

CREATE TABLE IF NOT EXISTS operation_approvals (
    id                      uuid NOT NULL DEFAULT uuid_generate_v4() PRIMARY KEY,
    created_at              TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at              TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    keyholder_id            uuid NOT NULL,
    operation_request_id    uuid NOT NULL,
    signature               VARCHAR NOT NULL,

    UNIQUE(keyholder_id, operation_request_id),
    FOREIGN KEY(keyholder_id) REFERENCES users(id) ON DELETE CASCADE,
    FOREIGN KEY(operation_request_id) REFERENCES operation_requests(id) ON DELETE CASCADE
);

SELECT diesel_manage_updated_at('operation_approvals');

CREATE TABLE IF NOT EXISTS proposed_users (
    id                      uuid NOT NULL DEFAULT uuid_generate_v4() PRIMARY KEY,
    created_at              TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    user_id                 uuid NOT NULL,
    operation_request_id    uuid NOT NULL,
    position                INTEGER NOT NULL,

    UNIQUE(user_id, operation_request_id),
    FOREIGN KEY(user_id) REFERENCES users(id) ON DELETE CASCADE,
    FOREIGN KEY(operation_request_id) REFERENCES operation_requests(id) ON DELETE CASCADE
);

CREATE TABLE IF NOT EXISTS authentication_challenges (
    id                  uuid NOT NULL DEFAULT uuid_generate_v4() PRIMARY KEY,
    created_at          TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at          TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    expires_at          TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP + (5 * interval '1 minute'),
    address             VARCHAR NOT NULL,
    challenge           VARCHAR NOT NULL,
    state               SMALLINT NOT NULL DEFAULT 0
);

SELECT diesel_manage_updated_at('authentication_challenges');

CREATE TABLE IF NOT EXISTS capabilities (
    id                      uuid NOT NULL DEFAULT uuid_generate_v4() PRIMARY KEY,
    created_at              TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    contract_id             uuid NOT NULL,
    operation_request_kind  SMALLINT NOT NULL,

    UNIQUE(contract_id, operation_request_kind),
    FOREIGN KEY(contract_id) REFERENCES contracts(id) ON DELETE CASCADE
);