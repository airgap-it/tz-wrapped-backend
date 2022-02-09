-- Your SQL goes here
CREATE EXTENSION IF NOT EXISTS "uuid-ossp";

CREATE TABLE IF NOT EXISTS node_endpoints (
    id                  uuid NOT NULL DEFAULT uuid_generate_v4() PRIMARY KEY,
    created_at          TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at          TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    name                VARCHAR NOT NULL,
    url                 VARCHAR NOT NULL UNIQUE,
    network             VARCHAR NOT NULL,
    selected            BOOLEAN NOT NULL DEFAULT 'f'
)