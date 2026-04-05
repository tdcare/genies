# Genies DDD 微服务开发 — 快速 API 参考

> 按 DDD 分层模式组织，仅包含高频使用的 API。

---

## 1. 接口层（Salvo Web 框架）

### Handler 定义

```rust
use salvo::prelude::*;

// 基础 handler
#[handler]
pub async fn my_handler(req: &mut Request, depot: &mut Depot, res: &mut Response) {
    // ...
}

// 带 OpenAPI 文档的 handler（推荐）
#[endpoint]
pub async fn my_endpoint() -> ResultDTO<MyVO> {
    ResultDTO::success("ok", data)
}
```

### 请求数据提取

```rust
// JSON body
let dto: MyDto = req.parse_json::<MyDto>().await?;

// 路径参数: /resource/{id}
let id: String = req.param::<String>("id").unwrap();

// 查询参数: /resource?key=value
let key: Option<String> = req.query::<String>("key");

// 表单字段
let field: String = req.form::<String>("field").await.unwrap();

// Salvo 提取器（用于 #[endpoint]）
async fn handler(id: PathParam<i64>, body: JsonBody<MyDto>) -> ResultDTO<String> { }
```

### 响应渲染

```rust
// JSON 响应
res.render(Json(data));

// #[casbin] VO 自动字段过滤渲染（直接返回即可）
#[endpoint]
async fn get_user() -> MyVO {
    MyVO { /* ... */ }
}

// ResultDTO 响应
ResultDTO::success("操作成功", data)
ResultDTO::error("操作失败")
ResultDTO::success_empty("完成")

// RespVO 响应
RespVO::from(&data)
RespVO::from_result(&result)
RespVO::from_error_info("VALIDATION_ERROR", "参数无效")
```

### 路由配置

```rust
Router::with_path("resource")
    .hoop(casbin_auth)                                    // 权限中间件
    .push(Router::with_path("list").get(list_handler))
    .push(Router::with_path("{id}").get(get_handler))
    .push(Router::with_path("create").post(create_handler))
```

### OpenAPI / ToSchema

```rust
// DTO 自动生成 Schema
#[derive(Deserialize, ToSchema)]
pub struct CreateUserDto {
    pub name: String,
    pub email: String,
}

// 生成 OpenAPI 文档
let doc = OpenApi::new("my-service", "1.0.0").merge_router(&router);
```

---

## 2. 应用层（Dapr 事件消费）

### 事件消费者定义

```rust
use genies_derive::topic;
use rbatis::executor::Executor;

// name = 聚合类型名（即 Message.destination）
// pubsub = Dapr pubsub 组件名
#[topic(name = "Device", pubsub = "messagebus")]
pub async fn on_device_created(tx: &mut dyn Executor, event: DeviceCreatedEvent) -> anyhow::Result<u64> {
    // 处理事件逻辑
    println!("收到事件: {:?}", event);
    Ok(0)  // 返回 0 表示成功
}
```

### Dapr 路由注册

```rust
use genies::dapr_event_router;

// 一行代码完成 Dapr 事件路由（推荐）
let router = Router::new()
    .push(business_router())
    .push(dapr_event_router());
// 自动生成:
//   GET  /dapr/subscribe      — Dapr 订阅配置
//   POST /daprsub/consumers   — 事件消费端点
```

---

## 3. 领域层（DDD 核心）

### 聚合根定义

```rust
use genies_derive::Aggregate;
use serde::{Deserialize, Serialize};

#[derive(Aggregate, Debug, Clone, Serialize, Deserialize)]
#[aggregate_type("Device")]          // 聚合类型名称
pub struct Device {
    pub id: String,                  // 第一个字段自动作为 aggregate_id
    pub name: String,
    pub status: String,
}
// 自动实现: AggregateType trait + WithAggregateId trait
```

### 领域事件定义

```rust
use genies_derive::DomainEvent;
use serde::{Deserialize, Serialize};

#[derive(DomainEvent, Debug, Serialize, Deserialize, Default, Clone)]
#[event_type("com.example.device.event.DeviceCreated")]      // 事件全限定名
#[event_type_version("V1")]                                   // 事件版本
#[event_source("com.example.device.domain.Device")]           // 来源聚合
pub struct DeviceCreatedEvent {
    pub id: String,
    pub name: String,
    pub created_at: i64,
}
// 自动实现: DomainEvent trait（event_type, event_type_version, event_source, json）
```

