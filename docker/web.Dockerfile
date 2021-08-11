FROM rust:1.53-buster as builder

WORKDIR /usr/src/ascii-pay-server
ENV CARGO_TERM_COLOR always
RUN apt-get update && apt-get install -y libpq-dev libssl-dev build-essential clang llvm-dev libclang-dev

RUN cargo new --bin /usr/src/ascii-pay-server
COPY Cargo.lock ./
COPY Cargo.toml ./
RUN cargo install --path . --locked

COPY . .
RUN cargo install --path . --locked

FROM debian:buster-slim
RUN apt-get update && apt-get install -y libpq5 libssl-dev
RUN mkdir /opt/ascii-pay-server

WORKDIR /opt/ascii-pay-server
ENTRYPOINT /opt/ascii-pay-server/ascii-pay-server run

COPY --from=builder /usr/local/cargo/bin/ascii-pay-server /usr/src/ascii-pay-server/static /opt/ascii-pay-server/
COPY ./static /opt/ascii-pay-server/static
