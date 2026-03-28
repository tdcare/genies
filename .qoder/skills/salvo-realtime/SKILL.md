---
name: salvo-realtime
description: Implement real-time features using WebSocket and Server-Sent Events (SSE). Use for chat applications, live updates, notifications, and bidirectional communication.
version: 0.89.3
tags: [realtime, websocket, sse, overview]
---

# Salvo Real-time Communication

This skill provides an overview of real-time communication options in Salvo.

## Choosing Between WebSocket and SSE

| Feature | WebSocket | SSE |
|---------|-----------|-----|
| Direction | Bidirectional | Server → Client only |
| Protocol | Custom protocol | HTTP |
| Reconnection | Manual | Automatic |
| Binary data | Yes | No (text only) |
| Firewall friendly | May have issues | Yes (standard HTTP) |
| Complexity | Higher | Lower |

### When to Use WebSocket

- Chat applications
- Online gaming
- Collaborative editing
- Trading platforms
- Any bidirectional real-time data

### When to Use SSE

- Live notifications
- News feeds
- Stock tickers
- Progress updates
- Server monitoring dashboards

## Quick WebSocket Example

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

#[tokio::main]
async fn main() {
    let router = Router::new()
        .push(Router::with_path("ws").goal(ws_handler));

    let acceptor = TcpListener::new("0.0.0.0:8080").bind().await;
    Server::new(acceptor).serve(router).await;
}
```

## Quick SSE Example

```rust
use std::convert::Infallible;
use std::time::Duration;
use futures_util::StreamExt;
use salvo::prelude::*;
use salvo::sse::{self, SseEvent};
use tokio::time::interval;
use tokio_stream::wrappers::IntervalStream;

#[handler]
async fn sse_handler(res: &mut Response) {
    let event_stream = {
        let mut counter: u64 = 0;
        let interval = interval(Duration::from_secs(1));
        let stream = IntervalStream::new(interval);

        stream.map(move |_| {
            counter += 1;
            Ok::<_, Infallible>(SseEvent::default().text(counter.to_string()))
        })
    };

    sse::stream(res, event_stream);
}

#[tokio::main]
async fn main() {
    let router = Router::new()
        .push(Router::with_path("events").get(sse_handler));

    let acceptor = TcpListener::new("0.0.0.0:8080").bind().await;
    Server::new(acceptor).serve(router).await;
}
```

## Broadcasting to Multiple Clients

```rust
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{RwLock, mpsc};
use salvo::websocket::Message;

type Users = Arc<RwLock<HashMap<usize, mpsc::UnboundedSender<Message>>>>;

async fn broadcast(users: &Users, sender_id: usize, message: &str) {
    let formatted = format!("User {}: {}", sender_id, message);
    let users = users.read().await;

    for (&uid, tx) in users.iter() {
        if uid != sender_id {
            let _ = tx.send(Message::text(formatted.clone()));
        }
    }
}
```

## Client-Side Examples

### WebSocket Client (JavaScript)

```javascript
const ws = new WebSocket('ws://localhost:8080/ws');

ws.onopen = () => console.log('Connected');
ws.onmessage = (e) => console.log('Received:', e.data);
ws.onclose = () => console.log('Disconnected');
ws.onerror = (e) => console.error('Error:', e);

ws.send('Hello, Server!');
ws.close();
```

### SSE Client (JavaScript)

```javascript
const source = new EventSource('http://localhost:8080/events');

source.onopen = () => console.log('Connected');
source.onmessage = (e) => console.log('Message:', e.data);
source.onerror = (e) => console.error('Error:', e);

source.addEventListener('notification', (e) => {
    console.log('Notification:', e.data);
});

source.close();
```

## Combining WebSocket and SSE

```rust
let router = Router::new()
    .push(Router::with_path("chat").goal(ws_chat_handler))
    .push(Router::with_path("notifications").get(sse_notifications))
    .push(Router::with_path("feed").get(sse_feed));
```

## Best Practices

### WebSocket

1. Handle disconnections gracefully
2. Implement ping/pong for heartbeat
3. Use message queues for slow clients
4. Authenticate before upgrade
5. Limit message size

### SSE

1. Use keep-alive to prevent timeout
2. Include event IDs for reconnection
3. Set retry interval
4. Use named events
5. Handle client disconnects

## Related Skills

- **salvo-websocket**: Detailed WebSocket implementation guide
- **salvo-sse**: Detailed SSE implementation guide
