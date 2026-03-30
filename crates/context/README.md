# genies_context

Application context management library for the Genies framework, providing global context, database connections, cache services, and cross-service authentication.

## Overview

genies_context provides centralized management of application runtime context including database connection pools, cache services, Keycloak authentication keys, and cross-service token management. It uses the `lazy_static` pattern to provide global singleton access throughout the application.

## Features

- **Global Context Singleton**: `CONTEXT` provides access to config, database, and cache
- **Database Connection Pool**: Async MySQL connection pool via RBatis
- **Cache Services**: Redis-backed cache with separate data and save channels
- **Keycloak Integration**: JWT key retrieval and token verification
- **Cross-Service Token**: `REMOTE_TOKEN` for service-to-service authentication
- **K8s Health Status**: `SERVICE_STATUS` for readiness/liveness probes
- **Salvo Auth Middleware**: `salvo_auth` for JWT authentication

## Architecture

### Core Components

| Component | File | Description |
|-----------|------|-------------|
| `ApplicationContext` | app_context.rs | Main context struct with config, rbatis, cache services |
| `CONTEXT` | lib.rs | Global singleton via `lazy_static` |
| `REMOTE_TOKEN` | lib.rs | Cross-service token storage (`Mutex<RemoteToken>`) |
| `SERVICE_STATUS` | lib.rs | K8s probe status (`Mutex<HashMap>`) |
| `init_mysql` | app_context.rs | Async database pool initialization |
| `RemoteToken` | app_context.rs | Service-to-service authentication token |
| `salvo_auth` | auth.rs | Salvo JWT authentication middleware |
| `checked_token` | auth.rs | Token verification function |
| `is_white_list_api` | auth.rs | API whitelist checking |

### Initialization Flow

```
Application Start → CONTEXT (lazy_static) → init_mysql() → Ready
                         │
                         ├── ApplicationConfig (./application.yml)
                         ├── Keycloak Keys (async fetch)
                         ├── CacheService (Redis)
                         └── RBatis (MySQL pool)
```

## Quick Start

### 1. Add Dependency

```toml
[dependencies]
genies_context = { path = "../path/to/genies_context" }
genies_config = { path = "../path/to/genies_config" }
genies_cache = { path = "../path/to/genies_cache" }
genies_core = { path = "../path/to/genies_core" }
rbatis = "4.x"
```

### 2. Initialize Database

```rust
use genies::context::CONTEXT;

#[tokio::main]
async fn main() {
    // Initialize MySQL connection pool (thread-safe, only executes once)
    CONTEXT.init_mysql().await;
    
    // Now ready to use CONTEXT.rbatis
    println!("Database connected: {:?}", CONTEXT.rbatis.get_pool().unwrap().state().await);
}
```

### 3. Use Database Connection

```rust
use genies::context::CONTEXT;
use rbatis::executor::Executor;

pub async fn query_users() -> Vec<User> {
    let rb = &CONTEXT.rbatis;
    User::select_all(rb).await.unwrap()
}

// With transaction
pub async fn create_user(user: &User) {
    let mut tx = CONTEXT.rbatis.acquire_begin().await.unwrap();
    User::insert(&mut tx, user).await.unwrap();
    tx.commit().await.unwrap();
}
```

### 4. Use Cache Service

```rust
use genies::context::CONTEXT;

pub async fn cache_example() {
    // Use cache_service (standard cache)
    CONTEXT.cache_service.set_string("key", "value").await.unwrap();
    let value = CONTEXT.cache_service.get_string("key").await.unwrap();
    
    // Use redis_save_service (persistent cache)
    CONTEXT.redis_save_service.set_string("persistent_key", "data").await.unwrap();
}
```

### 5. Configure Salvo Auth Middleware

```rust
use genies::context::auth::salvo_auth;
use salvo::prelude::*;

let router = Router::new()
    .hoop(salvo_auth)  // JWT authentication middleware
    .push(Router::with_path("/api/users").get(get_users));
```

## API Reference

### ApplicationContext Struct

```rust
pub struct ApplicationContext {
    /// Application configuration from ./application.yml
    pub config: ApplicationConfig,
    
    /// RBatis database connection pool
    pub rbatis: RBatis,
    
    /// Standard cache service (Redis)
    pub cache_service: CacheService,
    
    /// Persistent cache service (Redis)
    pub redis_save_service: CacheService,
    
    /// Keycloak JWT verification keys
    pub keycloak_keys: Keys,
}

impl ApplicationContext {
    /// Initialize MySQL connection pool (thread-safe, idempotent)
    pub async fn init_mysql(&self);
    
    /// Create new ApplicationContext (reads ./application.yml)
    pub fn new() -> Self;
}
```

