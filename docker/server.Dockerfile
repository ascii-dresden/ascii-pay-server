FROM rust:1.56-buster as builder

WORKDIR /usr/src/ascii-pay-server
ENV CARGO_TERM_COLOR always
RUN apt-get update && apt-get install -y libpq-dev libssl-dev build-essential protobuf-compiler libprotobuf-dev

RUN mkdir src/
RUN echo "fn main() {println!(\"if you see this, the build broke\")}" > src/main.rs
RUN echo "fn main() {println!(\"if you see this, the build broke\")}" > build.rs
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

COPY --from=builder /usr/local/cargo/bin/ascii-pay-server /opt/ascii-pay-server/
