FROM rust:1.97-slim AS builder

WORKDIR /app
COPY . .
RUN cargo build --release

FROM debian:bookworm-slim
RUN apt-get update && \
    apt-get install -y --no-install-recommends ca-certificates && \
    rm -rf /var/lib/apt/lists/*

RUN groupadd -r mcp && useradd -r -g mcp -s /sbin/nologin mcp
COPY --from=builder /app/target/release/mcp-web-search /usr/local/bin/

USER mcp
ENTRYPOINT ["mcp-web-search"]
