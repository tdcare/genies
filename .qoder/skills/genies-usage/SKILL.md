---
name: genies-usage
description: Guide for using the Genies Rust microservice framework with DDD and Dapr. Use when developing with Genies, creating aggregates, domain events, Dapr subscriptions, Casbin field-level permissions, configuration management, or when the user asks about Genies framework usage patterns.
---

# Genies 框架使用指南

## 1. 框架简介

**Genies** 是一个基于 Rust 的 DDD + Dapr 微服务开发框架（v1.4.5），通过宏驱动架构提供声明式的聚合根、领域事件、权限控制和配置管理能力。

### 核心模块

| Crate | 版本 | 说明 |
|-------|------|------|
| `genies` | 1.4.5 | 主入口，重导出所有子 crate |
| `genies_derive` | 1.4.5 | 过程宏库：Aggregate、DomainEvent、Config、topic、casbin 等 |
| `genies_core` | 1.4.4 | 核心基础：错误处理、JWT、HTTP 响应模型 |
| `genies_config` | 1.4.2 | 配置管理：ApplicationConfig、日志配置 |
| `genies_context` | 1.4.3 | 全局上下文：CONTEXT、REMOTE_TOKEN、SERVICE_STATUS |
| `genies_cache` | 1.4.2 | 缓存抽象：Redis/内存双后端 |
| `genies_dapr` | 1.4.2 | Dapr 集成：CloudEvent、PubSub、Topic 自动收集 |
| `genies_ddd` | 1.4.2 | DDD 核心：聚合根、领域事件、消息发布 |
| `genies_k8s` | 1.4.2 | Kubernetes 探针：存活/就绪检查 |
| `genies_auth` | 1.4.2 | Casbin 权限：API 访问控制、字段级过滤、Admin UI |

## 2. 快速引用

```toml
[dependencies]
genies = "1.4"
genies_derive = "1.4"
genies_auth = "1.4"  # 可选：Casbin 权限管理
rbatis = { version = "4.5", features = ["debug_mode"] }
tokio = { version = "1.22", features = ["full"] }
salvo = { version = "0.89", features = ["rustls", "oapi", "affix-state"] }
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
```

## 3. 核心宏速查表

| 宏 | 用途 | 示例 |
|---|---|---|
| `#[derive(Aggregate)]` | 定义 DDD 聚合根 | `#[derive(Aggregate)] struct Order {...}` |
| `#[derive(DomainEvent)]` | 定义领域事件 | `#[derive(DomainEvent)] struct OrderCreated {...}` |
| `#[topic(...)]` | 订阅 Dapr 事件 | `#[topic(name="...", pubsub="messagebus")]` |
| `#[derive(Config)]` | 定义配置结构 | `#[derive(Config)] struct MyConfig {...}` |
| `#[derive(ConfigCore)]` | 框架内部配置（避免循环依赖） | 同 Config |
| `#[casbin]` | 字段级权限控制（Writer 层过滤） | `#[casbin] #[derive(Serialize)] struct User {...}` |
| `#[remote]` | HTTP 调用包装（自动刷新 Token） | `#[remote] #[get("...")] async fn ...` |

## 4. 聚合根定义

使用 `#[derive(Aggregate)]` 定义 DDD 聚合根：

```rust
use genies_derive::Aggregate;
use serde::{Deserialize, Serialize};

#[derive(Aggregate, Debug, Serialize, Deserialize, Clone)]
#[aggregate_type("me.tdcarefor.order.domain.Order")]  // 可选：自定义类型名
#[id_field(id)]                                       // 必需：指定 ID 字段
#[initialize_with_defaults]                           // 可选：启用默认值初始化
pub struct Order {
    pub id: String,
    pub customer_id: String,
    pub total_amount: f64,
    pub status: String,
}
```

**属性说明：**
- `#[aggregate_type("...")]` - 自定义聚合类型名称，默认为结构体名
- `#[id_field(field_name)]` - 必需，指定聚合 ID 字段
- `#[initialize_with_defaults]` - 自动实现 `InitializeAggregate` trait

