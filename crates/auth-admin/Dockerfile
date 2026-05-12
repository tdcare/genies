# ============================================================
# Stage 1: Builder
# ============================================================
FROM rust:1.90-bookworm AS builder

WORKDIR /usr/src/genies

# 1. Copy manifests only (for dependency caching layer)
COPY Cargo.toml ./
COPY crates/auth-admin/Cargo.toml crates/auth-admin/Cargo.toml
COPY crates/auth/Cargo.toml crates/auth/Cargo.toml
COPY crates/cache/Cargo.toml crates/cache/Cargo.toml
COPY crates/config/Cargo.toml crates/config/Cargo.toml
COPY crates/context/Cargo.toml crates/context/Cargo.toml
COPY crates/core/Cargo.toml crates/core/Cargo.toml
COPY crates/dapr/Cargo.toml crates/dapr/Cargo.toml
COPY crates/ddd/Cargo.toml crates/ddd/Cargo.toml
COPY crates/genies/Cargo.toml crates/genies/Cargo.toml
COPY crates/genies_derive/Cargo.toml crates/genies_derive/Cargo.toml
COPY crates/k8s/Cargo.toml crates/k8s/Cargo.toml
COPY crates/test/Cargo.toml crates/test/Cargo.toml

# 2. Create dummy lib.rs/main.rs for each crate so cargo can resolve & fetch deps
RUN mkdir -p crates/auth-admin/src && echo "fn main() {}" > crates/auth-admin/src/main.rs && echo "" > crates/auth-admin/src/lib.rs \
    && mkdir -p crates/auth/src && echo "" > crates/auth/src/lib.rs \
    && mkdir -p crates/cache/src && echo "" > crates/cache/src/lib.rs \
    && mkdir -p crates/config/src && echo "" > crates/config/src/lib.rs \
    && mkdir -p crates/context/src && echo "" > crates/context/src/lib.rs \
    && mkdir -p crates/core/src && echo "" > crates/core/src/lib.rs \
    && mkdir -p crates/dapr/src && echo "" > crates/dapr/src/lib.rs \
    && mkdir -p crates/ddd/src && echo "" > crates/ddd/src/lib.rs \
    && mkdir -p crates/genies/src && echo "" > crates/genies/src/lib.rs \
    && mkdir -p crates/genies_derive/src && echo "" > crates/genies_derive/src/lib.rs \
    && mkdir -p crates/k8s/src && echo "" > crates/k8s/src/lib.rs \
    && mkdir -p crates/test/src && echo "" > crates/test/src/lib.rs

# 3. Fetch and compile dependencies (this layer is cached if Cargo.toml unchanged)
RUN cargo build --release -p genies_auth_admin --bin auth-admin 2>/dev/null || true

# 4. Copy real source code
COPY crates crates

# 5. Touch source files to invalidate previous build artifacts, then build for real
RUN find crates -name "*.rs" -exec touch {} + \
    && cargo build --release -p genies_auth_admin --bin auth-admin

# ============================================================
# Stage 2: Runtime
# ============================================================
FROM debian:bookworm-slim

RUN apt-get update && apt-get install -y --no-install-recommends \
    libssl3 \
    ca-certificates \
    && rm -rf /var/lib/apt/lists/*

# Set timezone to Asia/Shanghai
RUN cp /usr/share/zoneinfo/Asia/Shanghai /etc/localtime && \
    echo "Asia/Shanghai" > /etc/timezone

WORKDIR /app

# Copy the compiled binary from builder
COPY --from=builder /usr/src/genies/target/release/auth-admin ./auth-admin

# Copy runtime configuration
COPY crates/auth-admin/application.yml ./application.yml

# Copy database migration files
COPY crates/auth-admin/migrations ./migrations

EXPOSE 9099

ENTRYPOINT ["./auth-admin"]
