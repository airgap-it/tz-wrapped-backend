-- Your SQL goes here
CREATE EXTENSION IF NOT EXISTS "uuid-ossp";

CREATE TABLE IF NOT EXISTS contracts (
    id                  uuid NOT NULL DEFAULT uuid_generate_v4() PRIMARY KEY,
    created_at          TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at          TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    pkh                 VARCHAR NOT NULL,
    token_id            INTEGER NOT NULL DEFAULT 0,
    mutisig_pkh         VARCHAR NOT NULL,
    kind                SMALLINT NOT NULL,
    display_name        VARCHAR NOT NULL,

    UNIQUE(pkh, token_id, mutisig_pkh)
);

CREATE TABLE IF NOT EXISTS gatekeepers (
    id                  uuid NOT NULL DEFAULT uuid_generate_v4() PRIMARY KEY,
    created_at          TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at          TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    public_key          VARCHAR NOT NULL UNIQUE,
    contract_id         uuid NOT NULL,

    UNIQUE(public_key, contract_id),
    FOREIGN KEY(contract_id) REFERENCES contracts(id) ON DELETE CASCADE
);

CREATE TABLE IF NOT EXISTS keyholders (
    id                  uuid NOT NULL DEFAULT uuid_generate_v4() PRIMARY KEY,
    created_at          TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at          TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    public_key          VARCHAR NOT NULL UNIQUE,
    contract_id         uuid NOT NULL,

    UNIQUE(public_key, contract_id),
    FOREIGN KEY(contract_id) REFERENCES contracts(id) ON DELETE CASCADE
);

CREATE TABLE IF NOT EXISTS operation_requests (
    id                  uuid NOT NULL DEFAULT uuid_generate_v4() PRIMARY KEY,
    created_at          TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at          TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    requester           uuid NOT NULL,
    destination         uuid NOT NULL,
    target_address      VARCHAR NOT NULL,
    amount              BIGINT NOT NULL,
    kind                SMALLINT NOT NULL,
    gk_signature        VARCHAR NOT NULL,
    chain_id            VARCHAR NOT NULL,
    nonce               INTEGER NOT NULL,
    state               SMALLINT NOT NULL DEFAULT 0,

    FOREIGN KEY(requester) REFERENCES gatekeepers(id) ON DELETE CASCADE,
    FOREIGN KEY(destination) REFERENCES contracts(id) ON DELETE CASCADE
);

CREATE TABLE IF NOT EXISTS operation_approvals (
    id                  uuid NOT NULL DEFAULT uuid_generate_v4() PRIMARY KEY,
    created_at          TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at          TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    approver            uuid NOT NULL,
    request             uuid NOT NULL,
    kh_signature        VARCHAR NOT NULL,

    FOREIGN KEY(approver) REFERENCES keyholders(id) ON DELETE CASCADE,
    FOREIGN KEY(request) REFERENCES operation_requests(id) ON DELETE CASCADE
);