**生成的 Traits：**
- `AggregateType` - 提供 `aggregate_type()` 和 `atype()` 方法
- `WithAggregateId` - 提供 `aggregate_id()` 方法

## 5. 领域事件

使用 `#[derive(DomainEvent)]` 标记领域事件：

```rust
use genies_derive::DomainEvent;
use serde::{Deserialize, Serialize};

// Struct 形式
#[derive(DomainEvent, Debug, Serialize, Deserialize, Default, Clone)]
#[event_type_version("V1")]
#[event_source("me.tdcarefor.order.domain.Order")]
#[event_type("me.tdcarefor.order.event.OrderCreated")]
pub struct OrderCreatedEvent {
    pub order_id: String,
    pub customer_id: String,
    pub total_amount: f64,
}

// Enum 形式（多事件类型）
#[derive(DomainEvent, Debug, Serialize, Deserialize, Clone)]
#[event_type_version("V1")]
#[event_source("me.tdcarefor.order.domain.Order")]
pub enum OrderEvent {
    #[event_type("OrderCreated")]
    Created { order_id: String },
    #[event_type("OrderShipped")]
    Shipped { tracking_number: String },
}
```

**属性说明：**
- `#[event_type("...")]` - 事件类型标识
- `#[event_type_version("...")]` - 事件版本，默认 "V0"
- `#[event_source("...")]` - 事件来源（通常是聚合根全限定名）

## 6. 事件发布

使用 `DomainEventPublisher` 发布领域事件：

```rust
use genies::ddd::DomainEventPublisher::{publish, publishGenericDomainEvent};
use rbatis::executor::Executor;

// 发布聚合根关联的事件
async fn create_order(tx: &mut dyn Executor, order: &Order) {
    let event = OrderCreatedEvent {
        order_id: order.id.clone(),
        customer_id: order.customer_id.clone(),
        total_amount: order.total_amount,
    };
    publish(tx, order, Box::new(event)).await;
}

// 发布通用领域事件（无聚合根关联）
async fn send_notification(tx: &mut dyn Executor) {
    let event = NotificationEvent { message: "Hello".to_string() };
    publishGenericDomainEvent(tx, Box::new(event)).await;
}
```

**消息表结构（message）：**
- `id` - VARCHAR(36) 主键
- `destination` - VARCHAR(255) 目标
- `headers` - TEXT 消息头
- `payload` - TEXT 消息体
- `published` - INT 发布状态（0=未发布）
- `creation_time` - BIGINT 创建时间

## 7. 事件消费

使用 `#[topic]` 宏订阅 Dapr 事件：

```rust
use genies_derive::topic;
use rbatis::executor::Executor;

#[topic(
    name = "me.tdcarefor.order.domain.Order",  // 订阅的 topic 名称
    pubsub = "messagebus"                       // Dapr pubsub 组件名
)]
pub async fn on_order_created(
    tx: &mut dyn Executor,
    event: OrderCreatedEvent
) -> anyhow::Result<u64> {
    log::info!("收到订单创建事件: {:?}", event);
    // 处理事件逻辑...
    Ok(0)
}
```

**`#[topic]` 参数：**
- `name` - Topic 名称，默认使用聚合类型名
- `pubsub` - PubSub 组件名，默认 "messagebus"
- `metadata` - 额外元数据，格式 "key1=value1,key2=value2"

**宏自动生成：**
- `{fn_name}_hoop` - Salvo Handler
- `{fn_name}_dapr` - Dapr 订阅配置
- `{fn_name}_hoop_router` - 路由注册
- 自动幂等性检查（基于 Redis）
- 自动事务管理和重试逻辑

**注册消费者路由：**
```rust
use genies::dapr::dapr_sub::dapr_sub;

pub fn event_router() -> Router {
    Router::new().push(
        Router::with_path("/daprsub/consumers")
            .hoop(on_order_created_hoop)
            .post(dapr_sub)
    )
}
```

## 8. 配置管理

使用 `#[derive(Config)]` 定义配置结构：

