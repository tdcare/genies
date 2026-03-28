# genies_auth

基于 [Casbin](https://casbin.org/) 的 Genies 框架权限管理模块，提供完整的 RBAC 权限解决方案。

## 概述

genies_auth 提供完整的基于角色的访问控制（RBAC）解决方案，支持 API 接口级和字段级权限控制。它与 Salvo Web 框架无缝集成，支持运行时动态更新权限策略，无需重启服务。

## 核心特性

- **混合权限模型**：API 接口级权限 + 字段级权限控制
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
| `casbin_auth` | middleware.rs | Salvo 中间件，JWT 认证 + Casbin 权限检查，注入 enforcer/subject 到 Depot |
| `auth_admin_router` | admin_api.rs | Admin API 路由（12 个端点，策略/角色/分组/模型 CRUD + reload） |
| `RBatisAdapter` | adapter.rs | Casbin Adapter 实现，对接 MySQL 存储 |
| `extract_and_sync_schemas` | schema_extractor.rs | 从 OpenAPI 文档提取 Schema 并同步到数据库 |
| `cache` | cache.rs | Redis 缓存层，策略/Schema 缓存 + 版本同步 |
| `models` | models.rs | 数据库模型 + Flyway 迁移（`run_migrations`） |

### 中间件执行流程

```
请求 → salvo_auth(JWT验证) → casbin_auth(权限检查) → 业务Handler → Writer(字段过滤) → 响应
```

## 快速开始

### 1. 添加依赖

```toml
[dependencies]
genies_auth = { path = "../path/to/genies_auth" }
genies = { path = "../path/to/genies" }
genies_derive = { path = "../path/to/genies_derive" }
```

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
- `enforcer` 和 `subject` 字段（`#[serde(skip)]`）
- `with_policy()` 方法
- `check_permission(field_name)` 方法
- 自定义 `Serialize` 实现（序列化时按字段检查权限）
- Salvo `Writer` trait 实现（自动从 Depot 注入 enforcer/subject）

### 3. 初始化并启动服务

```rust
use std::sync::Arc;
use salvo::prelude::*;
use genies::context::CONTEXT;
use genies_auth::{EnforcerManager, casbin_auth, auth_admin_router, extract_and_sync_schemas};

#[endpoint]
async fn get_user() -> Json<User> {
    Json(User {
        id: 1,
        name: Some("张三".into()),
        email: "zhangsan@example.com".into(),
        phone: "13800138000".into(),
        enforcer: None,
        subject: None,
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
        .push(auth_admin_router());
    
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
| `/auth/policies/<id>` | DELETE | 删除策略 |
| `/auth/roles` | GET | 列出角色映射 (g) |
| `/auth/roles` | POST | 添加角色映射 |
| `/auth/roles/<id>` | DELETE | 删除角色映射 |
| `/auth/groups` | GET | 列出分组 (g2) |
| `/auth/groups` | POST | 添加分组 |
| `/auth/groups/<id>` | DELETE | 删除分组 |
| `/auth/reload` | POST | 手动重载 Enforcer |

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

13 个端到端测试覆盖：Schema 同步、模型管理、策略 CRUD、热更新、角色分配、分组管理、403 拒绝、字段过滤、Redis 缓存、并发安全等。

```bash
cargo test -p integration auth_tests -- --nocapture --test-threads=1
```

## 许可证

请参阅项目根目录的许可证信息。
