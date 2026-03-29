FROM rust:1.85

RUN rustup target add wasm32-unknown-unknown \
 && cargo install trunk

WORKDIR /workspace
