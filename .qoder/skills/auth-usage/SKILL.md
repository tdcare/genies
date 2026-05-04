---
name: auth-usage
description: Guide for using the genies_auth permission and authentication module. Use when implementing API-level access control, field-level permission filtering, Casbin policy management, role assignment, resource grouping, JWT authentication middleware (local_auth, combined_auth), or when integrating auth middleware into Salvo routes. Also use when the user asks about #[casbin] macro, EnforcerManager, casbin_auth middleware, local_auth, combined_auth, LocalAuthConfig, event-driven sync, startup_sync, or auth Admin API endpoints.
---

# Auth Module (genies_auth)

## Overview

genies_auth 是基于 Casbin 的完整 RBAC 权限管理库，提供 JWT 认证 + API 接口级权限 + 字段级权限控制。采用黑名单 deny 模式（默认允许，仅 deny 规则生效）。纯库 crate，无 binary。

**核心特性：**
- **JWT 认证中间件**（`local_auth` / `combined_auth`，Token 由 auth-admin 服务签发）
- API 接口级访问控制（基于路径 + HTTP 方法）
- 字段级权限过滤（序列化时自动检查每个字段）
- 动态策略管理（14 个 Admin API 端点，支持 OpenAPI/SwaggerUI）
- OpenApi Schema 自动同步
- Redis 缓存 + 多实例版本同步
- **Dapr 事件驱动同步**（自动接收 auth-admin 的 casbin_rules 变更事件）
- **启动时用户-角色同步**（从 auth-admin 拉取 g 规则）
- Flyway 数据库迁移

> **注意：** 管理界面（Admin UI）已从 genies_auth 中移除，迁移到独立的 `genies_auth_admin` crate 提供。详见 auth-admin 的文档。

## Architecture

```
请求 → local_auth(JWT) 或 combined_auth(JWT+Casbin) → Handler → Writer(字段过滤) → 响应
```

**Three middleware modes:**

| Middleware | JWT Auth | Casbin API Check | Use Case |
|------------|----------|------------------|----------|
| `local_auth` | ✅ | ❌ | JWT-only auth, no API-level permission check |
| `casbin_auth` | ❌ (reads JWT from Depot) | ✅ | Casbin-only, requires separate JWT middleware (e.g. `salvo_auth`) |
| `combined_auth` | ✅ | ✅ | All-in-one: JWT + Casbin in a single middleware |

核心组件：
- `EnforcerManager` - Enforcer 管理器（RwLock<Arc<Enforcer>>），支持热更新
- `local_auth` - JWT 认证中间件（验证 Token、注入用户信息到 Depot）
- `combined_auth` - 组合中间件（JWT 认证 + Casbin API 权限检查）
- `casbin_auth` - Salvo 中间件，API 级权限检查 + Depot 注入
- `LocalAuthConfig` - 本地 JWT 配置（secret + expiry）
- `LocalClaims` - JWT Claims 结构体
- `verify_token` - JWT 验证函数
- `auth_admin_router` - 14 个 Admin API 端点（策略/角色/分组/模型 CRUD + reload + auth），支持 OpenAPI 元数据
- `auth_public_router` - 不需要认证的公共路由（如 `/auth/token`）
- `RBatisAdapter` - MySQL Casbin Adapter
- `extract_and_sync_schemas` - OpenApi Schema 自动同步
- `version_sync` - Enforcer 多实例版本同步
- `event_handler` - Dapr 事件订阅处理（接收 auth-admin 的同步事件）
- `startup_sync` - 启动时用户-角色同步
- `service_registry` - 微服务实例注册、心跳和注销客户端
- `models::run_migrations` - Flyway 数据库迁移

## Quick Start

### 1. Dependencies

```toml
[dependencies]
genies_auth = { workspace = true }
genies_derive = { workspace = true }  # for #[casbin] macro
casbin = { version = "2.10.1", features = ["runtime-tokio"] }
jsonwebtoken = "9"   # only if you need verify_token directly
```

### 2. Define struct with field-level permissions

```rust
use genies_derive::casbin;
use serde::Deserialize;
use salvo::oapi::ToSchema;

#[casbin]
#[derive(Deserialize, ToSchema)]
pub struct User {
    pub id: u64,
    pub name: Option<String>,
    pub email: String,       // sensitive
    pub phone: String,       // sensitive
}
```

`#[casbin]` 宏自动生成：
- 自定义 `Serialize`（Writer 层 JSON 树过滤）
- Salvo `Writer` trait（从 Depot 提取 enforcer/subject 并过滤字段）

