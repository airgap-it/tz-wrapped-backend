# tz-wrapped-backend

The backend component of the tz-wrapped application.

The frontend can be found [here](https://github.com/airgap-it/tz-wrapped-frontend).

## Dependencies

This project is written in Rust. It uses the [actix-web](https://github.com/actix/actix-web) web framework and [Diesel](https://github.com/diesel-rs/diesel) as the ORM.

In order to run the service locally, Rust and the Diesel cli need to be installed.

To install Rust follow [these instructions](https://www.rust-lang.org/tools/install).

To install the Diesel cli run `cargo install diesel_cli`.

## Build

Run `cargo build` to build the project.

## Running unit tests

Run `cargo test` to execute the unit tests.

## Run the server locally

1. Start the postgres database service by running `docker-compose up -d postgres`
2. Setup the database by running `diesel database reset --database-url postgres://user:password@localhost/tz-wrapped`
3. To send email notifications, the server needs to be able to connect to an SMTP service. The service can be configured in the `config/Default.toml` configuration file.
4. For the local deployment, the contracts to use is configured in the `config/Local.toml` configuration file. See the Configuration section for more information.
5. Run `cargo run`

## Docker

Build the docker image with `docker build -t tz-wrapped-backend:latest .`.

Make sure the postgres service is running and the database is setup as described in the previous section.

Run the built docker image with `docker-compose up -d tz-wrapped-backend`.

The docker container will listen on port 8080, to change that, edit the `docker-compose.yml` file.

## Configuration

The server can be configured by adapting the configuration files in the `config` folder.

For a local deployment, both the `config/Default.toml` and the `config/Local.toml` files will be used, with the value defined in the latter overwriting the values defined in the former.

You can force the server to load different configurations with the `RUN_ENV` environment variable. Valid values are: `Local`, `Development`, `Production`. The corresponding toml config file will be used.

### Server

Server configuration:

```
[server]
address = "0.0.0.0:80"
domain_name = "localhost"
inactivity_timeout_seconds = 1800
```

- **address**: the local address to bind the server to, to listen for incoming requests.
- **domain_name**: the server domain name, this is used to configure CORS and cookies.
- **inactivity_timeout_seconds**: the inactivity timeout in seconds for the logged in user.

### Database

The postgres database configuration:

```
[database]
host = "postgres"
port = "5432"
user = "user"
password = "password"
name = "tz-wrapped"
```

### Tezos

In the configuration files, it is possible to specify the node URL to use:

```
[tezos]
node_url = "https://edonet.smartpy.io"
```

### Contracts

The contract and its multisig contract address and other informations like the name, symbol, etc.:

```
[[contracts]] # Contract config for tzBTC Owner
address = "KT192P1oDzf3fNb7BSEiC1d74KvQf4HrivBE" # address of the contract
multisig = "KT1KiJ1N9wgEVGkgPDYhLBYRBMPy1RG3pN2J" # address of the multisig contract used to interact with the contract
name = "tzBTC - Owner" # The name displayed in the frontend dropdown menu
kind = "fa1" # this can either be fa1 or fa2
token_id = 0 # this value is not important if the kind value is fa1
symbol = "tzBTC"
decimals = 8
```

Also the capabilities of the multisig and the list of gatekeepers need to be configured:

```
[[contracts.capabilities]]
operation_request_kind = "update_keyholders" # what the multisig contract can do, valid values are: update_keyholders, mint, burn
[[contracts.gatekeepers]] # The list of gatekeepers public keys
public_key = "edpkuHG9N83cBavucaLSeeKX3AVjn9wDyFeFmrhaSLqvmBycP5N7Zs"
[[contracts.gatekeepers]]
public_key = "edpktfkToequZjyn3jz3GJobiYApkc5q4xnJiksStYbZkznUdsxDUw"
[[contracts.gatekeepers]]
public_key = "edpktgVTATaPnXTLUV88RmGKVF5GA12QXH1GKPpCcn56htnGpQbk2b"
```
