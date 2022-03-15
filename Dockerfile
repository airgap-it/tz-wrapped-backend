FROM rust:1.59.0 as build
ENV PKG_CONFIG_ALLOW_CROSS=1

WORKDIR /usr/src/tz-wrapped-backend

RUN USER=root cargo init
COPY ./Cargo.toml ./Cargo.toml
COPY ./Cargo.lock ./Cargo.lock
RUN cargo build --release
RUN rm src/*.rs

ADD . ./

RUN rm ./target/release/deps/tz_wrapped_backend*
RUN cargo install --path .

FROM debian:stable-slim

RUN apt-get update \
    && apt-get install -y ca-certificates tzdata \
    && apt-get install -y libssl-dev \
    && apt-get install -y libpq-dev \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /usr/src/tz-wrapped-backend

COPY --from=build /usr/local/cargo/bin/tz-wrapped-backend .
COPY --from=build /usr/src/tz-wrapped-backend/config ./config

EXPOSE 80

CMD ["./tz-wrapped-backend"]