### 事件发布

```rust
use genies_ddd::DomainEventPublisher::{publish, publishGenericDomainEvent};

// 带聚合上下文发布（headers 自动填充 aggregate_type/aggregate_id）
publish(tx, &device, Box::new(event)).await;

// 通用事件发布（destination = "GenericDomainEvent"）
publishGenericDomainEvent(tx, Box::new(event)).await;
```

---

## 4. 仓储层（RBatis ORM）

### 模型定义 + CRUD 宏

```rust
use rbatis::crud;
use rbatis::rbdc::datetime::DateTime;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub struct User {
    pub id: Option<String>,
    pub name: Option<String>,
    pub status: Option<i32>,
    pub create_time: Option<DateTime>,
}

// 自动生成: insert, insert_batch, select_by_map, update_by_map, delete_by_map
crud!(User {});
// 自定义表名: crud!(User {}, "t_user");
```

### 常用 CRUD 操作

```rust
use rbs::value;

// 插入
User::insert(&rb, &user).await?;
User::insert_batch(&rb, &users, 10).await?;  // 批量，batch_size=10

// 查询
let list = User::select_by_map(&rb, value!{"id": "1"}).await?;
let list = User::select_by_map(&rb, value!{"name like ": "%test%"}).await?;
let list = User::select_by_map(&rb, value!{"id": &["1", "2", "3"]}).await?;

// 更新
User::update_by_map(&rb, &user, value!{"id": "1"}).await?;

// 删除
User::delete_by_map(&rb, value!{"id": "1"}).await?;
```

### 动态 SQL（py_sql）

```rust
use rbatis::py_sql;
use rbatis::executor::Executor;

#[py_sql(
    "`select * from user where 1=1`
      if name != '':
        ` and name like #{name}`
      if status != null:
        ` and status = #{status}`
      ` order by create_time desc`"
)]
pub async fn search_users(
    rb: &dyn Executor,
    name: &str,
    status: &Option<i32>,
) -> rbatis::Result<Vec<User>> {
    impled!()
}
```

**占位符规则**：
- `#{arg}` — 预编译参数（防 SQL 注入）
- `${arg}` — 直接拼接（用于动态片段如 `ids.sql()`）

### 分页查询

```rust
use rbatis::plugin::page::{Page, PageRequest};

#[html_sql(
    r#"<select id="select_page">
        `select * from user`
        <where>
            <if test="name != ''">` and name like #{name}`</if>
        </where>
    </select>"#
)]
pub async fn select_user_page(
    rb: &dyn Executor,
    page_req: &PageRequest,
    name: &str,
) -> rbatis::Result<Page<User>> {
    impled!()
}

// 使用
let page_req = PageRequest::new(1, 10);  // 第1页，每页10条
let page: Page<User> = select_user_page(&rb, &page_req, "test").await?;
// page.records, page.total, page.page_no, page.page_size
```

### 事务

```rust
// 获取事务（drop 时自动 rollback）
let tx = rb.acquire_begin().await?;
let tx = tx.defer_async(|tx| async move {
    if !tx.done() {
        let _ = tx.rollback().await;
    }
});

// 执行操作
User::insert(&tx, &user).await?;

// 提交
tx.commit().await?;
```

### 原生 SQL

```rust
use rbatis::sql;

#[sql("select * from user where status = ? and name like ?")]
async fn find_active_users(rb: &RBatis, status: &i32, name: &str) -> rbatis::Result<Vec<User>> {
    impled!()
}
```

---

## 5. 基础设施层（数据库迁移 / 上下文 / 缓存）

### Flyway 数据库迁移

**SQL 文件命名**：`V<版本号>__<描述>.sql`（双下划线），放在 `migrations/` 目录下。

```
migrations/
├── V1__create_users.sql
├── V2__create_orders.sql
└── V3__add_indexes.sql
```

**迁移模块**：

```rust
use std::sync::Arc;
use flyway::MigrationRunner;
use flyway_rbatis::RbatisMigrationDriver;
use genies::context::CONTEXT;

