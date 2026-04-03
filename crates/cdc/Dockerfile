FROM rust:1.81  as builder
# Use prebuilt builder image
# FROM rust:1.50-prebuilt as builder

RUN rustup component add rustfmt

# New cargo project and copy Rust dependencies (and store as a separate Docker layer)
# NOTE: must call `cargo vendor` first and add `vendor` folder to git
RUN USER=root 
RUN cargo new cdc
WORKDIR /usr/src/cdc
RUN mkdir -p .cargo
COPY ./.cargo/config.toml .cargo/config.toml
# COPY vendor vendor
COPY Cargo.toml  ./
# COPY build.rs ./

COPY ./src src


RUN cargo build --release 

# Copy the app to an base Docker image, here we use distroless

FROM openeuler/openeuler:22.03-lts
RUN cp /usr/share/zoneinfo/Asia/Shanghai /etc/localtime


COPY --from=builder /usr/src/cdc/target/release/cdc /
COPY application.yml /
EXPOSE 8010

CMD ["/cdc"]