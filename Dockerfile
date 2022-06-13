FROM --platform=$BUILDPLATFORM rust:1.61.0 AS builder

WORKDIR /usr/src/

COPY Cargo.toml Cargo.lock ./

RUN mkdir ./src && echo 'fn main() { println!("Dummy!"); }' > ./src/main.rs
RUN cat src/main.rs
RUN cargo fetch
RUN cargo build --release
COPY src/* ./src

RUN cargo install --path .

FROM --platform=$BUILDPLATFORM debian:stable-slim AS base
RUN apt-get -y update &&  \
    apt-get -y upgrade &&  \
    apt-get -y install librdkafka1

COPY --from=builder /usr/local/cargo/bin/kafka-sse-gateway /app/kafka-sse-gateway
USER 1000
CMD ["/app/kafka-sse-gateway"]