#[flyway::migrations("migrations")]   // 路径相对于 CARGO_MANIFEST_DIR
pub struct Migrations {}

pub async fn run_migrations() {
    let rbatis = Arc::new(CONTEXT.rbatis.clone());
    let driver = Arc::new(RbatisMigrationDriver::new(rbatis, None));
    let runner = MigrationRunner::new(
        Migrations {},
        driver.clone(),   // StateManager
        driver.clone(),   // Executor
        false,            // false=失败停止（推荐生产环境）
    );
    runner.migrate().await.expect("数据库迁移失败");
}
```

**Cargo.toml 依赖**：

```toml
[dependencies]
flyway = { workspace = true }        # "0.5"
flyway-rbatis = { workspace = true } # "0.5"
```

### 全局上下文（CONTEXT）

```rust
use genies::context::CONTEXT;

// 初始化（应用启动时调用，幂等）
CONTEXT.init_database().await;

// 数据库连接池
let rb = &CONTEXT.rbatis;

// 应用配置
let config = &CONTEXT.config;
let server_name = &config.server_name;

// 缓存服务
let cache = &CONTEXT.cache_service;

// 持久化缓存（事件幂等性）
let save_cache = &CONTEXT.redis_save_service;

// Keycloak 公钥
let keys = &CONTEXT.keycloak_keys;
```

### 缓存操作

```rust
use genies::context::CONTEXT;
use std::time::Duration;

let cache = &CONTEXT.cache_service;

// 基础操作
cache.set_string("key", "value").await?;
let val = cache.get_string("key").await?;
cache.del_string("key").await?;

// 带过期时间
cache.set_string_ex("key", "value", Some(Duration::from_secs(300))).await?;

// 二进制操作
cache.set_value("key", &bytes).await?;
let bytes = cache.get_value("key").await?;

// TTL 查询（-1=永不过期, -2=键不存在）
let ttl = cache.ttl("key").await?;
```

---

## 6. 权限控制（Casbin）

### 字段级权限（VO 自动过滤）

```rust
use genies_derive::casbin;
use serde::{Deserialize, Serialize};
use salvo::oapi::ToSchema;

#[casbin]                                   // 自动生成 Writer + casbin_filter
#[derive(Serialize, Deserialize, ToSchema)]
pub struct UserVO {
    pub id: u64,
    pub name: String,
    pub email: String,        // 可被 deny 策略隐藏
    pub phone: String,        // 可被 deny 策略隐藏
    pub address: Address,     // 自动递归过滤嵌套类型
}

// handler 中直接返回，Writer 自动过滤
#[endpoint]
async fn get_user() -> UserVO {
    UserVO { /* ... */ }
}
```

### API 级权限中间件

```rust
use genies_auth::{EnforcerManager, casbin_auth, auth_admin_router};
use genies::context::auth::salvo_auth;
use salvo::prelude::*;
use std::sync::Arc;

// 初始化 Enforcer
let mgr = Arc::new(EnforcerManager::new().await.unwrap());

// 中间件挂载顺序（必须严格遵循）
let router = Router::new()
    .push(business_router())
    .hoop(salvo_auth)                          // 1. JWT 认证
    .hoop(affix_state::inject(mgr.clone()))    // 2. 注入 EnforcerManager
    .hoop(casbin_auth)                         // 3. Casbin 权限检查
    .push(auth_admin_router());                // Admin API（可选）
```

### Casbin 策略示例

```sql
-- API deny: guest 不能访问 /api/admin
INSERT INTO casbin_rules (ptype,v0,v1,v2,v3) VALUES ('p','guest','/api/admin','get','deny');

-- 字段 deny: bob 不能看 UserVO.email
INSERT INTO casbin_rules (ptype,v0,v1,v2,v3) VALUES ('p','bob','UserVO.email','read','deny');