**自动嵌套检测**：宏自动识别非原始类型字段（struct、`Option<T>`、`Vec<T>`）并递归过滤。无需 `#[casbin(nested)]` 标记。

### 3. Handler（零模板代码）

```rust
#[endpoint]
async fn get_user() -> Json<User> {
    Json(User {
        id: 1,
        name: Some("张三".into()),
        email: "zhangsan@example.com".into(),
        phone: "13800138000".into(),
    })
}
```

### ⚠️ RespVO 包装时的字段过滤限制

`#[casbin]` 宏生成的 `Writer` 实现**只在 handler 直接返回 `Json<T>` 或 `T` 时自动触发字段过滤**。当返回 `Json<RespVO<T>>` 时，`RespVO` 的 Writer 接管序列化，**不会**自动调用内层类型 `T` 的字段过滤。

**解决方案：** 在 handler 中手动从 Depot 获取 enforcer/subject，调用 `T::casbin_filter()` 过滤后再包装为 RespVO 返回。

```rust
use salvo::prelude::*;
use genies_auth::EnforcerManager;
use genies_core::RespVO;

#[endpoint]
async fn list_users(depot: &mut Depot) -> Json<RespVO<Vec<User>>> {
    let users = fetch_users().await;

    // 1. 从 Depot 获取 enforcer 和 subject
    let enforcer = depot.obtain::<std::sync::Arc<casbin::Enforcer>>().ok();
    let subject = depot.get::<String>("subject").ok();

    // 2. 序列化为 JSON Value，手动调用 casbin_filter
    let mut value = serde_json::to_value(&users).unwrap();
    if let (Some(e), Some(s)) = (&enforcer, &subject) {
        if let Some(arr) = value.as_array_mut() {
            for item in arr.iter_mut() {
                User::casbin_filter(item, e.as_ref(), s.as_str());
            }
        }
    }

    // 3. 反序列化回业务类型，包装为 RespVO
    let filtered: Vec<User> = serde_json::from_value(value).unwrap();
    Json(RespVO::from(&Ok::<_, String>(filtered)))
}
```

**参数类型转换注意事项：**
- `enforcer.as_ref()` — `Arc<Enforcer>` → `&Enforcer`
- `subject.as_str()` — `String` → `&str`

**类型注册要求：** 类型必须出现在 `#[endpoint]` handler 的返回类型中（如 `Json<RespVO<User>>`），这样 `extract_and_sync_schemas` 才会将其注册到 `auth_api_schemas` 表，用户才能在 `genies_auth_admin` 管理界面中看到并设置字段权限。如果类型只在内部使用但不出现在返回类型中，schema_extractor 无法发现它。

### 4. Initialize and mount

#### Option A: `combined_auth` (Recommended — JWT + Casbin all-in-one)

```rust
use std::sync::Arc;
use genies::context::CONTEXT;
use genies_auth::{
    EnforcerManager, LocalAuthConfig, combined_auth,
    auth_admin_router, auth_public_router, extract_and_sync_schemas,
};

// Init DB + migrations
CONTEXT.init_mysql().await;
genies_auth::models::run_migrations().await;

// Startup sync (optional: pull g rules from auth-admin)
genies_auth::startup_sync::try_sync_on_startup(&config).await;

// Build router
let router = Router::new()
    .push(Router::with_path("/api/users").get(get_user));

// Schema sync (optional)
let doc = OpenApi::new("my-service", "1.0.0").merge_router(&router);
extract_and_sync_schemas(&doc).await.ok();

// Create config + Enforcer
let auth_config = Arc::new(LocalAuthConfig::new("my-jwt-secret"));
let mgr = Arc::new(EnforcerManager::new().await.unwrap());

let router = router
    .hoop(affix_state::inject(auth_config.clone()))  // inject JWT config
    .hoop(affix_state::inject(mgr.clone()))           // inject EnforcerManager
    .hoop(combined_auth)                              // JWT + Casbin in one
    .push(auth_admin_router());                       // Admin API (protected)

// Public routes (no auth required)
let router = Router::new()
    .push(auth_public_router())  // e.g. /auth/token
    .push(router);
```

#### Option B: `local_auth` only (JWT authentication, no Casbin)

