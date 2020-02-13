FROM ubuntu:18.04
WORKDIR /opt/ascii-pay-server
RUN apt update
RUN apt --yes install libssl-dev libpq-dev
COPY ./static ./static
COPY ./target/release/ascii-pay-server .
ENTRYPOINT ["./ascii-pay-server"]