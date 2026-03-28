---
name: salvo-tls-acme
description: Configure TLS/HTTPS with automatic certificate management via ACME (Let's Encrypt). Use for production deployments with secure connections.
version: 0.89.3
tags: [security, tls, https, acme, certificate]
---

# Salvo TLS and ACME Configuration

This skill helps configure TLS/HTTPS and automatic certificate management.

## TLS with Rustls

### Setup

```toml
[dependencies]
salvo = { version = "0.89.3", features = ["rustls"] }
```

### Basic TLS Configuration

```rust
use salvo::prelude::*;
use salvo::conn::rustls::{Keycert, RustlsConfig};

#[handler]
async fn hello() -> &'static str {
    "Hello over HTTPS!"
}

#[tokio::main]
async fn main() {
    let router = Router::new().get(hello);

    let config = RustlsConfig::new(
        Keycert::new()
            .cert_from_path("certs/cert.pem")
            .unwrap()
            .key_from_path("certs/key.pem")
            .unwrap()
    );

    let acceptor = TcpListener::new("0.0.0.0:443")
        .rustls(config)
        .bind()
        .await;

    Server::new(acceptor).serve(router).await;
}
```

## ACME (Let's Encrypt) Auto-Certificates

### Setup

```toml
[dependencies]
salvo = { version = "0.89.3", features = ["acme"] }
```

### HTTP-01 Challenge

```rust
use salvo::prelude::*;
use salvo::conn::acme::{AcmeConfig, AcmeListener, ChallengeType};

#[handler]
async fn hello() -> &'static str {
    "Hello with auto-certificate!"
}

#[tokio::main]
async fn main() {
    let router = Router::new().get(hello);

    let config = AcmeConfig::builder()
        .domains(["example.com", "www.example.com"])
        .contacts(["mailto:admin@example.com"])
        .challenge_type(ChallengeType::Http01)
        .cache_path("./acme_cache")
        .build()
        .unwrap();

    let acceptor = AcmeListener::builder()
        .acme_config(config)
        .bind("0.0.0.0:443")
        .await;

    Server::new(acceptor).serve(router).await;
}
```

### TLS-ALPN-01 Challenge

```rust
use salvo::conn::acme::{AcmeConfig, AcmeListener, ChallengeType};

let config = AcmeConfig::builder()
    .domains(["example.com"])
    .contacts(["mailto:admin@example.com"])
    .challenge_type(ChallengeType::TlsAlpn01)
    .cache_path("./acme_cache")
    .build()
    .unwrap();
```

## Force HTTPS Redirect

```rust
use salvo::prelude::*;

#[handler]
async fn force_https(req: &mut Request, depot: &mut Depot, res: &mut Response, ctrl: &mut FlowCtrl) {
    if req.uri().scheme_str() == Some("http") {
        let host = req.header::<String>("Host").unwrap_or_default();
        let path = req.uri().path_and_query().map(|p| p.as_str()).unwrap_or("/");
        let https_url = format!("https://{}{}", host, path);

        res.status_code(StatusCode::MOVED_PERMANENTLY);
        res.headers_mut().insert("Location", https_url.parse().unwrap());
        ctrl.skip_rest();
        return;
    }

    ctrl.call_next(req, depot, res).await;
}
```

## HTTP/2 Support

HTTP/2 is automatically enabled when using Rustls.

## HTTP/3 (QUIC) Support

```toml
[dependencies]
salvo = { version = "0.89.3", features = ["quinn"] }
```

```rust
use salvo::prelude::*;
use salvo::conn::quinn::QuinnListener;

#[tokio::main]
async fn main() {
    let router = Router::new().get(hello);

    let acceptor = QuinnListener::builder()
        .cert_path("certs/cert.pem")
        .key_path("certs/key.pem")
        .bind("0.0.0.0:443")
        .await;

    Server::new(acceptor).serve(router).await;
}
```

## Security Headers for HTTPS

```rust
use salvo::prelude::*;

#[handler]
async fn security_headers(req: &mut Request, depot: &mut Depot, res: &mut Response, ctrl: &mut FlowCtrl) {
    res.headers_mut().insert(
        "Strict-Transport-Security",
        "max-age=31536000; includeSubDomains; preload".parse().unwrap()
    );

    res.headers_mut().insert(
        "Content-Security-Policy",
        "upgrade-insecure-requests".parse().unwrap()
    );

    ctrl.call_next(req, depot, res).await;
}
```

## Complete ACME Example

```rust
use salvo::prelude::*;
use salvo::conn::acme::{AcmeConfig, AcmeListener, ChallengeType};

#[handler]
async fn hello() -> &'static str {
    "Hello with Let's Encrypt!"
}

#[handler]
async fn security_headers(req: &mut Request, depot: &mut Depot, res: &mut Response, ctrl: &mut FlowCtrl) {
    res.headers_mut().insert(
        "Strict-Transport-Security",
        "max-age=31536000; includeSubDomains".parse().unwrap()
    );
    ctrl.call_next(req, depot, res).await;
}

#[tokio::main]
async fn main() {
    let router = Router::new()
        .hoop(security_headers)
        .get(hello);

    let acme_config = AcmeConfig::builder()
        .domains(["example.com", "www.example.com"])
        .contacts(["mailto:admin@example.com"])
        .challenge_type(ChallengeType::Http01)
        .cache_path("./acme_cache")
        .directory_url("https://acme-v02.api.letsencrypt.org/directory")
        .build()
        .unwrap();

    let acceptor = AcmeListener::builder()
        .acme_config(acme_config)
        .bind("0.0.0.0:443")
        .await;

    println!("Server running on https://example.com");
    Server::new(acceptor).serve(router).await;
}
```

## Staging Environment

```rust
let acme_config = AcmeConfig::builder()
    .domains(["example.com"])
    .contacts(["mailto:admin@example.com"])
    .directory_url("https://acme-staging-v02.api.letsencrypt.org/directory")
    .build()
    .unwrap();
```

## Best Practices

1. Use ACME in production for automatic certificate renewal
2. Set HSTS header to force browsers to use HTTPS
3. Enable HTTP/2 for better performance
4. Test with staging before production
5. Cache certificates to disk for restart recovery
6. Monitor certificate expiration
7. Redirect HTTP to HTTPS

## Related Skills

- **salvo-graceful-shutdown**: Graceful shutdown for HTTPS servers
- **salvo-cors**: Security headers for HTTPS