-- 角色: alice 是 admin
INSERT INTO casbin_rules (ptype,v0,v1) VALUES ('g','alice','admin');
```

---

## 7. 微服务启动模板

```rust
use genies::context::CONTEXT;
use genies_auth::{
    auth_admin_router, auth_admin_ui_router, auth_public_router,
    casbin_auth, extract_and_sync_schemas, EnforcerManager,
};
use salvo::prelude::*;
use std::sync::Arc;

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt().init();

    // 1. 初始化数据库
    CONTEXT.init_database().await;

    // 2. 运行迁移
    crate::migrations::run_migrations().await;

    // 3. 构建业务路由
    let business_router = Router::new()
        .push(Router::with_path("/api/users").get(get_user));

    // 4. OpenAPI Schema 同步（可选）
    let doc = OpenApi::new("my-service", "1.0.0").merge_router(&business_router);
    extract_and_sync_schemas(&doc).await.ok();

    // 5. 权限中间件
    let mgr = Arc::new(EnforcerManager::new().await.unwrap());
    let protected_router = business_router
        .hoop(genies::context::auth::salvo_auth)
        .hoop(affix_state::inject(mgr.clone()))
        .hoop(casbin_auth)
        .push(auth_admin_router());

    // 6. 组装完整路由
    let router = Router::new()
        .push(protected_router)
        .push(auth_admin_ui_router())       // Auth Admin UI
        .push(auth_public_router())         // 公开 Token 端点
        .push(genies::k8s::k8s_health_check())  // K8s 探针
        .push(genies::dapr_event_router()); // Dapr 事件路由

    // 7. 启动服务器
    let acceptor = TcpListener::new(&CONTEXT.config.server_url).bind().await;
    Server::new(acceptor).serve(router).await;
}
```

---

## 8. 常用 Cargo.toml 依赖模板

```toml
[dependencies]
# Genies 框架
genies = { workspace = true }
genies_derive = { workspace = true }
genies_ddd = { workspace = true }
genies_auth = { workspace = true }

# 数据库迁移
flyway = { workspace = true }
flyway-rbatis = { workspace = true }

# ORM
rbatis = { version = "4.8", features = ["debug_mode"] }
rbs = { version = "4.7" }

# Web 框架
salvo = { version = "0.89", features = ["oapi"] }

# 序列化
serde = { version = "1", features = ["derive"] }
serde_json = "1"

# 异步
tokio = { version = "1", features = ["full"] }
anyhow = "1"
```

---

## 9. 速查表

| 场景 | API |
|------|-----|
| 定义 handler | `#[endpoint]` 或 `#[handler]` |
| 解析 JSON body | `req.parse_json::<T>().await` |
| 路径参数 | `req.param::<T>("name")` |
| 查询参数 | `req.query::<T>("key")` |
| JSON 响应 | `res.render(Json(data))` |
| 成功响应 | `ResultDTO::success("msg", data)` |
| 错误响应 | `ResultDTO::error("msg")` |
| 定义聚合根 | `#[derive(Aggregate)] #[aggregate_type("X")]` |
| 定义领域事件 | `#[derive(DomainEvent)] #[event_type("...")] #[event_type_version("V1")] #[event_source("...")]` |
| 发布事件 | `publish(tx, &aggregate, Box::new(event)).await` |
| 消费事件 | `#[topic(name = "X", pubsub = "messagebus")]` |
| CRUD 宏 | `crud!(Entity {});` |
| 按条件查询 | `Entity::select_by_map(&rb, value!{"col": val}).await` |
| 动态 SQL | `#[py_sql("...")]` |
| 分页查询 | `PageRequest::new(page, size)` + `Page<T>` |
| 事务 | `rb.acquire_begin().await` → `tx.commit().await` |
| 数据库迁移 | `#[flyway::migrations("migrations")]` |
| 全局上下文 | `CONTEXT.rbatis` / `CONTEXT.config` / `CONTEXT.cache_service` |
| 缓存读写 | `cache.set_string("k","v").await` / `cache.get_string("k").await` |
| 字段权限 | `#[casbin]` 标注在 VO struct 上 |
| API 权限 | `.hoop(salvo_auth).hoop(inject(mgr)).hoop(casbin_auth)` |
| Dapr 路由 | `router.push(dapr_event_router())` |
| K8s 探针 | `router.push(genies::k8s::k8s_health_check())` |
| SQL 文件命名 | `V<N>__<desc>.sql` |
