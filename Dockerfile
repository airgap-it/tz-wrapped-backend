FROM rust:1.47.0 as build
ENV PKG_CONFIG_ALLOW_CROSS=1

WORKDIR /usr/src/tz-wrapped-backend
COPY . .

RUN cargo install --path .

EXPOSE 80

CMD ["tz-wrapped-backend"]