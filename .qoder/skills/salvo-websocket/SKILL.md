---
name: salvo-websocket
description: Implement WebSocket connections for real-time bidirectional communication. Use for chat, live updates, gaming, and collaborative features.
version: 0.89.3
tags: [realtime, websocket, bidirectional, chat]
---

# Salvo WebSocket

This skill helps implement WebSocket connections in Salvo applications.

## Setup

```toml
[dependencies]
salvo = { version = "0.89.3", features = ["websocket"] }
futures-util = "0.3"
tokio = { version = "1", features = ["full"] }
```

## Basic WebSocket Echo Server

```rust
use salvo::prelude::*;
use salvo::websocket::WebSocketUpgrade;

#[handler]
async fn ws_handler(req: &mut Request, res: &mut Response) -> Result<(), StatusError> {
    WebSocketUpgrade::new()
        .upgrade(req, res, |mut ws| async move {
            while let Some(msg) = ws.recv().await {
                let msg = match msg {
                    Ok(msg) => msg,
                    Err(_) => return,
                };

                if ws.send(msg).await.is_err() {
                    return;
                }
            }
        })
        .await
}

#[handler]
async fn index(res: &mut Response) {
    res.render(Text::Html(r#"
        <!DOCTYPE html>
        <html>
        <body>
            <h1>WebSocket Echo</h1>
            <input type="text" id="msg" />
            <button onclick="send()">Send</button>
            <div id="output"></div>
            <script>
                const ws = new WebSocket(`ws://${location.host}/ws`);
                ws.onmessage = (e) => {
                    document.getElementById('output').innerHTML += `<p>${e.data}</p>`;
                };
                function send() {
                    ws.send(document.getElementById('msg').value);
                }
            </script>
        </body>
        </html>
    "#));
}

#[tokio::main]
async fn main() {
    let router = Router::new()
        .get(index)
        .push(Router::with_path("ws").goal(ws_handler));

    let acceptor = TcpListener::new("0.0.0.0:8080").bind().await;
    Server::new(acceptor).serve(router).await;
}
```

## Chat Room Example

```rust
use std::collections::HashMap;
use std::sync::LazyLock;
use std::sync::atomic::{AtomicUsize, Ordering};

use futures_util::{FutureExt, StreamExt};
use salvo::prelude::*;
use salvo::websocket::{Message, WebSocket, WebSocketUpgrade};
use tokio::sync::{RwLock, mpsc};
use tokio_stream::wrappers::UnboundedReceiverStream;

type Users = RwLock<HashMap<usize, mpsc::UnboundedSender<Result<Message, salvo::Error>>>>;

static NEXT_USER_ID: AtomicUsize = AtomicUsize::new(1);
static ONLINE_USERS: LazyLock<Users> = LazyLock::new(Users::default);

#[handler]
async fn chat(req: &mut Request, res: &mut Response) -> Result<(), StatusError> {
    WebSocketUpgrade::new()
        .upgrade(req, res, handle_socket)
        .await
}

async fn handle_socket(ws: WebSocket) {
    let my_id = NEXT_USER_ID.fetch_add(1, Ordering::Relaxed);
    println!("New user connected: {}", my_id);

    let (user_ws_tx, mut user_ws_rx) = ws.split();
    let (tx, rx) = mpsc::unbounded_channel();
    let rx = UnboundedReceiverStream::new(rx);

    let send_task = rx.forward(user_ws_tx).map(|result| {
        if let Err(e) = result {
            eprintln!("WebSocket send error: {:?}", e);
        }
    });
    tokio::spawn(send_task);

    ONLINE_USERS.write().await.insert(my_id, tx);

    while let Some(result) = user_ws_rx.next().await {
        match result {
            Ok(msg) => {
                if let Ok(text) = msg.as_str() {
                    broadcast_message(my_id, text).await;
                }
            }
            Err(e) => {
                eprintln!("WebSocket error: {:?}", e);
                break;
            }
        }
    }

    ONLINE_USERS.write().await.remove(&my_id);
    println!("User {} disconnected", my_id);
}

async fn broadcast_message(sender_id: usize, msg: &str) {
    let formatted = format!("<User#{}>: {}", sender_id, msg);

    for (&uid, tx) in ONLINE_USERS.read().await.iter() {
        if uid != sender_id {
            let _ = tx.send(Ok(Message::text(formatted.clone())));
        }
    }
}
```

## Handling Different Message Types

```rust
use salvo::websocket::{Message, WebSocket};

async fn handle_messages(mut ws: WebSocket) {
    while let Some(result) = ws.recv().await {
        let msg = match result {
            Ok(msg) => msg,
            Err(_) => break,
        };

        if msg.is_text() {
            let text = msg.as_str().unwrap();
            println!("Text message: {}", text);
            ws.send(Message::text(format!("You said: {}", text))).await.ok();
        } else if msg.is_binary() {
            let bytes = msg.as_bytes();
            println!("Binary message: {} bytes", bytes.len());
            ws.send(Message::binary(bytes.to_vec())).await.ok();
        } else if msg.is_ping() {
            println!("Ping received");
        } else if msg.is_close() {
            println!("Close requested");
            break;
        }
    }
}
```

## WebSocket with Authentication

```rust
use salvo::prelude::*;
use salvo::websocket::WebSocketUpgrade;
use salvo::jwt_auth::JwtAuthDepotExt;

#[handler]
async fn ws_authenticated(
    req: &mut Request,
    depot: &mut Depot,
    res: &mut Response,
) -> Result<(), StatusError> {
    let token = depot.jwt_auth_data::<Claims>();
    if token.is_none() {
        return Err(StatusError::unauthorized());
    }

    let user_id = token.unwrap().claims.user_id;

    WebSocketUpgrade::new()
        .upgrade(req, res, move |mut ws| async move {
            println!("Authenticated user {} connected", user_id);

            while let Some(msg) = ws.recv().await {
                match msg {
                    Ok(msg) => {
                        if ws.send(msg).await.is_err() {
                            break;
                        }
                    }
                    Err(_) => break,
                }
            }
        })
        .await
}
```

## Best Practices

1. Handle disconnections gracefully
2. Implement ping/pong heartbeat
3. Use message queues for slow clients
4. Authenticate before upgrade
5. Limit message size
6. Clean up resources on disconnect

## Related Skills

- **salvo-realtime**: Overview of real-time options
- **salvo-sse**: Server-Sent Events alternative
