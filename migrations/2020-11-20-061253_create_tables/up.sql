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

    UNIQUE(pkh, token_id)
);

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

CREATE TABLE IF NOT EXISTS operation_requests (
    id                  uuid NOT NULL DEFAULT uuid_generate_v4() PRIMARY KEY,
    created_at          TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at          TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    requester           uuid NOT NULL,
    destination         uuid NOT NULL,
    target_address      VARCHAR DEFAULT NULL,
    amount              BIGINT NOT NULL,
    kind                SMALLINT NOT NULL,
    gk_signature        VARCHAR NOT NULL,
    chain_id            VARCHAR NOT NULL,
    nonce               BIGINT NOT NULL,
    state               SMALLINT NOT NULL DEFAULT 0,
    operation_hash      VARCHAR DEFAULT NULL,

    FOREIGN KEY(requester) REFERENCES users(id) ON DELETE CASCADE,
    FOREIGN KEY(destination) REFERENCES contracts(id) ON DELETE CASCADE
);

CREATE TABLE IF NOT EXISTS operation_approvals (
    id                  uuid NOT NULL DEFAULT uuid_generate_v4() PRIMARY KEY,
    created_at          TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at          TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    approver            uuid NOT NULL,
    request             uuid NOT NULL,
    kh_signature        VARCHAR NOT NULL,

    FOREIGN KEY(approver) REFERENCES users(id) ON DELETE CASCADE,
    FOREIGN KEY(request) REFERENCES operation_requests(id) ON DELETE CASCADE
);