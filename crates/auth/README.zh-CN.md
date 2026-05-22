# genies_auth

基于 [Casbin](https://casbin.org/) 的 Genies (神灯) 框架权限管理模块，提供完整的 RBAC 权限解决方案。

## 概述

genies_auth 提供完整的基于角色的访问控制（RBAC）解决方案，支持 API 接口级和字段级权限控制。它与 Salvo Web 框架无缝集成，支持运行时动态更新权限策略，无需重启服务。

> **说明**：Web 管理界面已迁移至独立的 [auth-admin](../auth-admin/) 服务。本 crate 仅专注于运行时权限执行。

## 核心特性

- **混合权限模型**：API 接口级权限 + 字段级权限控制
- **OAuth 2.0 资源服务器**：验证 auth-admin 签发的 OAuth 令牌，支持 JWT 本地验证（快速）和 introspect 远程验证（opaque 令牌回退）
- **黑名单 deny 模式**：默认允许，仅 deny 规则生效（`e = !some(where (p.eft == deny))`）
- **动态热更新**：通过 Admin API 实时修改权限，无需重启
- **多实例同步**：Redis 缓存版本号实现分布式缓存失效
- **自动 Schema 同步**：从 OpenAPI 文档自动提取 Schema 到数据库
- **Flyway 数据库迁移**：自动创建所需表结构

## 架构设计

### 核心组件

| 组件 | 文件 | 功能 |
|------|------|------|
| `EnforcerManager` | enforcer_manager.rs | Casbin Enforcer 管理器，支持热更新，`RwLock<Arc<Enforcer>>` 并发安全 |
| `local_auth` / `combined_auth` | auth_middleware.rs | JWT 认证中间件，验证 auth-admin 签发的本地 JWT 令牌 |
| `oauth2_auth` / `combined_oauth2_auth` | oauth2_middleware.rs | OAuth 2.0 资源服务器中间件 — 通过 JWT（快速）或 introspect（回退）验证 OAuth 令牌 |
| `casbin_auth` | middleware.rs | Salvo 中间件，Casbin 权限检查，注入 enforcer/subject 到 Depot |
| `auth_router` | admin_api.rs | Admin API 路由（14 个端点，带 OpenAPI 注解，策略/角色/分组/模型 CRUD + reload） |
| `RBatisAdapter` | adapter.rs | Casbin Adapter 实现，对接 MySQL 存储 |
| `extract_and_sync_schemas` | schema_extractor.rs | 从 OpenAPI 文档提取 Schema 并同步到数据库 |
| `version_sync` | version_sync.rs | Enforcer 多实例版本同步，基于 Redis（`invalidate_and_reload()`、`get_enforcer_version()`） |
| `models` | models.rs | 数据库模型 + Flyway 迁移（`run_migrations`） |

### 中间件执行流程

```
# 本地 JWT 认证（内部服务默认方案）
请求 → local_auth(JWT验证) → casbin_auth(权限检查) → 业务Handler → Writer(字段过滤) → 响应

# OAuth 2.0 资源服务器（第三方 API 访问）
请求 → oauth2_auth(OAuth令牌验证) → casbin_auth(权限检查) → 业务Handler → Writer(字段过滤) → 响应

# 组合中间件（认证+授权一步完成）
请求 → combined_auth(JWT + Casbin) → 业务Handler → Writer(字段过滤) → 响应
     → combined_oauth2_auth(OAuth + Casbin) → 业务Handler → Writer(字段过滤) → 响应
```

## 认证中间件

genies_auth 提供两种认证策略，每种都有独立版和组合版（认证 + Casbin 权限检查）：

| 中间件 | 认证方式 | Casbin 检查 | 适用场景 |
|--------|---------|------------|---------|
| `local_auth` | JWT（与 auth-admin 共享密钥） | 否 | 内部服务间调用 |
| `combined_auth` | JWT（共享密钥） | 是 | 需要权限控制的内部服务 |
| `oauth2_auth` | OAuth 2.0（JWT + introspect 回退） | 否 | 使用 OAuth 令牌的第三方 API 访问 |
| `combined_oauth2_auth` | OAuth 2.0 | 是 | 需要权限控制的第三方 API |

### OAuth 2.0 资源服务器

`oauth2_auth` 中间件采用**双重验证策略**来验证 auth-admin 签发的 OAuth 2.0 访问令牌：

1. **JWT 本地验证（快速）**：对于 JWT 格式令牌，使用共享 JWT 密钥验证签名 — 无需网络调用。
2. **Introspect 远程验证（回退）**：如果 JWT 验证失败（如 opaque 令牌或格式不符），通过 HTTP 调用 auth-admin 的 `/oauth/introspect` 端点。

```rust
use genies_auth::{oauth2_auth, combined_oauth2_auth, OAuth2AuthConfig};

// 配置 OAuth2 资源服务器
let oauth2_config = OAuth2AuthConfig {
    jwt_secret: "与 auth-admin 共享的密钥".to_string(),
    introspect_url: "http://127.0.0.1:9099/auth-admin/oauth/introspect".to_string(),
    service_token: Some("内部服务令牌".to_string()), // 用于向 introspect 端点认证
};

// 应用到路由
let router = Router::new()
    .push(Router::with_path("/api/protected").get(handler))
    .hoop(affix_state::inject(oauth2_config))
    .hoop(oauth2_auth)                          // OAuth2 令牌验证
    .hoop(affix_state::inject(mgr))
    .hoop(casbin_auth);                          // Casbin 权限检查

// 或使用 combined_oauth2_auth 一步完成：
let router = Router::new()
    .push(Router::with_path("/api/protected").get(handler))
    .hoop(affix_state::inject(oauth2_config))
    .hoop(affix_state::inject(mgr))
    .hoop(combined_oauth2_auth);                 // OAuth2 认证 + Casbin 一步完成
```

认证成功后，中间件会向 `Depot` 注入：
- `jwtToken`：`JWTToken` 结构体（解析后的 JWT claims）
- `subject`：用户标识字符串（如 `"user:123"`）
- `oauth_claims`：`OAuthClaims`（包含 `client_id`、`scope`、`uid`、`name`、`jti`）

## 快速开始

### 1. 添加依赖

```sh
cargo add genies_auth genies genies_derive
```

> 也可以手动在 `Cargo.toml` 中添加依赖，请前往 [crates.io](https://crates.io) 查看最新版本。

### 2. 定义带字段级权限的结构体

```rust
use genies_derive::casbin;
use salvo::oapi::ToSchema;

#[casbin]
#[derive(serde::Deserialize, ToSchema)]
pub struct User {
    pub id: u64,
    pub name: Option<String>,
    pub email: String,       // 敏感字段
    pub phone: String,       // 敏感字段
}
```

`#[casbin]` 宏自动生成：
- 自定义 `Serialize` 实现（Writer 层 JSON 树过滤）
- Salvo `Writer` trait 实现（从 Depot 提取 enforcer/subject 并过滤字段）

**自动嵌套检测**：宏自动识别非原始类型字段（struct、`Option<T>`、`Vec<T>`），对嵌套对象递归过滤。无需 `#[casbin(nested)]` 标记。

### 3. 初始化并启动服务

```rust
use std::sync::Arc;
use salvo::prelude::*;
use genies::context::CONTEXT;
use genies_auth::{EnforcerManager, casbin_auth, auth_router, extract_and_sync_schemas};

#[endpoint]
async fn get_user() -> Json<User> {
    Json(User {
        id: 1,
        name: Some("张三".into()),
        email: "zhangsan@example.com".into(),
        phone: "13800138000".into(),
    })
}

#[tokio::main]
async fn main() {
    // 初始化数据库
    CONTEXT.init_mysql().await;
    genies_auth::models::run_migrations().await;
    
    // 构建路由
    let router = Router::new()
        .push(Router::with_path("/api/users").get(get_user));
    
    // 同步 OpenAPI schemas 到数据库
    let doc = OpenApi::new("my-service", "1.0.0").merge_router(&router);
    extract_and_sync_schemas(&doc).await.ok();
    
    // 初始化 EnforcerManager
    let mgr = Arc::new(EnforcerManager::new().await.unwrap());
    
    // 应用中间件
    let router = router
        .hoop(genies::context::auth::salvo_auth)
        .hoop(affix_state::inject(mgr.clone()))
        .hoop(casbin_auth)
        .push(auth_router());
    
    let acceptor = TcpListener::new("0.0.0.0:5800").bind().await;
    Server::new(acceptor).serve(router).await;
}
```

## Admin API 接口参考

| 端点 | 方法 | 功能 |
|-----|------|------|
| `/auth/schemas` | GET | 列出所有 API Schema |
| `/auth/model` | GET | 获取 Casbin 模型定义 |
| `/auth/model` | PUT | 修改 Casbin 模型 |
| `/auth/policies` | GET | 列出所有策略 |
| `/auth/policies` | POST | 添加策略 |
| `/auth/policies/{id}` | DELETE | 删除策略 |
| `/auth/roles` | GET | 列出角色映射 (g) |
| `/auth/roles` | POST | 添加角色映射 |
| `/auth/roles/{id}` | DELETE | 删除角色映射 |
| `/auth/groups` | GET | 列出分组 (g2) |
| `/auth/groups` | POST | 添加分组 |
| `/auth/groups/{id}` | DELETE | 删除分组 |
| `/auth/reload` | POST | 手动重载 Enforcer |
| `/auth/version` | GET | 获取当前 Enforcer 版本号 |

所有端点都带有 OpenAPI 元数据注解（`#[endpoint(tags, summary, description)]`），支持自动生成 API 文档。

### SwaggerUI 集成

使用 integration 示例时，SwaggerUI 可通过以下地址访问：
- **SwaggerUI 界面**：`/swagger-ui/`
- **OpenAPI JSON**：`/api-doc/openapi.json`

## 数据库表

通过 Flyway 迁移自动创建：

| 表名 | 说明 |
|------|------|
| `casbin_rules` | 策略规则存储（ptype, v0-v5） |
| `casbin_model` | Casbin 模型定义存储 |
| `auth_api_schemas` | API Schema 元信息存储 |

## 配置说明

### 策略配置示例

```sql
-- API 接口级：guest 不能访问 /api/admin
INSERT INTO casbin_rules (ptype, v0, v1, v2, v3) VALUES ('p', 'guest', '/api/admin', 'get', 'deny');

-- 字段级：bob 不能看 email
INSERT INTO casbin_rules (ptype, v0, v1, v2, v3) VALUES ('p', 'bob', 'User.email', 'read', 'deny');

-- 角色分配：alice 是 admin
INSERT INTO casbin_rules (ptype, v0, v1) VALUES ('g', 'alice', 'admin');

-- 资源分组
INSERT INTO casbin_rules (ptype, v0, v1) VALUES ('g2', '/api/users', 'user:manage');
```

### 默认 Casbin 模型

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

## 依赖项

- **casbin** 2.10.1 - 授权库
- **salvo** - Web 框架
- **rbatis** - ORM 框架
- **flyway** - 数据库迁移
- **tokio** - 异步运行时

## 测试

61 个端到端测试覆盖：Schema 同步、模型管理、策略 CRUD、热更新、角色分配、分组管理、403 拒绝、字段过滤（原始字段、嵌套对象、Vec 数组、混合场景、Admin 完整访问、动态策略生效）、Redis 缓存、并发安全等。

```bash
cargo test -p integration auth_tests -- --nocapture --test-threads=1
```

## 许可证

请参阅项目根目录的许可证信息。
