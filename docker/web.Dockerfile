FROM rust:1.53-buster as builder

WORKDIR /usr/src/ascii-pay-server
RUN apt-get update && apt-get install -y libpq-dev libssl-dev build-essential clang llvm-dev libclang-dev
COPY . .
RUN cargo build --release

FROM debian:buster-slim
RUN apt-get update && apt-get install -y libpq5 libssl-dev
RUN mkdir /opt/ascii-pay-server

WORKDIR /opt/ascii-pay-server
ENTRYPOINT /opt/ascii-pay-server/ascii-pay-server run

COPY --from=builder /usr/src/ascii-pay-server/target/release/ascii-pay-server /usr/src/ascii-pay-server/static /opt/ascii-pay-server/
COPY ./static /opt/ascii-pay-server/static
