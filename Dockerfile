FROM rust:latest as builder

WORKDIR /usr/src/acad

RUN cargo new --bin acad
COPY Cargo.toml Cargo.lock ./