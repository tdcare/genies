# ============================================================
# Stage 1: Builder
# ============================================================
FROM rust:1.90-bookworm AS builder

WORKDIR /usr/src/cdc

# Configure China mirror for crates.io
RUN mkdir -p /usr/local/cargo/ && \
    echo '[source.crates-io]' > /usr/local/cargo/config.toml && \
    echo 'replace-with = "ustc"' >> /usr/local/cargo/config.toml && \
    echo '[source.ustc]' >> /usr/local/cargo/config.toml && \
    echo 'registry = "sparse+https://mirrors.ustc.edu.cn/crates.io-index/"' >> /usr/local/cargo/config.toml

# 1. Copy manifest only (for dependency caching layer)
COPY Cargo.toml ./

# 2. Create dummy main.rs so cargo can resolve & fetch deps
RUN mkdir -p src && echo "fn main() {}" > src/main.rs

# 3. Fetch and compile dependencies (this layer is cached if Cargo.toml unchanged)
RUN cargo build --release 2>/dev/null || true

# 4. Copy real source code
COPY src src

# 5. Touch source files to invalidate previous build artifacts, then build for real
RUN find src -name "*.rs" -exec touch {} + \
    && cargo build --release

# ============================================================
# Stage 2: Runtime
# ============================================================
FROM debian:bookworm-slim

# Use China apt mirror
RUN sed -i 's|deb.debian.org|mirrors.ustc.edu.cn|g' /etc/apt/sources.list.d/debian.sources

RUN apt-get update && apt-get install -y --no-install-recommends \
    libssl3 \
    ca-certificates \
    && rm -rf /var/lib/apt/lists/*

# Set timezone to Asia/Shanghai
RUN cp /usr/share/zoneinfo/Asia/Shanghai /etc/localtime && \
    echo "Asia/Shanghai" > /etc/timezone

WORKDIR /app

# Copy the compiled binary from builder
COPY --from=builder /usr/src/cdc/target/release/cdc ./cdc

# Copy runtime configuration
COPY application.yml ./application.yml

EXPOSE 8010

ENTRYPOINT ["./cdc"]