### Global Singletons

```rust
lazy_static! {
    /// Global application context (database, cache, config)
    pub static ref CONTEXT: ApplicationContext = ApplicationContext::default();
    
    /// Cross-service access token storage
    pub static ref REMOTE_TOKEN: Mutex<RemoteToken> = Mutex::new(RemoteToken::new());
    
    /// K8s service health status
    pub static ref SERVICE_STATUS: Mutex<HashMap<String, bool>> = Mutex::new({
        let mut map = HashMap::new();
        map.insert("readinessProbe".to_string(), true);
        map.insert("livenessProbe".to_string(), true);
        map
    });
}
```

### RemoteToken Struct

```rust
#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct RemoteToken {
    pub access_token: String,
}

impl RemoteToken {
    /// Create new RemoteToken by fetching from Keycloak
    pub fn new() -> Self;
}
```

### Auth Functions

```rust
/// Check if path is in API whitelist
pub fn is_white_list_api(context: &ApplicationContext, path: &str) -> bool;

/// Verify token and return JWTToken
pub async fn checked_token(
    context: &ApplicationContext,
    token: &str,
    path: &str,
) -> Result<JWTToken, Error>;

/// Check authorization (currently returns Ok)
pub async fn check_auth(
    context: &ApplicationContext,
    token: &JWTToken,
    path: &str,
) -> Result<(), Error>;

/// Salvo JWT authentication middleware
#[handler]
pub async fn salvo_auth(
    req: &mut Request,
    depot: &mut Depot,
    res: &mut Response,
    ctrl: &mut FlowCtrl,
);
```

## Configuration

### application.yml Structure

```yaml
server_url: "0.0.0.0:5800"
database_url: "mysql://user:pass@localhost:3306/db"
max_connections: 10
wait_timeout: 30
max_lifetime: 3600

# Redis
redis_host: "localhost"
redis_port: 6379

# Keycloak
keycloak_auth_server_url: "http://localhost:8080"
keycloak_realm: "myrealm"
keycloak_resource: "myapp"
keycloak_credentials_secret: "secret"

# Whitelist APIs (skip auth)
white_list_api:
  - "/health"
  - "/dapr/*"
  - "/swagger-ui/*"
```

## Auth Middleware Flow

```
Request → salvo_auth
    │
    ├── is_white_list_api? → Yes → Continue (skip auth)
    │
    └── No → checked_token()
               │
               ├── Valid → check_auth() → depot.insert("jwtToken", token)
               │                              → Continue
               │
               └── Invalid → 401 Unauthorized
```

## K8s Health Status

```rust
use genies::context::SERVICE_STATUS;

// Update readiness status
{
    let mut status = SERVICE_STATUS.lock().unwrap();
    status.insert("readinessProbe".to_string(), false);
}

// Check liveness status
{
    let status = SERVICE_STATUS.lock().unwrap();
    let is_alive = *status.get("livenessProbe").unwrap_or(&false);
}
```

## Dependencies

- **genies_config** - Application configuration
- **genies_cache** - Cache service abstraction
- **genies_core** - JWT utilities, error types
- **rbatis** - ORM framework
- **rbdc-mysql** - MySQL driver
- **lazy_static** - Global singleton pattern
- **tokio** - Async runtime
- **salvo** - Web framework (for auth middleware)

## Integration with Other Crates

- **genies_auth**: Uses `CONTEXT.rbatis` for policy storage, `salvo_auth` for JWT verification
- **genies_ddd**: Uses `CONTEXT.rbatis` for event publishing
- **genies_dapr**: Uses `CONTEXT.rbatis` for transaction management
- **genies_config**: Provides `ApplicationConfig` for context initialization

## Thread Safety

- `CONTEXT`: `lazy_static` ensures single initialization, fields are thread-safe
- `init_mysql()`: Uses `Once` for idempotent initialization
- `REMOTE_TOKEN`: `Mutex<RemoteToken>` for thread-safe access
- `SERVICE_STATUS`: `Mutex<HashMap>` for thread-safe status updates

## License

See the project root for license information.