```rust
use std::sync::Arc;
use genies_auth::{LocalAuthConfig, local_auth};

let auth_config = Arc::new(LocalAuthConfig::new("my-jwt-secret"));

let router = Router::new()
    .hoop(affix_state::inject(auth_config.clone()))
    .hoop(local_auth)           // JWT only, no Casbin check
    .push(Router::with_path("/api/users").get(get_user));
```

#### Option C: `salvo_auth` + `casbin_auth` (Keycloak JWT + Casbin)

```rust
use std::sync::Arc;
use genies::context::CONTEXT;
use genies_auth::{EnforcerManager, casbin_auth, auth_admin_router, extract_and_sync_schemas};

let mgr = Arc::new(EnforcerManager::new().await.unwrap());
let router = router
    .hoop(genies::context::auth::salvo_auth)       // Keycloak JWT
    .hoop(affix_state::inject(mgr.clone()))         // inject EnforcerManager
    .hoop(casbin_auth)                              // Casbin check
    .push(auth_admin_router());                     // Admin API
```

**中间件顺序必须是**: inject config/mgr → auth middleware

## JWT Authentication Middleware

### LocalAuthConfig

JWT signing/verification configuration, injected via `affix_state::inject`.

```rust
use genies_auth::LocalAuthConfig;

// Default: secret + 2h expiry
let config = LocalAuthConfig::new("my-jwt-secret");

// Custom expiry (seconds)
let config = LocalAuthConfig::with_expiry("my-jwt-secret", 3600);
```

### LocalClaims

JWT token payload structure:

```rust
pub struct LocalClaims {
    pub sub: String,           // username
    pub uid: Option<i64>,      // user ID
    pub name: Option<String>,  // display name
    pub iat: usize,            // issued at (UTC seconds)
    pub exp: usize,            // expiration (UTC seconds)
}
```

### local_auth middleware

JWT-only authentication. Verifies the `Authorization: Bearer <token>` header and injects user info into Depot.

**Depot injections:**
- `"jwtToken"` → `genies::core::jwt::JWTToken` (compatible with `casbin_auth`)
- `"local_user"` → `LocalClaims`
- `"subject"` → username `String`

**Requirements:**
- `Arc<LocalAuthConfig>` must be injected into Depot via `affix_state::inject`

**Error responses:**
- `401 Unauthorized` — missing or invalid token
- `500 Internal Server Error` — `LocalAuthConfig` not injected

### combined_auth middleware

All-in-one middleware: JWT authentication + Casbin API permission check.

**Requirements:**
- `Arc<LocalAuthConfig>` must be injected into Depot
- `Arc<EnforcerManager>` must be injected into Depot

**Behavior:**
1. Verify JWT token (same as `local_auth`)
2. Extract subject from JWT claims
3. Run Casbin `enforce(subject, path, method)`
4. If denied → `403 Forbidden`
5. If EnforcerManager not injected → skip Casbin check (graceful degradation)

### verify_token function

Standalone JWT verification (for non-middleware use cases):

```rust
use genies_auth::verify_token;

let claims = verify_token("Bearer eyJ...", "my-jwt-secret")?;
println!("User: {}", claims.sub);
```

## Event-Driven Sync (Dapr)

genies_auth subscribes to domain events published by auth-admin via Dapr pub/sub (`messagebus`). Events auto-update local `casbin_rules` table.

### Event Topics

| Topic | Action | casbin_rules Effect |
|-------|--------|---------------------|
| `auth.user.created` | Log only | — |
| `auth.user.updated` | Log only | — |
| `auth.user.deleted` | Delete g rules | `DELETE WHERE ptype='g' AND v0=username` |
| `auth.role.created` | Log only | — |
| `auth.role.updated` | Log only | — |
| `auth.role.deleted` | Delete g+p rules | `DELETE WHERE ptype='g' AND v1=role` + `DELETE WHERE ptype='p' AND v0=role` |
| `auth.permission.created` | Log only | — |
| `auth.permission.updated` | Log only | — |
| `auth.permission.deleted` | Delete p rules | `DELETE WHERE ptype='p' AND v1=resource AND v2=action` |
| `auth.user_role.assigned` | Insert g rule | `INSERT (ptype='g', v0=username, v1=role)` |
| `auth.user_role.revoked` | Delete g rule | `DELETE WHERE ptype='g' AND v0=username AND v1=role` |
| `auth.role_permission.assigned` | Insert p rule | `INSERT (ptype='p', v0=role, v1=resource, v2=action, v3='allow')` |
| `auth.role_permission.revoked` | Delete p rule | `DELETE WHERE ptype='p' AND v0=role AND v1=resource AND v2=action` |