```rust
use genies_derive::Config;
use serde::Deserialize;

#[derive(Config, Debug, Deserialize)]
pub struct MyAppConfig {
    #[config(default = "my-service")]
    pub server_name: String,
    
    #[config(default = "8080")]
    pub port: u32,
    
    #[config(default = "")]
    pub api_key: Option<String>,
    
    #[config(default = "")]
    pub allowed_origins: Vec<String>,
}

// 使用
let config = MyAppConfig::from_sources("./application.yml").unwrap();
```

**配置加载优先级：** 默认值 → YAML 文件 → 环境变量

**环境变量格式：** 支持 `field_name` 和 `FIELD_NAME` 两种格式

**生成的方法：**
- `from_file(path)` - 从文件加载
- `from_sources(path)` - 从多源加载（推荐）
- `validate()` - 验证配置
- `merge(other)` - 合并配置
- `load_env()` - 加载环境变量

## 9. 字段级权限（方案 C：Writer 层过滤）

使用 `#[casbin]` 宏实现 Casbin 字段级访问控制。该宏采用 **Writer 层 JSON 树过滤** 方案：

### 9.1 基本用法

```rust
use genies_derive::casbin;
use serde::{Deserialize, Serialize};
use salvo::oapi::ToSchema;

#[casbin]  // 必须放在最前面
#[derive(Serialize, Deserialize, ToSchema)]
pub struct Employee {
    pub id: u64,
    pub name: String,
    pub id_card_number: String,   // 敏感字段
    pub base_salary: f64,         // 敏感字段
    pub home_address: Address,    // 嵌套对象（自动检测）
    pub bank_accounts: Vec<BankAccount>,  // 嵌套数组（自动检测）
}

// 在 Handler 中使用 — 直接返回结构体，Writer 自动过滤
#[endpoint]
async fn get_employee() -> Employee {
    Employee {
        id: 1,
        name: "张三".into(),
        id_card_number: "310101199501011234".into(),
        base_salary: 25000.0,
        home_address: Address { city: "上海".into(), street: "张江路".into() },
        bank_accounts: vec![BankAccount { account_number: "622202xxx".into() }],
    }
}
```

### 9.2 宏生成内容

`#[casbin]` 宏自动生成：

1. **`casbin_filter()` 方法** — 对 JSON Value 树进行递归权限过滤
2. **`Writer` trait 实现** — 在 HTTP 响应序列化时自动应用权限过滤
3. **自动嵌套检测** — 非原始类型字段自动递归过滤（无需 `#[casbin(nested)]`）

**原始类型白名单**（这些类型不会递归）：
```
i8, i16, i32, i64, i128, u8, u16, u32, u64, u128, f32, f64,
isize, usize, String, str, bool, char
```

### 9.3 中间件配置

权限过滤依赖 `casbin_auth` 中间件将 `enforcer` 和 `subject` 注入 Depot：

```rust
use genies_auth::{EnforcerManager, casbin_auth};
use salvo::prelude::*;

let mgr = Arc::new(EnforcerManager::new().await?);

let router = Router::new()
    .hoop(genies::context::auth::salvo_auth)  // JWT 认证
    .hoop(affix_state::inject(mgr.clone()))    // 注入 EnforcerManager
    .hoop(casbin_auth)                          // 权限中间件
    .push(Router::with_path("/api/employees").get(get_employee));
```

### 9.4 policy.csv 示例

```csv
# 字段级权限（黑名单模式：默认允许，deny 规则生效）
p, guest, Employee.id_card_number, read, deny
p, guest, Employee.base_salary, read, deny
p, guest, BankAccount.account_number, read, deny

# 角色继承
g, alice, hr_admin
g, bob, guest

# 资源分组
g2, Employee.base_salary, sensitive_data
g2, Employee.id_card_number, sensitive_data
```

## 10. 缓存服务

使用 `CacheService` 进行缓存操作：

