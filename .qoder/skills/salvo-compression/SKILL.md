---
name: salvo-compression
description: Compress HTTP responses using gzip, brotli, zstd, or deflate. Use for reducing bandwidth and improving load times.
version: 0.89.3
tags: [performance, compression, gzip, brotli]
---

# Salvo Response Compression

This skill helps configure response compression in Salvo applications.

## Setup

```toml
[dependencies]
salvo = { version = "0.89.3", features = ["compression"] }
```

## Basic Usage

```rust
use salvo::prelude::*;
use salvo::compression::Compression;

#[handler]
async fn large_response() -> String {
    "This response will be compressed if the client supports it. ".repeat(100)
}

#[tokio::main]
async fn main() {
    let router = Router::new()
        .hoop(Compression::new())
        .get(large_response);

    let acceptor = TcpListener::new("0.0.0.0:8080").bind().await;
    Server::new(acceptor).serve(router).await;
}
```

## Supported Algorithms

| Algorithm | Description |
|-----------|-------------|
| **Gzip** | Most widely supported, good balance |
| **Brotli (br)** | Best compression ratio, higher CPU |
| **Zstd** | Fast with good compression |
| **Deflate** | Legacy, rarely used alone |

## Configuring Specific Algorithms

```rust
use salvo::compression::{Compression, CompressionLevel};

// Only gzip
let gzip_only = Compression::new()
    .enable_gzip(CompressionLevel::Default);

// Only brotli
let brotli_only = Compression::new()
    .enable_brotli(CompressionLevel::Best);

// Multiple algorithms
let multi = Compression::new()
    .enable_gzip(CompressionLevel::Default)
    .enable_brotli(CompressionLevel::Default)
    .enable_zstd(CompressionLevel::Default);
```

## Compression Levels

```rust
use salvo::compression::CompressionLevel;

CompressionLevel::Fastest  // Fastest speed, lower compression
CompressionLevel::Default  // Balanced speed and compression
CompressionLevel::Best     // Best compression, slower
CompressionLevel::Precise(6)  // Exact level (algorithm-specific)
```

## Minimum Response Size

```rust
let compression = Compression::new()
    .min_length(1024);  // Only compress responses > 1KB
```

## Content Type Filtering

```rust
let compression = Compression::new()
    .content_types(vec![
        "text/html",
        "text/css",
        "text/javascript",
        "application/json",
        "application/xml",
        "image/svg+xml",
    ]);
```

## Different Routes, Different Compression

```rust
use salvo::compression::{Compression, CompressionLevel};
use salvo::prelude::*;

#[tokio::main]
async fn main() {
    let static_compression = Compression::new()
        .enable_brotli(CompressionLevel::Best)
        .enable_gzip(CompressionLevel::Best);

    let api_compression = Compression::new()
        .enable_gzip(CompressionLevel::Fastest)
        .min_length(256);

    let router = Router::new()
        .push(
            Router::with_path("static")
                .hoop(static_compression)
                .get(StaticDir::new("./public"))
        )
        .push(
            Router::with_path("api")
                .hoop(api_compression)
                .get(api_handler)
        );

    let acceptor = TcpListener::new("0.0.0.0:8080").bind().await;
    Server::new(acceptor).serve(router).await;
}
```

## Complete Example

```rust
use salvo::compression::{Compression, CompressionLevel};
use salvo::prelude::*;
use serde::Serialize;

#[derive(Serialize)]
struct LargeResponse {
    items: Vec<String>,
}

#[handler]
async fn large_json() -> Json<LargeResponse> {
    Json(LargeResponse {
        items: (0..1000).map(|i| format!("Item {}", i)).collect(),
    })
}

#[tokio::main]
async fn main() {
    let compression = Compression::new()
        .enable_gzip(CompressionLevel::Default)
        .enable_brotli(CompressionLevel::Default)
        .min_length(512)
        .content_types(vec![
            "text/html",
            "text/css",
            "application/json",
            "application/javascript",
        ]);

    let router = Router::new()
        .hoop(compression)
        .push(Router::with_path("api/data").get(large_json));

    let acceptor = TcpListener::new("0.0.0.0:8080").bind().await;
    Server::new(acceptor).serve(router).await;
}
```

## Best Practices

1. Use compression for text content (HTML, CSS, JS, JSON)
2. Skip already compressed content (JPEG, PNG, MP4)
3. Set minimum size threshold
4. Choose level based on content type
5. Pre-compress static assets at build time
6. Monitor CPU usage

## Related Skills

- **salvo-static-files**: Serve and compress static files
- **salvo-caching**: Combine compression with caching
