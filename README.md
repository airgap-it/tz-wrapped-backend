# tz-wrapped-backend

The beckend component of the tz-wrapped application.

## Dependencies

This project is writtend in Rust. It uses the [actix-web](https://github.com/actix/actix-web) web framework and [Diesel](https://github.com/diesel-rs/diesel) as the ORM.

In order to run the service locally, Rust and the Diesel cli need to be installed.

To install Rust follow [these instructions](https://www.rust-lang.org/tools/install).

To install the Diesel cli run `cargo install diesel_cli`.

## Build

Run `cargo build` to build the project.

## Running unit tests

Run `cargo test` to execute the unit tests.

## Docker

Run `docker build .` to build the docker image.

## Run the server locally

1. Start the postgres detabase service by running `docker-compose up -d postgres`
2. Setup the database by running `diesel database reset --database-url postgres://user:password@localhost/tz-wrapped`
3. The server needs to be able to connect to an SMTP service. The service can be configured in the `config/Default.toml` configuration file.
4. Run `cargo run`
