# syntax=docker/dockerfile:1.7

FROM rust:1.94-slim AS builder

WORKDIR /workspace

RUN apt-get update \
  && apt-get install -y --no-install-recommends ca-certificates pkg-config \
  && rm -rf /var/lib/apt/lists/*

RUN cargo install --locked cargo-nextest

COPY Cargo.toml Cargo.lock ./
COPY crates ./crates

RUN cargo build --release -p sharo-cli -p sharo-daemon -p sharo-tui -p sharo-hazel-mcp

FROM builder AS test

RUN cargo nextest run --workspace \
  && cargo test -p sharo-daemon --test loom_submit_shutdown -- --nocapture \
  && cargo test -p sharo-core --test protocol_tests prop_protocol_roundtrip_preserves_task_summary_fields

FROM debian:trixie-slim AS base

RUN apt-get update \
  && apt-get install -y --no-install-recommends ca-certificates tzdata \
  && rm -rf /var/lib/apt/lists/*

RUN useradd --system --create-home --home-dir /home/sharo --uid 10001 sharo \
  && mkdir -p /var/lib/sharo \
  && chown -R sharo:sharo /var/lib/sharo /home/sharo

ENV HOME=/home/sharo
WORKDIR /home/sharo

FROM base AS runtime

COPY --from=builder /workspace/target/release/sharo /usr/local/bin/sharo
COPY --from=builder /workspace/target/release/sharo-daemon /usr/local/bin/sharo-daemon
COPY --from=builder /workspace/target/release/sharo-tui /usr/local/bin/sharo-tui
COPY --from=builder /workspace/target/release/sharo-hazel-mcp /usr/local/bin/sharo-hazel-mcp

USER sharo
VOLUME ["/var/lib/sharo"]

ENTRYPOINT ["sharo-daemon"]
CMD ["start", "--socket-path", "/tmp/sharo-daemon.sock", "--store-path", "/var/lib/sharo/daemon-store.json"]

