---
name: salvo-session
description: Implement session management for user state persistence. Use for login systems, shopping carts, and user preferences.
version: 0.89.3
tags: [security, session, cookie, login]
---

# Salvo Session Management

This skill helps implement session management in Salvo applications.

## Setup

```toml
[dependencies]
salvo = { version = "0.89.3", features = ["session"] }
```

## Basic Session Setup

```rust
use salvo::prelude::*;
use salvo::session::{CookieStore, Session, SessionDepotExt, SessionHandler};

#[tokio::main]
async fn main() {
    let session_handler = SessionHandler::builder(
        CookieStore::new(),
        b"secretabsecretabsecretabsecretabsecretabsecretabsecretabsecretab",
    )
    .build()
    .unwrap();

    let router = Router::new()
        .hoop(session_handler)
        .get(home)
        .push(Router::with_path("login").get(login).post(login))
        .push(Router::with_path("logout").get(logout));

    let acceptor = TcpListener::new("0.0.0.0:8080").bind().await;
    Server::new(acceptor).serve(router).await;
}
```

## Session Operations

### Creating a Session

```rust
use salvo::session::{Session, SessionDepotExt};

#[handler]
async fn login(req: &mut Request, depot: &mut Depot, res: &mut Response) {
    if req.method() == salvo::http::Method::POST {
        let username = req.form::<String>("username").await.unwrap();

        let mut session = Session::new();
        session.insert("username", username).unwrap();
        session.insert("logged_in", true).unwrap();

        depot.set_session(session);
        res.render(Redirect::other("/"));
    } else {
        res.render(Text::Html(LOGIN_FORM));
    }
}
```

### Reading Session Data

```rust
#[handler]
async fn home(depot: &mut Depot, res: &mut Response) {
    if let Some(session) = depot.session_mut() {
        if let Some(username) = session.get::<String>("username") {
            res.render(Text::Html(format!("Hello, {}!", username)));
            return;
        }
    }
    res.render(Text::Html("Please login"));
}
```

### Updating Session Data

```rust
#[handler]
async fn update_preferences(depot: &mut Depot, res: &mut Response) {
    if let Some(session) = depot.session_mut() {
        session.insert("theme", "dark").unwrap();
        let visits: i32 = session.get("visits").unwrap_or(0);
        session.insert("visits", visits + 1).unwrap();
    }
    res.render("Preferences updated");
}
```

### Removing Session Data

```rust
#[handler]
async fn logout(depot: &mut Depot, res: &mut Response) {
    if let Some(session) = depot.session_mut() {
        session.remove("username");
    }
    res.render(Redirect::other("/"));
}
```

## Session Stores

### Cookie Store (Default)

```rust
use salvo::session::CookieStore;

let session_handler = SessionHandler::builder(
    CookieStore::new(),
    b"secretabsecretabsecretabsecretabsecretabsecretabsecretabsecretab",
)
.build()
.unwrap();
```

### Memory Store

```rust
use salvo::session::MemoryStore;

let session_handler = SessionHandler::builder(
    MemoryStore::new(),
    b"secretabsecretabsecretabsecretabsecretabsecretabsecretabsecretab",
)
.build()
.unwrap();
```

## Session Configuration

```rust
use std::time::Duration;
use salvo::session::{CookieStore, SessionHandler};

let session_handler = SessionHandler::builder(
    CookieStore::new(),
    b"secretabsecretabsecretabsecretabsecretabsecretabsecretabsecretab",
)
.session_ttl(Some(Duration::from_secs(3600)))
.cookie_name("session_id")
.cookie_path("/")
.cookie_secure(true)
.cookie_http_only(true)
.cookie_same_site(salvo::http::cookie::SameSite::Strict)
.build()
.unwrap();
```

## Complete Login Example

```rust
use salvo::prelude::*;
use salvo::session::{CookieStore, Session, SessionDepotExt, SessionHandler};

#[handler]
async fn home(depot: &mut Depot, res: &mut Response) {
    let content = if let Some(session) = depot.session_mut()
        && let Some(username) = session.get::<String>("username")
    {
        format!(r#"<h1>Welcome, {username}!</h1><p><a href="/logout">Logout</a></p>"#)
    } else {
        r#"<h1>Welcome, Guest!</h1><p><a href="/login">Login</a></p>"#.to_string()
    };
    res.render(Text::Html(content));
}

#[handler]
async fn login(req: &mut Request, depot: &mut Depot, res: &mut Response) {
    if req.method() == salvo::http::Method::POST {
        let username = req.form::<String>("username").await.unwrap_or_default();
        let password = req.form::<String>("password").await.unwrap_or_default();

        if username == "admin" && password == "password" {
            let mut session = Session::new();
            session.insert("username", username).unwrap();
            session.insert("role", "admin").unwrap();
            depot.set_session(session);
            res.render(Redirect::other("/"));
        } else {
            res.render(Text::Html("Invalid credentials. <a href='/login'>Try again</a>"));
        }
    } else {
        res.render(Text::Html(r#"
            <h1>Login</h1>
            <form method="post">
                <p><input type="text" name="username" placeholder="Username" /></p>
                <p><input type="password" name="password" placeholder="Password" /></p>
                <button type="submit">Login</button>
            </form>
        "#));
    }
}

#[handler]
async fn logout(depot: &mut Depot, res: &mut Response) {
    if let Some(session) = depot.session_mut() {
        session.remove("username");
        session.remove("role");
    }
    res.render(Redirect::other("/"));
}

#[tokio::main]
async fn main() {
    let session_handler = SessionHandler::builder(
        CookieStore::new(),
        b"secretabsecretabsecretabsecretabsecretabsecretabsecretabsecretab",
    )
    .build()
    .unwrap();

    let router = Router::new()
        .hoop(session_handler)
        .get(home)
        .push(Router::with_path("login").get(login).post(login))
        .push(Router::with_path("logout").get(logout));

    let acceptor = TcpListener::new("0.0.0.0:8080").bind().await;
    Server::new(acceptor).serve(router).await;
}
```

## Session with Authentication Middleware

```rust
#[handler]
async fn require_login(
    depot: &mut Depot,
    res: &mut Response,
    ctrl: &mut FlowCtrl,
) {
    let logged_in = depot
        .session_mut()
        .and_then(|s| s.get::<bool>("logged_in"))
        .unwrap_or(false);

    if !logged_in {
        res.render(Redirect::other("/login"));
        ctrl.skip_rest();
    }
}

#[handler]
async fn require_admin(
    depot: &mut Depot,
    res: &mut Response,
    ctrl: &mut FlowCtrl,
) {
    let is_admin = depot
        .session_mut()
        .and_then(|s| s.get::<String>("role"))
        .map(|r| r == "admin")
        .unwrap_or(false);

    if !is_admin {
        res.status_code(StatusCode::FORBIDDEN);
        res.render("Admin access required");
        ctrl.skip_rest();
    }
}
```

## Best Practices

1. Use secure cookies in production (HTTPS)
2. Set appropriate session TTL
3. Use SameSite cookies to prevent CSRF
4. Regenerate session ID after login
5. Clear sensitive data on logout
6. Use memory store for development only

## Related Skills

- **salvo-auth**: Authentication patterns
- **salvo-csrf**: CSRF protection with sessions
