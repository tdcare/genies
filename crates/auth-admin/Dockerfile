# ============================================================
# Stage 1: Builder
# ============================================================
FROM rust:1.87-bookworm AS builder

WORKDIR /usr/src/genies

# Copy the entire workspace (root Cargo.toml and all crates)
COPY Cargo.toml ./
COPY crates crates

# Build auth-admin in release mode
RUN cargo build --release -p genies_auth_admin --bin auth-admin

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