```rust
use genies::context::CONTEXT;
use std::time::Duration;

async fn cache_example() -> Result<()> {
    let cache = &CONTEXT.cache_service;
    
    // 字符串操作
    cache.set_string("key1", "value1").await?;
    let value = cache.get_string("key1").await?;
    cache.del_string("key1").await?;
    
    // 带过期时间
    cache.set_string_ex("session", "token", Some(Duration::from_secs(3600))).await?;
    
    // JSON 操作
    cache.set_json("user:1", &user).await?;
    let user: User = cache.get_json("user:1").await?;
    
    // 获取 TTL
    let ttl = cache.ttl("session").await?;
    Ok(())
}
```

**ICacheService 接口：**
- `set_string(k, v)` / `get_string(k)` / `del_string(k)`
- `set_string_ex(k, v, ex)` - 带过期时间
- `ttl(k)` - 获取剩余生存时间

**切换后端（application.yml）：**
```yaml
cache_type: "redis"  # 或 "mem"
redis_url: "redis://:password@localhost:6379"
```

## 11. HTTP 包装器

使用 `#[remote]` 宏包装跨微服务 HTTP 调用：

```rust
use genies_derive::remote;
use feignhttp::get;

#[remote]
#[get("${gateway}/user-service/api/users/{id}")]
pub async fn get_user_by_id(#[path] id: i64) -> feignhttp::Result<User> {}

// 使用时无需手动传递 Authorization header
async fn example() {
    let user = get_user_by_id(123).await.unwrap();
}
```

**功能：**
- 自动从 `REMOTE_TOKEN` 获取访问令牌
- 遇到 401 时自动刷新令牌并重试

**配置 gateway（使用 `config_gateway!` 宏）：**
```rust
lazy_static! {
    pub static ref GATEWAY: String = config_gateway!("/user");
}
```

## 12. 便捷宏

```rust
// pool!() - 获取数据库连接池
let rb = pool!();
User::select_by_id(rb, 1).await?;

// tx_defer!() - 获取带自动回滚的事务
let mut tx = tx_defer!();
User::insert(&mut tx, &user).await?;
tx.commit().await;  // 不 commit 会自动 rollback

// copy!(src, DestType) - 字段拷贝转换
let user_dto = copy!(&user_entity, UserDTO);

// config_gateway!(servlet_path) - 配置 gateway URL
// 如果 gateway 是 http/https 协议则使用网关，否则使用 Dapr
```

## 13. K8s 健康检查

```rust
use genies::k8s::k8s_health_check;
use salvo::Router;

let router = Router::new()
    .push(k8s_health_check());  // 自动添加健康检查路由
```

**端点：**
- `GET /actuator/health/liveness` - 存活探针
- `GET /actuator/health/readiness` - 就绪探针

**修改服务状态：**
```rust
use genies::context::SERVICE_STATUS;
use std::ops::DerefMut;

let mut status = SERVICE_STATUS.lock().unwrap();
status.deref_mut().insert("readinessProbe".to_string(), false);
```

## 14. OpenAPI 集成

Genies 项目使用 Salvo 的 OpenAPI 能力自动生成 API 文档，并与 `genies_auth` 权限系统联动。

### 14.1 `#[endpoint]` vs `#[handler]`

所有 HTTP handler **必须使用 `#[endpoint]`** 而非 `#[handler]`：

- `#[endpoint]` — 自动将函数签名注册到 OpenAPI 文档，支持 Schema 同步到权限系统
- `#[handler]` — 不生成 OpenAPI 文档，无法被 `extract_and_sync_schemas` 识别

> 两者函数签名完全相同，迁移只需替换宏名。

### 14.2 OpenAPI 参数提取方式

使用 `#[endpoint]` 时，推荐使用 OpenAPI 提取器替代手动从 `Request` 提取参数。提取器会自动在 OpenAPI 文档中生成参数描述：

| `#[handler]` 旧写法 | `#[endpoint]` 新写法 | 说明 |
|---------------------|---------------------|------|
| `req.param::<T>("name")` | `name: PathParam<T>` | 路径参数 `/users/{id}` |
| `req.query::<T>("name")` | `name: QueryParam<T, REQUIRED>` | 查询参数 `?page=0` |
| `req.parse_json::<T>().await` | `body: JsonBody<T>` | JSON 请求体 |
| `res.render(Json(data))` | `-> Json<T>` 返回值 | 响应体（自动生成响应 Schema） |

