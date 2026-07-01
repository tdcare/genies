# ============================================================
# Stage 1: Web Builder (Vue 3 frontend)
# ============================================================
FROM node:20-bookworm-slim AS web-builder

WORKDIR /usr/src/auth-admin/web

# 1. Copy package manifests (for npm install caching layer)
COPY web/package.json web/package-lock.json ./

# 2. Install dependencies (use npmmirror China mirror)
RUN npm config set registry https://registry.npmmirror.com && npm ci

# 3. Copy web source code and build (outputs to ../static)
COPY web/ ./
RUN npm run build

# ============================================================
# Stage 2: Rust Builder
# ============================================================
FROM rust:1.90-bookworm AS builder

WORKDIR /usr/src/auth-admin

# Configure China mirror for crates.io (rsproxy sparse)
ENV CARGO_REGISTRIES_CRATES_IO_PROTOCOL=sparse
RUN mkdir -p /usr/local/cargo/ && \
    echo '[source.crates-io]' > /usr/local/cargo/config.toml && \
    echo 'replace-with = "rsproxy-sparse"' >> /usr/local/cargo/config.toml && \
    echo '[source.rsproxy-sparse]' >> /usr/local/cargo/config.toml && \
    echo 'registry = "sparse+https://rsproxy.cn/index/"' >> /usr/local/cargo/config.toml

# 1. Copy built frontend assets (required by rust-embed at compile time)
COPY --from=web-builder /usr/src/auth-admin/static static

# 2. Copy manifest and migration files (compile-time embedded)
COPY Cargo.toml ./
COPY migrations/ ./migrations/

# 3. Create dummy lib.rs and main.rs so cargo can resolve & fetch deps
RUN mkdir -p src && echo "fn main() {}" > src/main.rs && echo "" > src/lib.rs

# 4. Fetch and compile dependencies (this layer is cached if Cargo.toml unchanged)
RUN cargo build --release --bin auth-admin 2>/dev/null || true

# 5. Copy real source code
COPY src src

# 6. Touch source files to invalidate previous build artifacts, then build for real
RUN find src -name "*.rs" -exec touch {} + \
    && cargo build --release --bin auth-admin

# ============================================================
# Stage 3: Runtime
# ============================================================
FROM debian:bookworm-slim

# Use China apt mirror (rsproxy does not provide apt, use aliyun)
RUN sed -i 's|deb.debian.org|mirrors.aliyun.com|g' /etc/apt/sources.list.d/debian.sources

RUN apt-get update && apt-get install -y --no-install-recommends \
    libssl3 \
    ca-certificates \
    && rm -rf /var/lib/apt/lists/*

# Set timezone to Asia/Shanghai
RUN cp /usr/share/zoneinfo/Asia/Shanghai /etc/localtime && \
    echo "Asia/Shanghai" > /etc/timezone

WORKDIR /app

# Copy the compiled binary from builder
COPY --from=builder /usr/src/auth-admin/target/release/auth-admin ./auth-admin

# Copy runtime configuration
COPY application.yml ./application.yml

EXPOSE 9099

ENTRYPOINT ["./auth-admin"]
