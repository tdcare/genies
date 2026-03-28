---
name: salvo-caching
description: Implement caching strategies for improved performance. Use for reducing database load and speeding up responses.
version: 0.89.3
tags: [performance, caching, cache-control, etag]
---

# Salvo Caching Strategies

This skill helps implement caching in Salvo applications for better performance.

## Setup

```toml
[dependencies]
salvo = "0.89.3"
moka = { version = "0.12", features = ["future"] }
tokio = { version = "1", features = ["full"] }
```

## HTTP Cache Headers

```rust
use salvo::prelude::*;

#[handler]
async fn cached_response(res: &mut Response) -> &'static str {
    res.headers_mut().insert(
        "Cache-Control",
        "public, max-age=3600".parse().unwrap()
    );

    "This response will be cached by browsers"
}
```

### Cache-Control Directives

```rust
// Public caching (can be cached by proxies)
res.headers_mut().insert("Cache-Control", "public, max-age=3600".parse().unwrap());

// Private caching (only browser can cache)
res.headers_mut().insert("Cache-Control", "private, max-age=3600".parse().unwrap());

// No caching
res.headers_mut().insert("Cache-Control", "no-store".parse().unwrap());

// Stale while revalidate
res.headers_mut().insert(
    "Cache-Control",
    "public, max-age=3600, stale-while-revalidate=86400".parse().unwrap()
);
```

## ETag for Validation

```rust
use salvo::prelude::*;
use std::hash::{Hash, Hasher};
use std::collections::hash_map::DefaultHasher;

#[handler]
async fn with_etag(req: &mut Request, res: &mut Response) -> Result<Json<Data>, StatusError> {
    let data = get_data().await?;

    let mut hasher = DefaultHasher::new();
    format!("{:?}", data).hash(&mut hasher);
    let etag = format!("\"{}\"", hasher.finish());

    if let Some(if_none_match) = req.header::<String>("If-None-Match") {
        if if_none_match == etag {
            res.status_code(StatusCode::NOT_MODIFIED);
            return Err(StatusError::not_modified());
        }
    }

    res.headers_mut().insert("ETag", etag.parse().unwrap());
    Ok(Json(data))
}
```

## Using Moka for Caching

```rust
use salvo::prelude::*;
use moka::future::Cache;
use std::time::Duration;

type AppCache = Cache<String, String>;

async fn create_cache() -> AppCache {
    Cache::builder()
        .max_capacity(10_000)
        .time_to_live(Duration::from_secs(300))
        .build()
}

#[handler]
async fn cached_handler(req: &mut Request, depot: &mut Depot) -> Result<String, StatusError> {
    let cache = depot.obtain::<AppCache>().unwrap();
    let key = req.uri().path().to_string();

    if let Some(cached) = cache.get(&key).await {
        return Ok(cached);
    }

    let result = expensive_computation().await?;
    cache.insert(key, result.clone()).await;

    Ok(result)
}

#[tokio::main]
async fn main() {
    let cache = create_cache().await;

    let router = Router::new()
        .hoop(affix_state::inject(cache))
        .get(cached_handler);

    let acceptor = TcpListener::new("0.0.0.0:8080").bind().await;
    Server::new(acceptor).serve(router).await;
}
```

## Database Query Caching

```rust
use salvo::prelude::*;
use moka::future::Cache;
use serde::{Deserialize, Serialize};
use std::time::Duration;

#[derive(Clone, Serialize, Deserialize)]
struct User {
    id: i64,
    name: String,
    email: String,
}

type UserCache = Cache<i64, User>;

#[handler]
async fn get_user(req: &mut Request, depot: &mut Depot) -> Result<Json<User>, StatusError> {
    let id = req.param::<i64>("id").ok_or_else(|| StatusError::bad_request())?;
    let cache = depot.obtain::<UserCache>().unwrap();
    let pool = depot.obtain::<PgPool>().unwrap();

    if let Some(user) = cache.get(&id).await {
        return Ok(Json(user));
    }

    let user = sqlx::query_as::<_, User>("SELECT id, name, email FROM users WHERE id = $1")
        .bind(id)
        .fetch_optional(pool)
        .await
        .map_err(|_| StatusError::internal_server_error())?
        .ok_or_else(|| StatusError::not_found())?;

    cache.insert(id, user.clone()).await;

    Ok(Json(user))
}
```

## Cache Invalidation

```rust
use moka::future::Cache;

struct CacheService {
    user_cache: Cache<i64, User>,
}

impl CacheService {
    async fn invalidate_user(&self, id: i64) {
        self.user_cache.invalidate(&id).await;
    }

    async fn invalidate_all_users(&self) {
        self.user_cache.invalidate_all();
    }
}

#[handler]
async fn update_user(
    req: &mut Request,
    depot: &mut Depot
) -> Result<StatusCode, StatusError> {
    let id = req.param::<i64>("id").unwrap();

    // Update in database...

    // Invalidate cache
    let cache = depot.obtain::<UserCache>().unwrap();
    cache.invalidate(&id).await;

    Ok(StatusCode::OK)
}
```

## Complete Caching Example

```rust
use salvo::prelude::*;
use moka::future::Cache;
use serde::Serialize;
use std::time::Duration;

#[derive(Clone, Serialize)]
struct Product {
    id: i64,
    name: String,
    price: f64,
}

type ProductCache = Cache<i64, Product>;

#[handler]
async fn get_product(req: &mut Request, depot: &mut Depot, res: &mut Response) -> Result<Json<Product>, StatusError> {
    let id = req.param::<i64>("id").ok_or_else(|| StatusError::bad_request())?;
    let cache = depot.obtain::<ProductCache>().unwrap();

    if let Some(product) = cache.get(&id).await {
        res.headers_mut().insert("X-Cache", "HIT".parse().unwrap());
        res.headers_mut().insert("Cache-Control", "public, max-age=60".parse().unwrap());
        return Ok(Json(product));
    }

    let product = Product {
        id,
        name: format!("Product {}", id),
        price: 99.99,
    };

    cache.insert(id, product.clone()).await;

    res.headers_mut().insert("X-Cache", "MISS".parse().unwrap());
    res.headers_mut().insert("Cache-Control", "public, max-age=60".parse().unwrap());

    Ok(Json(product))
}

#[tokio::main]
async fn main() {
    let cache: ProductCache = Cache::builder()
        .max_capacity(10_000)
        .time_to_live(Duration::from_secs(300))
        .build();

    let router = Router::new()
        .hoop(affix_state::inject(cache))
        .push(Router::with_path("products/{id}").get(get_product));

    let acceptor = TcpListener::new("0.0.0.0:8080").bind().await;
    Server::new(acceptor).serve(router).await;
}
```

## Best Practices

1. Use appropriate TTL for data freshness requirements
2. Set cache headers for browser and CDN caching
3. Implement cache invalidation when data changes
4. Use ETags for conditional requests
5. Monitor cache hit rates
6. Consider cache warming for critical data

## Related Skills

- **salvo-database**: Cache database queries
- **salvo-compression**: Combine caching with compression
