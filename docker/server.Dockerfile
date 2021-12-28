FROM alpine:3.15 as build

RUN apk update \
    && apk add ca-certificates gcc g++ rust cargo protoc cmake make binutils clang \
    musl-dev openssl-dev libpq-dev linux-headers

WORKDIR /usr/src/ascii-pay-server
ENV CARGO_TERM_COLOR always

RUN mkdir src/
RUN echo "fn main() {println!(\"if you see this, the build broke\")}" > src/main.rs
RUN echo "fn main() {println!(\"if you see this, the build broke\")}" > build.rs
COPY Cargo.lock ./
COPY Cargo.toml ./
RUN cargo install --path . --locked

COPY . .
RUN cargo install --path . --locked

FROM alpine:3.15 as dist

RUN apk update \
    && apk add ca-certificates libgcc libstdc++ libpq

WORKDIR /opt/ascii-pay-server
ENTRYPOINT /opt/ascii-pay-server/ascii-pay-server

COPY --from=build /root/.cargo/bin/ascii-pay-server /opt/ascii-pay-server/