> `QueryParam<T, false>` 表示可选参数，`QueryParam<T, true>` 表示必填参数。

**完整示例：**

```rust
use salvo::prelude::*;
use salvo::oapi::extract::*;

/// Path 参数 — 替代 req.param
#[endpoint]
async fn find_by_id(id: PathParam<String>) -> Json<DeviceVO> {
    let result = DeviceAppService::get_device(&id.into_inner()).await.unwrap();
    Json(result)
}

/// Query 参数 — 替代 req.query
#[endpoint]
async fn list(department_id: QueryParam<String, false>) -> Json<Vec<DeviceVO>> {
    let dept = department_id.into_inner().unwrap_or_default();
    let devices = DeviceAppService::list_by_dept(&dept).await;
    Json(devices)
}

/// JSON Body — 替代 req.parse_json
#[endpoint]
async fn add(body: JsonBody<DeviceBindRequest>) -> Json<RespVO<String>> {
    let dto = body.into_inner();
    match DeviceAppService::create(&dto).await {
        Ok(id) => Json(RespVO::from(&Ok::<_, String>(id))),
        Err(e) => Json(RespVO::<String>::from_error(&e)),
    }
}
```

### 14.3 DTO 必须 derive ToSchema

所有请求/响应 DTO 必须添加 `#[derive(ToSchema)]`，确保 OpenAPI 文档包含完整的 Schema 定义：

```rust
use salvo::oapi::ToSchema;
use serde::{Deserialize, Serialize};
use genies_derive::casbin;

/// 请求 DTO
#[derive(Debug, Deserialize, ToSchema)]
pub struct DeviceBindRequest {
    pub serial_number: String,
}

/// 响应 DTO — #[casbin] + ToSchema 联合使用
#[casbin]
#[derive(Deserialize, ToSchema)]
pub struct DeviceVO {
    pub id: String,
    pub name: String,
    pub serial_number: String,  // 敏感字段，可被权限策略隐藏
}
```

### 14.4 Schema 同步到权限系统

启动时调用 `extract_and_sync_schemas(&doc)` 将 OpenAPI Schema 中的所有 DTO 字段信息同步到 `auth_api_schemas` 表，供 Admin UI 配置字段级权限：

```rust
use genies_auth::extract_and_sync_schemas;
use salvo::oapi::OpenApi;

// 构建路由（所有 handler 使用 #[endpoint]）
let router = Router::new()
    .push(Router::with_path("/api")
        .push(business_router()));

// 生成 OpenAPI 文档并同步 Schema
let doc = OpenApi::new("my-service", "1.0.0").merge_router(&router);
extract_and_sync_schemas(&doc).await.ok();
```

### 14.5 与 `#[casbin]` 字段级权限的配合

完整链路：

1. DTO 添加 `#[derive(ToSchema)]` → OpenAPI 文档包含字段定义
2. `extract_and_sync_schemas(&doc)` → 字段信息写入 `auth_api_schemas` 表
3. 通过 Admin UI（`/auth/ui/`）基于 Schema 配置字段级 deny 策略
4. 响应 DTO 添加 `#[casbin]` → Writer 层在序列化时根据策略自动过滤字段

> 如果 DTO 不 derive ToSchema，`extract_and_sync_schemas` 无法提取其字段，Admin UI 中将看不到该 DTO 的字段列表。

## 15. 典型开发流程

1. **项目初始化**
   - 创建 Cargo.toml 添加依赖
   - 创建 application.yml 配置文件
   - 创建 model.conf 和 policy.csv（如需权限控制）

2. **定义聚合根**
   - 使用 `#[derive(Aggregate)]` 定义领域模型
   - 指定 `#[id_field]` 标识字段

