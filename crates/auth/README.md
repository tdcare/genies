# genies_auth

A comprehensive RBAC permission management module for the Genies framework, powered by [Casbin](https://casbin.org/).

## Overview

genies_auth provides a complete role-based access control (RBAC) solution with both API-level and field-level permission control. It integrates seamlessly with Salvo web framework and supports dynamic policy updates without service restart.

## Features

- **Hybrid Permission Model**: API endpoint-level + field-level access control
- **Deny-Mode Blacklist**: Default allow, only deny rules take effect (`e = !some(where (p.eft == deny))`)
- **Dynamic Hot Reload**: Modify permissions at runtime via Admin API, no restart required
- **Multi-Instance Sync**: Redis cache version numbers for distributed cache invalidation
- **Auto Schema Sync**: Automatically extract schemas from OpenAPI docs to database
- **Flyway Migrations**: Auto-create required database tables

## Architecture

### Core Components

| Component | File | Description |
|-----------|------|-------------|
| `EnforcerManager` | enforcer_manager.rs | Casbin Enforcer manager with hot reload, `RwLock<Arc<Enforcer>>` for concurrency safety |
| `casbin_auth` | middleware.rs | Salvo middleware for JWT auth + Casbin permission check, injects enforcer/subject into Depot |
| `auth_admin_router` | admin_api.rs | Admin API router (12 endpoints for policy/role/group/model CRUD + reload) |
| `RBatisAdapter` | adapter.rs | Casbin Adapter implementation backed by MySQL |
| `extract_and_sync_schemas` | schema_extractor.rs | Extract schemas from OpenAPI docs and sync to database |
| `cache` | cache.rs | Redis cache layer for policies/schemas + version sync |
| `models` | models.rs | Database models + Flyway migrations (`run_migrations`) |

### Middleware Flow

```
Request → salvo_auth(JWT) → casbin_auth(Permission) → Handler → Writer(Field Filter) → Response
```

## Quick Start

### 1. Add Dependency

```toml
[dependencies]
genies_auth = { path = "../path/to/genies_auth" }
genies = { path = "../path/to/genies" }
genies_derive = { path = "../path/to/genies_derive" }
```

### 2. Define Structs with Field-Level Permissions

```rust
use genies_derive::casbin;
use salvo::oapi::ToSchema;

#[casbin]
#[derive(serde::Deserialize, ToSchema)]
pub struct User {
    pub id: u64,
    pub name: Option<String>,
    pub email: String,       // Sensitive field
    pub phone: String,       // Sensitive field
}
```

The `#[casbin]` macro automatically generates:
- `enforcer` and `subject` fields (`#[serde(skip)]`)
- `with_policy()` method
- `check_permission(field_name)` method
- Custom `Serialize` implementation (checks permission per field during serialization)
- Salvo `Writer` trait implementation (auto-injects enforcer/subject from Depot)

### 3. Initialize and Start Server

```rust
use std::sync::Arc;
use salvo::prelude::*;
use genies::context::CONTEXT;
use genies_auth::{EnforcerManager, casbin_auth, auth_admin_router, extract_and_sync_schemas};

#[endpoint]
async fn get_user() -> Json<User> {
    Json(User {
        id: 1,
        name: Some("John".into()),
        email: "john@example.com".into(),
        phone: "13800138000".into(),
        enforcer: None,
        subject: None,
    })
}

#[tokio::main]
async fn main() {
    // Initialize database
    CONTEXT.init_mysql().await;
    genies_auth::models::run_migrations().await;
    
    // Build router
    let router = Router::new()
        .push(Router::with_path("/api/users").get(get_user));
    
    // Sync OpenAPI schemas to database
    let doc = OpenApi::new("my-service", "1.0.0").merge_router(&router);
    extract_and_sync_schemas(&doc).await.ok();
    
    // Initialize EnforcerManager
    let mgr = Arc::new(EnforcerManager::new().await.unwrap());
    
    // Apply middleware
    let router = router
        .hoop(genies::context::auth::salvo_auth)
        .hoop(affix_state::inject(mgr.clone()))
        .hoop(casbin_auth)
        .push(auth_admin_router());
    
    let acceptor = TcpListener::new("0.0.0.0:5800").bind().await;
    Server::new(acceptor).serve(router).await;
}
```

## Admin API Reference

| Endpoint | Method | Description |
|----------|--------|-------------|
| `/auth/schemas` | GET | List all API schemas |
| `/auth/model` | GET | Get Casbin model definition |
| `/auth/model` | PUT | Update Casbin model |
| `/auth/policies` | GET | List all policies |
| `/auth/policies` | POST | Add policy |
| `/auth/policies/<id>` | DELETE | Delete policy |
| `/auth/roles` | GET | List role mappings (g) |
| `/auth/roles` | POST | Add role mapping |
| `/auth/roles/<id>` | DELETE | Delete role mapping |
| `/auth/groups` | GET | List groups (g2) |
| `/auth/groups` | POST | Add group |
| `/auth/groups/<id>` | DELETE | Delete group |
| `/auth/reload` | POST | Manually reload Enforcer |

## Database Tables

Tables are auto-created via Flyway migrations:

| Table | Description |
|-------|-------------|
| `casbin_rules` | Policy rules storage (ptype, v0-v5) |
| `casbin_model` | Casbin model definition storage |
| `auth_api_schemas` | API schema metadata storage |

## Configuration

### Policy Examples

```sql
-- API-level: deny guest access to /api/admin
INSERT INTO casbin_rules (ptype, v0, v1, v2, v3) VALUES ('p', 'guest', '/api/admin', 'get', 'deny');

-- Field-level: deny bob from reading email field
INSERT INTO casbin_rules (ptype, v0, v1, v2, v3) VALUES ('p', 'bob', 'User.email', 'read', 'deny');

-- Role assignment: alice is admin
INSERT INTO casbin_rules (ptype, v0, v1) VALUES ('g', 'alice', 'admin');

-- Resource grouping
INSERT INTO casbin_rules (ptype, v0, v1) VALUES ('g2', '/api/users', 'user:manage');
```

### Default Casbin Model

```ini
[request_definition]
r = sub, obj, act

[policy_definition]
p = sub, obj, act, eft

[role_definition]
g = _, _
g2 = _, _

[policy_effect]
e = !some(where (p.eft == deny))

[matchers]
m = (g(r.sub, p.sub) || r.sub == p.sub) && (g2(r.obj, p.obj) || r.obj == p.obj || keyMatch2(r.obj, p.obj)) && r.act == p.act
```

## Dependencies

- **casbin** 2.10.1 - Authorization library
- **salvo** - Web framework
- **rbatis** - ORM framework
- **flyway** - Database migrations
- **tokio** - Async runtime

## Testing

13 end-to-end tests covering: Schema sync, model management, policy CRUD, hot reload, role assignment, group management, 403 rejection, field filtering, Redis cache, concurrency safety, etc.

```bash
cargo test -p integration auth_tests -- --nocapture --test-threads=1
```

## License

See the project root for license information.
