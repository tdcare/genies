---
name: salvo-csrf
description: Implement CSRF (Cross-Site Request Forgery) protection using cookie or session storage. Use for protecting forms and state-changing endpoints.
version: 0.89.3
tags: [security, csrf, protection]
---

# Salvo CSRF Protection

This skill helps implement CSRF protection in Salvo applications.

## Setup

```toml
[dependencies]
salvo = { version = "0.89.3", features = ["csrf"] }
```

## Basic CSRF with Cookie Store

```rust
use salvo::csrf::*;
use salvo::prelude::*;
use serde::Deserialize;

#[derive(Deserialize)]
struct FormData {
    csrf_token: String,
    message: String,
}

#[handler]
async fn show_form(depot: &mut Depot, res: &mut Response) {
    let token = depot.csrf_token().unwrap_or_default();
    res.render(Text::Html(format!(r#"
        <form method="post">
            <input type="hidden" name="csrf_token" value="{token}" />
            <input type="text" name="message" />
            <button type="submit">Submit</button>
        </form>
    "#)));
}

#[handler]
async fn handle_form(req: &mut Request, res: &mut Response) {
    let data = req.parse_form::<FormData>().await.unwrap();
    res.render(format!("Message received: {}", data.message));
}

#[tokio::main]
async fn main() {
    let form_finder = FormFinder::new("csrf_token");
    let csrf_handler = bcrypt_cookie_csrf(form_finder);

    let router = Router::new()
        .hoop(csrf_handler)
        .get(show_form)
        .post(handle_form);

    let acceptor = TcpListener::new("0.0.0.0:8080").bind().await;
    Server::new(acceptor).serve(router).await;
}
```

## CSRF Methods

### Bcrypt (No Key Required)

```rust
use salvo::csrf::{bcrypt_cookie_csrf, FormFinder};

let form_finder = FormFinder::new("csrf_token");
let csrf_handler = bcrypt_cookie_csrf(form_finder);
```

### HMAC (32-byte Key)

```rust
use salvo::csrf::{hmac_cookie_csrf, FormFinder};

let key = *b"01234567012345670123456701234567";
let form_finder = FormFinder::new("csrf_token");
let csrf_handler = hmac_cookie_csrf(key, form_finder);
```

### AES-GCM (32-byte Key)

```rust
use salvo::csrf::{aes_gcm_cookie_csrf, FormFinder};

let key = *b"01234567012345670123456701234567";
let form_finder = FormFinder::new("csrf_token");
let csrf_handler = aes_gcm_cookie_csrf(key, form_finder);
```

## CSRF with Session Store

```rust
use salvo::csrf::*;
use salvo::session::{CookieStore as SessionCookieStore, SessionHandler};
use salvo::prelude::*;

#[tokio::main]
async fn main() {
    let session_handler = SessionHandler::builder(
        SessionCookieStore::new(),
        b"secretabsecretabsecretabsecretabsecretabsecretabsecretabsecretab",
    )
    .build()
    .unwrap();

    let form_finder = FormFinder::new("csrf_token");
    let csrf_handler = bcrypt_session_csrf(form_finder);

    let router = Router::new()
        .hoop(session_handler)
        .hoop(csrf_handler)
        .get(show_form)
        .post(handle_form);

    let acceptor = TcpListener::new("0.0.0.0:8080").bind().await;
    Server::new(acceptor).serve(router).await;
}
```

## Token Finders

### Form Finder (POST Body)

```rust
use salvo::csrf::FormFinder;
let finder = FormFinder::new("csrf_token");
```

### Header Finder

```rust
use salvo::csrf::HeaderFinder;
let finder = HeaderFinder::new("X-CSRF-Token");
```

### Query Finder

```rust
use salvo::csrf::QueryFinder;
let finder = QueryFinder::new("csrf_token");
```

## Getting CSRF Token

```rust
use salvo::csrf::CsrfDepotExt;

#[handler]
async fn show_form(depot: &mut Depot, res: &mut Response) {
    let token = depot.csrf_token().unwrap_or_default();

    res.render(Text::Html(format!(r#"
        <form method="post">
            <input type="hidden" name="csrf_token" value="{token}" />
            <!-- form fields -->
        </form>
    "#)));
}
```

## CSRF for AJAX Requests

```rust
use salvo::csrf::{HeaderFinder, hmac_cookie_csrf};

let header_finder = HeaderFinder::new("X-CSRF-Token");
let csrf_handler = hmac_cookie_csrf(*b"01234567012345670123456701234567", header_finder);

// Client JavaScript:
// fetch('/api', {
//     method: 'POST',
//     headers: { 'X-CSRF-Token': token },
//     body: JSON.stringify(data)
// });
```

## Best Practices

1. Use HMAC or AES-GCM in production (Bcrypt is slow)
2. Generate secure keys
3. Session store is more secure than cookie-based
4. Include token in all forms
5. Validate on all state-changing requests
6. Combine with SameSite cookies
7. Rotate tokens after successful submission

## Related Skills

- **salvo-cors**: CORS for cross-origin requests
- **salvo-session**: CSRF with session storage