3. **定义领域事件**
   - 使用 `#[derive(DomainEvent)]` 定义事件
   - 设置 `event_type`、`event_source`、`event_type_version`

4. **实现业务逻辑**
   - 使用 `tx_defer!()` 管理事务
   - 使用 `DomainEventPublisher::publish()` 发布事件

5. **订阅事件**
   - 使用 `#[topic]` 宏定义事件处理器
   - 使用 `dapr_event_router()` 自动注册路由

6. **启动服务**
   ```rust
   use genies_auth::{EnforcerManager, casbin_auth, auth_admin_router, auth_admin_ui_router};
   
   #[tokio::main]
   async fn main() {
       genies::config::log_config::init_log();
       CONTEXT.init_mysql().await;
       
       // 初始化 Casbin Enforcer（可选）
       let mgr = Arc::new(EnforcerManager::new().await.unwrap());
       
       let router = Router::new()
           .push(genies::k8s::k8s_health_check())
           .push(Router::with_path("/api")
               .hoop(genies::context::auth::salvo_auth)
               .hoop(affix_state::inject(mgr.clone()))
               .hoop(casbin_auth)
               .push(business_router())
               .push(auth_admin_router()))  // 权限管理 API
           .push(auth_admin_ui_router())    // 权限管理 UI
           .push(genies::dapr_event_router());  // Dapr 事件路由
       
       let acceptor = TcpListener::new(&CONTEXT.config.server_url).bind().await;
       Server::new(acceptor).serve(router).await;
   }
   ```

7. **部署**
   - 配置 Dapr sidecar
   - 设置 K8s 健康检查探针
   - 通过环境变量覆盖配置

## 16. Auth 模块

`genies_auth` 提供完整的 Casbin 权限管理方案：

### 公开 API

| 导出项 | 说明 |
|--------|------|
| `EnforcerManager` | Casbin Enforcer 管理器，支持热更新 |
| `casbin_auth` | API 接口权限中间件 |
| `casbin_filter_object` | 嵌套字段过滤函数 |
| `auth_admin_router()` | Admin API 路由（需认证） |
| `auth_public_router()` | 公开 API 路由（Token 端点） |
| `auth_admin_ui_router()` | Admin UI 静态资源路由 |
| `extract_and_sync_schemas()` | OpenAPI Schema 同步到数据库 |

### 快速集成

```rust
use genies_auth::{EnforcerManager, casbin_auth, auth_admin_router, auth_admin_ui_router};
use std::sync::Arc;

// 1. 初始化 Enforcer
let mgr = Arc::new(EnforcerManager::new().await?);

// 2. 配置路由
let router = Router::new()
    .hoop(affix_state::inject(mgr.clone()))
    .hoop(casbin_auth)
    .push(auth_admin_router())   // /auth/policies, /auth/roles, ...
    .push(auth_admin_ui_router());  // /auth/ui/
```

### Admin UI

访问 `/auth/ui/` 可使用 Web 界面管理：
- 策略规则（Policies）
- 角色分配（Roles）
- 用户组（Groups）
- Casbin 模型配置
- OpenAPI Schema 浏览

## 17. ID 生成

Genies 提供了统一的雪花 ID 生成器，用于替代 UUID 生成所有业务 ID。

### 用法

```rust
// 生成唯一 ID — 最常见的用法
let id = genies::next_id();

// 示例：创建新实体
ward.id = Some(genies::next_id());
```

### 工作原理

- 使用 `rs-snowflake`（64 位分布式雪花算法）
- Worker ID 在启动时自动解析：Redis 槽位 → K8s HOSTNAME → 配置项 → 兜底值
- ID 以 `String` 类型返回，避免 JavaScript 精度丢失问题
- 通过 `Mutex<SnowflakeIdBucket>` 包装在 `OnceLock` 中，保证线程安全

### 从 UUID 迁移

替换所有以下写法：
```rust
// 迁移前
use uuid::Uuid;
let id = Uuid::new_v4().to_string();

// 迁移后
let id = genies::next_id();
```

迁移完成后，从 `Cargo.toml` 依赖中移除 `uuid`。
