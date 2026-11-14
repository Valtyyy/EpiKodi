FROM rust:1-bookworm
RUN apt-get update && DEBIAN_FRONTEND=noninteractive apt-get install -y --no-install-recommends \
      libsmbclient-dev libclang-dev clang pkg-config && rm -rf /var/lib/apt/lists/*
WORKDIR /app
COPY Cargo.toml ./
COPY src ./src
RUN cargo build --release 2>&1 | tail -30
ENTRYPOINT ["/app/target/release/pavao-spike"]
