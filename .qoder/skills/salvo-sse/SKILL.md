---
name: salvo-sse
description: Implement Server-Sent Events for real-time server-to-client updates. Use for live feeds, notifications, and streaming data.
version: 0.89.3
tags: [realtime, sse, server-sent-events, streaming]
---

# Salvo Server-Sent Events (SSE)

This skill helps implement Server-Sent Events in Salvo applications.

## Setup

```toml
[dependencies]
salvo = { version = "0.89.3", features = ["sse"] }
futures-util = "0.3"
tokio = { version = "1", features = ["full"] }
tokio-stream = "0.1"
async-stream = "0.3"
```

## Basic SSE Counter

```rust
use std::convert::Infallible;
use std::time::Duration;

use futures_util::StreamExt;
use salvo::prelude::*;
use salvo::sse::{self, SseEvent};
use tokio::time::interval;
use tokio_stream::wrappers::IntervalStream;

#[handler]
async fn sse_counter(res: &mut Response) {
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

#[handler]
async fn index(res: &mut Response) {
    res.render(Text::Html(r#"
        <!DOCTYPE html>
        <html>
        <body>
            <h1>SSE Counter</h1>
            <div id="count">0</div>
            <script>
                const source = new EventSource('/events');
                source.onmessage = (e) => {
                    document.getElementById('count').textContent = e.data;
                };
            </script>
        </body>
        </html>
    "#));
}

#[tokio::main]
async fn main() {
    let router = Router::new()
        .get(index)
        .push(Router::with_path("events").get(sse_counter));

    let acceptor = TcpListener::new("0.0.0.0:8080").bind().await;
    Server::new(acceptor).serve(router).await;
}
```

## SSE Event Types

```rust
use salvo::sse::SseEvent;
use std::time::Duration;

// Simple text event
let event = SseEvent::default().text("Hello, World!");

// Named event
let event = SseEvent::default()
    .name("notification")
    .text("New message received");

// JSON data
let event = SseEvent::default()
    .name("update")
    .json(&serde_json::json!({"count": 42}))?;

// With event ID (for reconnection)
let event = SseEvent::default()
    .id("msg-123")
    .text("Message content");

// With retry suggestion
let event = SseEvent::default()
    .retry(Duration::from_secs(5))
    .text("Reconnect in 5 seconds");

// Comment (keep-alive)
let event = SseEvent::default().comment("keep-alive");
```

## SSE with Keep-Alive

```rust
use salvo::sse::{SseEvent, SseKeepAlive};
use std::time::Duration;

#[handler]
async fn sse_with_keepalive(res: &mut Response) {
    let stream = create_event_stream();

    SseKeepAlive::new(stream)
        .interval(Duration::from_secs(15))
        .text("ping")
        .stream(res);
}
```

## Notification System

```rust
use std::sync::Arc;
use tokio::sync::broadcast;
use salvo::sse::{SseEvent, SseKeepAlive};
use serde::Serialize;

#[derive(Clone, Serialize)]
struct Notification {
    id: u64,
    title: String,
    message: String,
}

#[derive(Clone)]
struct NotificationService {
    sender: broadcast::Sender<Notification>,
}

impl NotificationService {
    fn new() -> Self {
        let (sender, _) = broadcast::channel(100);
        Self { sender }
    }

    fn subscribe(&self) -> broadcast::Receiver<Notification> {
        self.sender.subscribe()
    }

    fn send(&self, notification: Notification) {
        let _ = self.sender.send(notification);
    }
}

#[handler]
async fn notifications(depot: &mut Depot, res: &mut Response) {
    let service = depot.obtain::<NotificationService>().unwrap();
    let mut receiver = service.subscribe();

    let stream = async_stream::stream! {
        while let Ok(notification) = receiver.recv().await {
            yield Ok::<_, salvo::Error>(
                SseEvent::default()
                    .name("notification")
                    .id(notification.id.to_string())
                    .json(&notification)
                    .unwrap()
            );
        }
    };

    SseKeepAlive::new(stream)
        .interval(Duration::from_secs(30))
        .stream(res);
}
```

## Live Data Feed

```rust
use salvo::sse::SseEvent;
use serde::Serialize;
use std::time::Duration;

#[derive(Serialize)]
struct StockPrice {
    symbol: String,
    price: f64,
    change: f64,
}

#[handler]
async fn stock_feed(res: &mut Response) {
    let stream = async_stream::stream! {
        let symbols = vec!["AAPL", "GOOGL", "MSFT"];
        let mut prices: HashMap<&str, f64> = HashMap::new();
        prices.insert("AAPL", 150.0);
        prices.insert("GOOGL", 140.0);
        prices.insert("MSFT", 380.0);

        loop {
            tokio::time::sleep(Duration::from_secs(1)).await;

            for symbol in &symbols {
                let change = (rand::random::<f64>() - 0.5) * 2.0;
                let price = prices.get_mut(symbol).unwrap();
                *price += change;

                let stock = StockPrice {
                    symbol: symbol.to_string(),
                    price: *price,
                    change,
                };

                yield Ok::<_, Infallible>(
                    SseEvent::default()
                        .name("price")
                        .json(&stock)
                        .unwrap()
                );
            }
        }
    };

    sse::stream(res, stream);
}
```

## Best Practices

1. Use keep-alive to prevent connection timeout
2. Include event IDs for reconnection
3. Set retry interval
4. Use named events
5. Handle client disconnects
6. Clean up server resources

## Related Skills

- **salvo-realtime**: Overview of real-time options
- **salvo-websocket**: Bidirectional communication