### Event types (re-exported from `genies_auth`)

All events implement `DomainEvent` trait via `#[derive(DomainEvent)]`:

```rust
use genies_auth::{
    UserCreatedEvent, UserUpdatedEvent, UserDeletedEvent,
    RoleCreatedEvent, RoleUpdatedEvent, RoleDeletedEvent,
    PermissionCreatedEvent, PermissionUpdatedEvent, PermissionDeletedEvent,
    UserRoleAssignedEvent, UserRoleRevokedEvent,
    RolePermissionAssignedEvent, RolePermissionRevokedEvent,
};
```

## Startup Sync

On startup, if `auth_mode == "local"` and `auth_admin_url` is configured, the module pulls user-role mappings from auth-admin and replaces all local `g` rules.

```rust
use genies_auth::startup_sync::try_sync_on_startup;

// Call after DB migrations, before EnforcerManager::new()
try_sync_on_startup(&config).await;
```

**Configuration required in `ApplicationConfig`:**
- `auth_mode: "local"` — enables local JWT mode
- `auth_admin_url: "http://auth-admin:8080"` — auth-admin service URL
- `jwt_secret: "shared-secret"` — shared JWT secret for service-to-service calls

**Sync flow:**
1. Generate short-lived service JWT (60s expiry, sub=`"auth-service"`)
2. `GET {auth_admin_url}/auth-admin/sync/user-roles` with Bearer token
3. Replace all `g` rules in `casbin_rules` (transaction: DELETE all → INSERT new)
4. Failure is non-fatal (warns and continues with existing local rules)

## Service Registry (Instance Registration & Heartbeat)

`service_registry` 模块实现微服务启动时自动向 auth-admin 注册实例，后台定期发送心跳，进程退出时注销。

**模块文件**：`crates/auth/src/service_registry.rs`

### 核心 API

```rust
use genies_auth::service_registry::{try_register_and_heartbeat, ServiceRegistryGuard};

// 一站式入口：注册 + 启动心跳 + 返回 guard
// 条件：auth_admin_url 非空时执行
let _guard = genies_auth::try_register_and_heartbeat(&CONTEXT.config).await;
```

### 配置要求（ApplicationConfig）

- `auth_admin_url` — auth-admin 服务地址（非空时启用注册）
- `jwt_secret` — 服务间 JWT 认证密钥
- `server_name` — 用作 app_name
- `server_url` — 构建 base_url
- `heartbeat_interval` — 心跳间隔秒数（默认 30）

### 工作流程

1. 使用雪花ID生成器创建唯一 `instance_id`
2. POST `{auth_admin_url}/auth-admin/internal/instances/register` 注册实例
3. 后台 `tokio::spawn` 每 `heartbeat_interval` 秒 POST heartbeat
4. `ServiceRegistryGuard` 在 Drop 时 best-effort 发送 deregister 请求并 abort 心跳任务
5. 所有网络失败仅 warn 日志，不中断主流程

### 使用示例（main.rs）

```rust
// 在 CONTEXT.init_database() 之后, Server::new().serve() 之前
let _registry_guard = genies_auth::try_register_and_heartbeat(&CONTEXT.config).await;
```

### 内部函数

- `generate_instance_id() -> i64` — 雪花ID
- `register_instance()` — 注册
- `send_heartbeat()` — 心跳
- `deregister_instance()` — 注销
- `start_heartbeat_loop()` — 后台心跳循环

## Admin API Reference

所有端点均支持 OpenAPI 元数据（tags, summary, description），路径参数使用 `PathParam<i64>`，请求体使用 `JsonBody<T>`。

| Endpoint | Method | Tag | Description |
|----------|--------|-----|-------------|
| `/auth/schemas` | GET | schemas | List all API schemas |
| `/auth/model` | GET | model | Get Casbin model |
| `/auth/model` | PUT | model | Update model (JsonBody<ModelDto>) |
| `/auth/policies` | GET | policies | List all policies |
| `/auth/policies` | POST | policies | Add policy (JsonBody<PolicyDto>) |
| `/auth/policies/{id}` | DELETE | policies | Delete policy (PathParam<i64>) |
| `/auth/roles` | GET | roles | List role mappings (g) |
| `/auth/roles` | POST | roles | Add role mapping (JsonBody<PolicyDto>) |
| `/auth/roles/{id}` | DELETE | roles | Delete role mapping (PathParam<i64>) |
| `/auth/groups` | GET | groups | List groups (g2) |
| `/auth/groups` | POST | groups | Add group (JsonBody<PolicyDto>) |
| `/auth/groups/{id}` | DELETE | groups | Delete group (PathParam<i64>) |
| `/auth/reload` | POST | system | Reload enforcer |
| `/auth/check` | POST | auth | Check permission |

