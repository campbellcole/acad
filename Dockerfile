FROM rust:latest as builder

WORKDIR /usr/src/acad

# install cmake (required for building snmalloc)
# RUN apt update
# RUN apt install cmake -y

# create skeleton project
RUN cargo init
COPY .cargo ./.cargo

# copy over dependencies
COPY Cargo.toml Cargo.toml
COPY Cargo.lock Cargo.lock

# build dependencies
RUN --mount=type=cache,target=/usr/local/cargo/registry \
    cargo build --release

# copy over project
COPY src ./src

# build project
RUN --mount=type=cache,target=/usr/local/cargo/registry \
    touch src/main.rs && \
    cargo build --release --features logging

FROM ubuntu:latest

# install yt-dlp
RUN wget https://github.com/yt-dlp/yt-dlp/releases/latest/download/yt-dlp -O /usr/local/bin/yt-dlp && \
    chmod a+rx /usr/local/bin/yt-dlp

COPY --from=builder /usr/src/acad/target/release/acad /acad

# and run it (environment vars must be set in docker-compose.yml)
CMD ["/acad"]