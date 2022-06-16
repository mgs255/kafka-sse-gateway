FROM rust:1.61-slim AS builder

RUN apt-get -y update &&  \
    apt-get -y upgrade &&  \
    apt-get -y install build-essential librdkafka-dev python3 clang
RUN rustup update nightly
RUN export CMAKE_C_COMPILER=clang-11
RUN export CMAKE_CXX_COMPILER=clang++-11
RUN c++ --version

WORKDIR /usr/src/

COPY Cargo.toml Cargo.lock ./

RUN mkdir ./src && echo 'fn main() { println!("Dummy!"); }' > ./src/main.rs
RUN cat src/main.rs
RUN cargo +nightly -Z sparse-registry fetch
RUN cargo +nightly -Z sparse-registry build --release
COPY src/* ./src

RUN cargo +nightly -Z sparse-registry install --path .

FROM debian:stable-slim AS base
RUN apt-get -y update &&  \
    apt-get -y upgrade &&  \
    apt-get -y install librdkafka1

COPY --from=builder /usr/local/cargo/bin/kafka-sse-gateway /app/kafka-sse-gateway
WORKDIR /app
COPY self-signed-certs /app/self-signed-certs
COPY assets /app/assets

USER 1000
CMD ["/app/kafka-sse-gateway"]
