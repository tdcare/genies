---
name: auth-usage
description: Guide for using the genies_auth Casbin-based permission management module. Use when implementing API-level access control, field-level permission filtering, Casbin policy management, role assignment, resource grouping, or when integrating auth middleware into Salvo routes. Also use when the user asks about #[casbin] macro, EnforcerManager, casbin_auth middleware, or auth Admin API endpoints.
---

# Auth Module (genies_auth)

## Overview

genies_auth 是基于 Casbin 的完整 RBAC 权限管理库，提供 API 接口级权限 + 字段级权限控制。采用黑名单 deny 模式（默认允许，仅 deny 规则生效）。纯库 crate，无 binary。

**核心特性：**
- API 接口级访问控制（基于路径 + HTTP 方法）
- 字段级权限过滤（序列化时自动检查每个字段）
- 动态策略管理（14 个 Admin API 端点，支持 OpenAPI/SwaggerUI）
- OpenApi Schema 自动同步
- Redis 缓存 + 多实例版本同步
- Flyway 数据库迁移

## Architecture

```
请求 → salvo_auth(JWT) → casbin_auth(权限检查) → Handler → Writer(字段过滤) → 响应
```

核心组件：
- `EnforcerManager` - Enforcer 管理器（RwLock<Arc<Enforcer>>），支持热更新
- `casbin_auth` - Salvo 中间件，API 级权限检查 + Depot 注入
- `auth_admin_router` - 14 个 Admin API 端点（策略/角色/分组/模型 CRUD + reload + auth），支持 OpenAPI 元数据
- `RBatisAdapter` - MySQL Casbin Adapter
- `extract_and_sync_schemas` - OpenApi Schema 自动同步
- `version_sync` - Enforcer 多实例版本同步
- `models::run_migrations` - Flyway 数据库迁移

## Quick Start

### 1. Dependencies

```toml
[dependencies]
genies_auth = { workspace = true }
genies_derive = { workspace = true }  # for #[casbin] macro
casbin = { version = "2.10.1", features = ["runtime-tokio"] }
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

### 4. Initialize and mount

```rust
use std::sync::Arc;
use genies::context::CONTEXT;
use genies_auth::{EnforcerManager, casbin_auth, auth_admin_router, extract_and_sync_schemas};

// Init DB + migrations
CONTEXT.init_mysql().await;
genies_auth::models::run_migrations().await;

// Build router
let router = Router::new()
    .push(Router::with_path("/api/users").get(get_user));

// Schema sync (optional)
let doc = OpenApi::new("my-service", "1.0.0").merge_router(&router);
extract_and_sync_schemas(&doc).await.ok();

// Create Enforcer + mount middleware
let mgr = Arc::new(EnforcerManager::new().await.unwrap());
let router = router
    .hoop(genies::context::auth::salvo_auth)       // JWT
    .hoop(affix_state::inject(mgr.clone()))         // inject EnforcerManager
    .hoop(casbin_auth)                          // Casbin check
    .push(auth_admin_router());                     // Admin API
```

**中间件顺序必须是**: `salvo_auth` → `inject(mgr)` → `casbin_auth`

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

- [crates/auth/src/lib.rs](file:///d:/tdcare/genies/crates/auth/src/lib.rs) - 模块入口
- [crates/auth/src/middleware.rs](file:///d:/tdcare/genies/crates/auth/src/middleware.rs) - casbin_auth 中间件
- [crates/auth/src/enforcer_manager.rs](file:///d:/tdcare/genies/crates/auth/src/enforcer_manager.rs) - EnforcerManager
- [crates/auth/src/admin_api.rs](file:///d:/tdcare/genies/crates/auth/src/admin_api.rs) - Admin API 端点
- [crates/auth/src/version_sync.rs](file:///d:/tdcare/genies/crates/auth/src/version_sync.rs) - Enforcer 多实例版本同步
- [crates/auth/src/schema_extractor.rs](file:///d:/tdcare/genies/crates/auth/src/schema_extractor.rs) - Schema 提取
- [crates/genies_derive/src/casbin.rs](file:///d:/tdcare/genies/crates/genies_derive/src/casbin.rs) - #[casbin] 宏实现
