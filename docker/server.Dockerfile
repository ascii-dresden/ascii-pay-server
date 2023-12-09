FROM rust:1.74-alpine3.18 as build

RUN apk update && apk add musl-dev openssl-dev openssl-libs-static

WORKDIR /usr/src/ascii-pay-server
ENV CARGO_TERM_COLOR always

RUN echo "fn main() {}" > dummy.rs
COPY Cargo.toml .
COPY Cargo.lock .
RUN sed -i 's#src/main.rs#dummy.rs#' Cargo.toml
RUN cargo build --release
RUN sed -i 's#dummy.rs#src/main.rs#' Cargo.toml
COPY . .
RUN cargo build --release

FROM alpine:3.18 as dist

RUN apk update && apk add libc6-compat libcrypto3 libssl3

WORKDIR /opt/ascii-pay-server
ENTRYPOINT /opt/ascii-pay-server/ascii-pay-server

COPY --from=build /usr/src/ascii-pay-server/target/release/ascii-pay-server /opt/ascii-pay-server/
