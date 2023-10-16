FROM rustlang/rust:nightly-bullseye-slim as builder

WORKDIR /usr/src/acad

# create skeleton project
RUN cargo init

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
    cargo build --release

FROM ubuntu:latest

# install ca-certificates & python (required for yt-dlp)
RUN apt update && \
    apt install --no-install-recommends -y ca-certificates wget python3 ffmpeg && \
    rm -rf /var/lib/apt/lists/*

# install yt-dlp
RUN wget https://github.com/yt-dlp/yt-dlp/releases/latest/download/yt-dlp -O /usr/local/bin/yt-dlp && \
    chmod a+rx /usr/local/bin/yt-dlp

COPY --from=builder /usr/src/acad/target/release/acad /acad

# and run it (environment vars must be set in docker-compose.yml)
CMD ["/acad"]