All mutations auto-trigger `mgr.reload()` + `version_sync::invalidate_and_reload()`.

### SwaggerUI 集成

integration 示例服务器支持 SwaggerUI，启动后访问：
- **SwaggerUI**: `http://localhost:<port>/swagger-ui/`
- **OpenAPI JSON**: `http://localhost:<port>/api-doc/openapi.json`

## Policy Configuration

### Policy types

- `p` - policy rule: `(subject, object, action, effect)`
- `g` - role mapping: `(user, role)`
- `g2` - resource grouping: `(resource, group)`

### Examples

```sql
-- API deny: guest can't access /api/admin
INSERT INTO casbin_rules (ptype,v0,v1,v2,v3) VALUES ('p','guest','/api/admin','get','deny');

-- Field deny: bob can't see User.email
INSERT INTO casbin_rules (ptype,v0,v1,v2,v3) VALUES ('p','bob','User.email','read','deny');

-- Role: alice is admin
INSERT INTO casbin_rules (ptype,v0,v1) VALUES ('g','alice','admin');

-- Group: /api/users belongs to user:manage
INSERT INTO casbin_rules (ptype,v0,v1) VALUES ('g2','/api/users','user:manage');
```

## Database Tables (auto-created by Flyway)

- `casbin_rules` - ptype, v0-v5, id, created_at
- `casbin_model` - model_name (unique), model_text, description
- `auth_api_schemas` - schema_name, field_name, field_type, endpoint_path, http_method

## Default Casbin Model

Uses deny-override: `e = !some(where (p.eft == deny))`

Supports role inheritance (g) and resource grouping (g2) with keyMatch2 path matching.

**Model 定义：**
```
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

For detailed model definition and advanced usage, see [reference.md](reference.md)

## Testing

集成测试位于 `examples/integration/src/auth_tests.rs`，61 个端到端测试。

运行: `cargo test -p integration auth_tests -- --nocapture --test-threads=1`

测试使用独立线程启动内嵌 Auth 服务器（动态端口），通过 reqwest 发送 HTTP 请求验证。

## Key Files

- [crates/auth/src/lib.rs](file:///d:/tdcare/genies/crates/auth/src/lib.rs) - 模块入口 + re-exports
- [crates/auth/src/auth_middleware.rs](file:///d:/tdcare/genies/crates/auth/src/auth_middleware.rs) - JWT 中间件 (local_auth, combined_auth, verify_token, LocalAuthConfig, LocalClaims)
- [crates/auth/src/middleware.rs](file:///d:/tdcare/genies/crates/auth/src/middleware.rs) - casbin_auth 中间件 + casbin_filter_object
- [crates/auth/src/enforcer_manager.rs](file:///d:/tdcare/genies/crates/auth/src/enforcer_manager.rs) - EnforcerManager
- [crates/auth/src/admin_api.rs](file:///d:/tdcare/genies/crates/auth/src/admin_api.rs) - Admin API 端点 + auth_public_router
- [crates/auth/src/event.rs](file:///d:/tdcare/genies/crates/auth/src/event.rs) - 领域事件定义 (13 events)
- [crates/auth/src/event_handler.rs](file:///d:/tdcare/genies/crates/auth/src/event_handler.rs) - Dapr 事件订阅处理
- [crates/auth/src/startup_sync.rs](file:///d:/tdcare/genies/crates/auth/src/startup_sync.rs) - 启动时用户-角色同步
- [crates/auth/src/version_sync.rs](file:///d:/tdcare/genies/crates/auth/src/version_sync.rs) - Enforcer 多实例版本同步
- [crates/auth/src/schema_extractor.rs](file:///d:/tdcare/genies/crates/auth/src/schema_extractor.rs) - Schema 提取
- [crates/auth/src/service_registry.rs](file:///d:/tdcare/genies/crates/auth/src/service_registry.rs) - 服务注册/心跳/注销客户端
- [crates/genies_derive/src/casbin.rs](file:///d:/tdcare/genies/crates/genies_derive/src/casbin.rs) - #[casbin] 宏实现
