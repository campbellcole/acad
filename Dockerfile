FROM rust:latest as builder

WORKDIR /usr/src/acad

RUN cargo new --bin acad
COPY Cargo.toml Cargo.lock ./

RUN cargo build --release

RUN rm src/*.rs && rm target/release/deps/acad*

COPY ./src ./src

RUN cargo install --path .

CMD ["acad"]