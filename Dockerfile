FROM rust:1.43.1 as build
ENV PKG_CONFIG_ALLOW_CROSS=1

WORKDIR /usr/src/tz-wrapped-backend
COPY . .

RUN cargo install --path .

FROM gcr.io/distroless/cc-debian10

COPY --from=build /usr/local/cargo/bin/tz-wrapped-backend /usr/local/bin/tz-wrapped-backend

EXPOSE 80

CMD ["tz-wrapped-backend"]