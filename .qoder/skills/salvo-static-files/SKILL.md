---
name: salvo-static-files
description: Serve static files, directories, and embedded assets. Use for CSS, JavaScript, images, and downloadable content.
version: 0.89.3
tags: [data, static-files, assets, serve-static]
---

# Salvo Static File Serving

This skill helps serve static files in Salvo applications.

## Setup

```toml
[dependencies]
salvo = { version = "0.89.3", features = ["serve-static"] }
rust-embed = "8"  # For embedded files
```

## Serving a Directory

```rust
use salvo::prelude::*;
use salvo::serve_static::StaticDir;

#[tokio::main]
async fn main() {
    let router = Router::with_path("{*path}").get(
        StaticDir::new(["static", "public"])
            .defaults("index.html")
            .auto_list(true)
    );

    let acceptor = TcpListener::new("0.0.0.0:8080").bind().await;
    Server::new(acceptor).serve(router).await;
}
```

## StaticDir Options

```rust
use salvo::serve_static::StaticDir;

let static_handler = StaticDir::new(["static"])
    .defaults("index.html")
    .auto_list(true)
    .include_dot_files(false)
    .cache_control("max-age=3600");
```

## Serving a Single File

```rust
use salvo::prelude::*;
use salvo::serve_static::StaticFile;

#[tokio::main]
async fn main() {
    let router = Router::new()
        .push(Router::with_path("favicon.ico").get(StaticFile::new("static/favicon.ico")))
        .push(Router::with_path("robots.txt").get(StaticFile::new("static/robots.txt")));

    let acceptor = TcpListener::new("0.0.0.0:8080").bind().await;
    Server::new(acceptor).serve(router).await;
}
```

## Embedded Static Files

Embed files at compile time for single-binary deployment:

```rust
use rust_embed::RustEmbed;
use salvo::prelude::*;
use salvo::serve_static::static_embed;

#[derive(RustEmbed)]
#[folder = "static"]
struct Assets;

#[tokio::main]
async fn main() {
    let router = Router::with_path("{*path}").get(
        static_embed::<Assets>()
            .fallback("index.html")
    );

    let acceptor = TcpListener::new("0.0.0.0:8080").bind().await;
    Server::new(acceptor).serve(router).await;
}
```

## Combined API and Static Files

```rust
use salvo::prelude::*;
use salvo::serve_static::StaticDir;

#[handler]
async fn api_users() -> Json<Vec<String>> {
    Json(vec!["Alice".to_string(), "Bob".to_string()])
}

#[tokio::main]
async fn main() {
    let router = Router::new()
        .push(
            Router::with_path("api")
                .push(Router::with_path("users").get(api_users))
        )
        .push(
            Router::with_path("{*path}").get(
                StaticDir::new(["static"])
                    .defaults("index.html")
            )
        );

    let acceptor = TcpListener::new("0.0.0.0:8080").bind().await;
    Server::new(acceptor).serve(router).await;
}
```

## SPA (Single Page Application) Support

```rust
use rust_embed::RustEmbed;
use salvo::prelude::*;
use salvo::serve_static::static_embed;

#[derive(RustEmbed)]
#[folder = "dist"]
struct Assets;

#[tokio::main]
async fn main() {
    let router = Router::new()
        .push(Router::with_path("api/{**rest}").get(api_handler))
        .push(
            Router::with_path("{*path}").get(
                static_embed::<Assets>()
                    .fallback("index.html")
            )
        );

    let acceptor = TcpListener::new("0.0.0.0:8080").bind().await;
    Server::new(acceptor).serve(router).await;
}
```

## Serving Different Asset Types

```rust
use salvo::prelude::*;
use salvo::serve_static::StaticDir;

#[tokio::main]
async fn main() {
    let router = Router::new()
        .push(
            Router::with_path("css/{*path}").get(
                StaticDir::new(["static/css"])
                    .cache_control("max-age=31536000")
            )
        )
        .push(
            Router::with_path("js/{*path}").get(
                StaticDir::new(["static/js"])
                    .cache_control("max-age=31536000")
            )
        )
        .push(
            Router::with_path("images/{*path}").get(
                StaticDir::new(["static/images"])
                    .cache_control("max-age=86400")
            )
        );

    let acceptor = TcpListener::new("0.0.0.0:8080").bind().await;
    Server::new(acceptor).serve(router).await;
}
```

## File Downloads

```rust
use salvo::prelude::*;
use salvo::fs::NamedFile;

#[handler]
async fn download_file(req: &mut Request, res: &mut Response) {
    let filename: String = req.param("filename").unwrap();
    let file_path = format!("downloads/{}", filename);

    match NamedFile::builder(&file_path)
        .attached_name(&filename)
        .send(req.headers(), res)
        .await
    {
        Ok(_) => {}
        Err(_) => {
            res.status_code(StatusCode::NOT_FOUND);
            res.render("File not found");
        }
    }
}
```

## Directory Listing

```rust
use salvo::prelude::*;
use salvo::serve_static::StaticDir;

#[tokio::main]
async fn main() {
    let router = Router::with_path("{*path}").get(
        StaticDir::new(["files"])
            .auto_list(true)
            .include_dot_files(false)
            .defaults("index.html")
    );

    let acceptor = TcpListener::new("0.0.0.0:8080").bind().await;
    Server::new(acceptor).serve(router).await;
}
```

## Protected Static Files

```rust
use salvo::prelude::*;
use salvo::serve_static::StaticDir;

#[handler]
async fn check_auth(
    depot: &mut Depot,
    res: &mut Response,
    ctrl: &mut FlowCtrl,
) {
    let is_authenticated = depot
        .session_mut()
        .and_then(|s| s.get::<bool>("logged_in"))
        .unwrap_or(false);

    if !is_authenticated {
        res.status_code(StatusCode::UNAUTHORIZED);
        res.render("Please login to access files");
        ctrl.skip_rest();
    }
}

#[tokio::main]
async fn main() {
    let router = Router::new()
        .push(
            Router::with_path("public/{*path}").get(
                StaticDir::new(["static/public"])
            )
        )
        .push(
            Router::with_path("private/{*path}")
                .hoop(check_auth)
                .get(StaticDir::new(["static/private"]))
        );

    let acceptor = TcpListener::new("0.0.0.0:8080").bind().await;
    Server::new(acceptor).serve(router).await;
}
```

## Multiple Fallback Directories

```rust
use salvo::serve_static::StaticDir;

let static_handler = StaticDir::new([
    "static/overrides",
    "static/default",
    "node_modules",
])
.defaults("index.html");
```

## Best Practices

1. Use long cache times for versioned assets
2. Set appropriate content types
3. Enable compression for text assets
4. Use CDN for production
5. Embed assets for single-binary deployment
6. Separate public and private files

## Related Skills

- **salvo-compression**: Compress static files
- **salvo-caching**: Cache static file responses
