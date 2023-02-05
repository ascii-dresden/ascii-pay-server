FROM rust:alpine as build

RUN apk update \
    && apk add ca-certificates gcc g++ protoc cmake make binutils clang \
    musl-dev openssl-dev libpq-dev linux-headers

WORKDIR /usr/src/ascii-pay-server
ENV CARGO_TERM_COLOR always

COPY . .
RUN cargo install --path . --locked

FROM alpine:3.16 as dist

RUN apk update \
    && apk add ca-certificates libgcc libstdc++ libpq

WORKDIR /opt/ascii-pay-server
ENTRYPOINT /opt/ascii-pay-server/ascii-pay-server

COPY --from=build /usr/local/cargo/bin/ascii-pay-server /opt/ascii-pay-server/
