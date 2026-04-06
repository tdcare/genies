---
name: genies-ddd-microservice
description: Guide for developing Rust microservices using Genies framework following Java DDD layering principles. Use when creating new microservices, designing aggregate roots, implementing domain events, organizing service layers, setting up Flyway migrations, or structuring a DDD-based Genies project.
---

# Genies DDD 微服务开发指南

## 1. 概述

本指南将 Java DDD 四层架构映射到 Rust/Genies 框架，帮助从 Java DDD 微服务迁移到 Rust 微服务开发。

## 2. Java DDD 四层架构到 Rust/Genies 的映射

### Java 分层

- **接口层 (Interface/API)**: `web/controller/` — REST Controllers，参数验证，HTTP 响应
- **应用层 (Application)**: `service/` — 应用用例、事务管理；`consumers/` — 事件消费者；`dto/` — 数据传输对象
- **领域层 (Domain)**: `aggregate/` — 聚合根；`domain/entity/` — 领域实体；`command/` — 命令；`event/` — 事件；`service/` — 域服务；`repository/` — 仓储接口
- **基础设施层 (Infrastructure)**: `config/` — 基础设施配置；`db/` — Flyway 迁移；Spring Data JPA + Eventuate TRAM

### Rust/Genies 对应

| Java 层 | Rust/Genies 实现 |
|---------|-----------------|
| 接口层 | Salvo `#[endpoint]` + `Router` 配置（自动生成 OpenAPI 文档） |
| 应用层 | 应用服务 `async fn` + `#[topic]` 事件消费者 |
| 领域层 | `#[derive(Aggregate)]` 聚合根 + `#[derive(DomainEvent)]` 事件 + RBatis |
| 基础设施层 | `genies_config` + `genies_context` + `flyway-rs` 迁移 + RBatis ORM |

## 3. 标准目录结构

以 `my_service` crate 为例（包含多个聚合根的复杂场景）：

```
crates/my_service/
├── Cargo.toml
├── migrations/                         # 基础设施层：Flyway SQL 迁移
│   ├── V1__create_tables.sql
│   └── V2__seed_data.sql
├── src/
│   ├── lib.rs                          # 模块导出
│   ├── domain/                         # 领域层
│   │   ├── mod.rs
│   │   ├── aggregate/                  # 聚合根（按业务实体分文件）
│   │   │   ├── mod.rs
│   │   │   ├── device.rs               # 设备聚合根
│   │   │   ├── blood_bag.rs            # 血袋聚合根
│   │   │   └── doctor_advice.rs        # 医嘱聚合根
│   │   ├── event/                      # 领域事件（按聚合根或业务域分文件）
│   │   │   ├── mod.rs
│   │   │   ├── device_event.rs         # 设备相关事件
│   │   │   ├── blood_bag_event.rs      # 血袋相关事件
│   │   │   └── advice_event.rs         # 医嘱相关事件
│   │   ├── entity/                     # 领域实体（非聚合根的持久化实体）
│   │   │   ├── mod.rs
│   │   │   ├── device_entity.rs
│   │   │   └── blood_bag_entity.rs
│   │   ├── repository/                 # 仓储（按实体分文件）
│   │   │   ├── mod.rs
│   │   │   ├── device_repository.rs
│   │   │   └── blood_bag_repository.rs
│   │   ├── service/                    # 域服务（按业务职责分文件）
│   │   │   ├── mod.rs
│   │   │   ├── device_service.rs       # 设备域服务
│   │   │   ├── blood_bag_service.rs    # 血袋域服务
│   │   │   └── advice_service.rs       # 医嘱域服务（主服务，聚合子服务）
│   │   └── command/                    # 命令对象（可选，复杂业务时使用）
│   │       ├── mod.rs
│   │       └── blood_bag_command.rs
│   ├── application/                    # 应用层
│   │   ├── mod.rs
│   │   ├── service.rs                  # 应用服务（用例编排、事务）
│   │   ├── consumer/                   # 事件消费者（按监听的聚合根分文件）
│   │   │   ├── mod.rs
│   │   │   ├── device_consumer.rs      # 监听 Device 聚合根事件
│   │   │   └── blood_bag_consumer.rs   # 监听 BloodBag 聚合根事件
│   │   └── dto.rs                      # DTO（请求/响应对象）
│   ├── interfaces/                     # 接口层
│   │   ├── mod.rs
│   │   ├── handler/                    # Salvo HTTP handlers（按资源分文件）
│   │   │   ├── mod.rs
│   │   │   ├── device_handler.rs
│   │   │   └── blood_bag_handler.rs
│   │   └── router.rs                   # 路由配置（汇总所有 handler 的路由）
│   ├── infrastructure/                 # 基础设施层
│   │   ├── mod.rs
│   │   └── migration.rs                # Flyway 迁移配置
│   └── remote/                         # 远程服务调用层（独立于 DDD 四层）
│       ├── mod.rs                      # 模块导出
│       ├── patient_service.rs          # 患者服务远程调用
│       ├── baseinfo_service.rs         # 基础信息服务远程调用
│       └── his_service.rs              # HIS 系统远程调用
```

**目录划分原则：**
- `domain/` 下按职责（aggregate、event、service 等）分子目录，每个子目录内按业务实体分文件
- 简单服务（仅 1-2 个聚合根）可将子目录退化为单文件（如 `aggregate.rs` 代替 `aggregate/`）
- `command/` 目录仅在有复杂命令模式时创建（参见第 12.1 节）
- `remote/` 目录**独立于 DDD 四层架构**，用于跨微服务 HTTP 远程调用，不同的外部服务接口用不同文件表示

## 4. 领域层 (Domain Layer)

### 4.1 聚合根

使用 `#[derive(Aggregate)]`，包含业务方法和事件生成：

```rust
use genies_derive::Aggregate;
use serde::{Deserialize, Serialize};

#[derive(Aggregate, Debug, Clone, Serialize, Deserialize)]
#[aggregate_type("Device")]
pub struct Device {
    pub id: String,
    pub name: String,
    pub serial_number: String,
    pub status: i32,  // 0=未绑定, 1=已绑定
}

impl Device {
    /// 工厂方法 - 创建设备（类似 Java 聚合根的静态 create 方法）
    pub fn create(name: String, serial_number: String) -> (Self, DeviceCreatedEvent) {
        let id = uuid::Uuid::new_v4().to_string();
        let device = Self { id: id.clone(), name: name.clone(), serial_number, status: 0 };
        let event = DeviceCreatedEvent { id, name, created_at: chrono::Utc::now().timestamp_millis() };
        (device, event)
    }

    /// 业务方法 - 绑定设备
    pub fn bind(&mut self) -> Result<DeviceBindEvent, String> {
        if self.status == 1 {
            return Err("设备已绑定".to_string());
        }
        self.status = 1;
        Ok(DeviceBindEvent { id: self.id.clone() })
    }
}
```

### 4.2 领域事件

使用 `#[derive(DomainEvent)]`：

```rust
use genies_derive::DomainEvent;
use serde::{Deserialize, Serialize};

#[derive(DomainEvent, Debug, Serialize, Deserialize, Default, Clone)]
#[event_type_version("V1")]
#[event_source("com.example.device.domain.Device")]
#[event_type("com.example.device.event.DeviceCreated")]
pub struct DeviceCreatedEvent {
    pub id: String,
    pub name: String,
    pub created_at: i64,
}
```

### 4.3 仓储

使用 RBatis CRUD 宏：

```rust
use rbatis::crud;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DeviceEntity {
    pub id: Option<String>,
    pub name: Option<String>,
    pub serial_number: Option<String>,
    pub status: Option<i32>,
}

crud!(DeviceEntity {}, "device");
```

### 4.4 域服务

封装跨实体的复杂业务逻辑：

```rust
use genies_ddd::DomainEventPublisher::publish;
use rbatis::executor::Executor;

pub struct DeviceDomainService;

impl DeviceDomainService {
    /// 复杂业务逻辑：创建设备并发布事件
    pub async fn create_device(
        tx: &mut dyn Executor,
        device: &Device,
        event: DeviceCreatedEvent,
    ) {
        // 保存聚合根
        DeviceEntity::insert(tx, &device.into()).await.unwrap();
        // 发布领域事件
        publish(tx, device, Box::new(event)).await;
    }
}
```

## 5. 应用层 (Application Layer)

### 5.1 应用服务

编排用例、管理事务：

```rust
use genies::context::CONTEXT;
use genies_ddd::DomainEventPublisher::publish;

pub struct DeviceAppService;

impl DeviceAppService {
    /// 设备绑定用例
    pub async fn bind_device(serial_number: &str) -> Result<String, String> {
        let rb = &CONTEXT.rbatis;
        // 查询设备
        let device = DeviceEntity::select_by_column(rb, "serial_number", serial_number)
            .await.map_err(|e| e.to_string())?;
        if device.is_empty() {
            return Err("设备不存在".to_string());
        }
        // 聚合根执行业务方法
        let mut agg: Device = device[0].clone().into();
        let event = agg.bind()?;
        // 持久化 + 事件发布
        let mut tx = rb.acquire_begin().await.unwrap();
        DeviceEntity::update_by_column(&mut tx, &(&agg).into(), "id").await.unwrap();
        publish(&mut tx, &agg, Box::new(event)).await;
        tx.commit().await.unwrap();
        Ok(agg.id)
    }
}
```

### 5.2 事件消费者

使用 `#[topic]` 宏：

```rust
use genies_derive::topic;
use rbatis::executor::Executor;

#[topic(name = "Device", pubsub = "messagebus")]
pub async fn on_device_created(
    tx: &mut dyn Executor,
    event: DeviceCreatedEvent,
) -> anyhow::Result<u64> {
    // 处理事件：例如创建关联数据、发送通知等
    log::info!("收到设备创建事件: {:?}", event);
    Ok(0)
}
```

### 5.3 DTO

请求/响应数据传输对象：

```rust
use salvo::oapi::ToSchema;
use serde::{Deserialize, Serialize};

/// 请求 DTO — 必须 derive ToSchema 以支持 OpenAPI 文档自动生成
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct DeviceBindRequest {
    pub serial_number: String,
    pub department_id: String,
}

/// 响应 DTO — 必须加 #[casbin] 实现字段级动态权限过滤
/// 同时 derive ToSchema 确保 OpenAPI Schema 正确注册
/// 这是 Genies 独有能力，Java 中无对应机制
use genies_derive::casbin;

#[casbin]
#[derive(Debug, Deserialize, ToSchema)]
pub struct DeviceVO {
    pub id: String,
    pub name: String,
    pub serial_number: String,   // 敏感字段，可被权限策略隐藏
    pub status: i32,
    pub department_id: String,   // 敏感字段，可被权限策略隐藏
}
```

### 5.4 对象字段拷贝：`copy!` 宏（替代 Java BeanUtils.copyProperties）

Java DDD 项目中大量使用 `BeanUtils.copyProperties(source, target)` 在 Entity、DTO、VO 之间进行对象字段拷贝。Genies 框架提供了 `copy!` 宏来完成同样的工作。

#### 语法

```rust
let target: TargetType = copy!(&source, TargetType);
```

`copy!` 宏的原理是通过 serde 序列化/反序列化实现字段拷贝：先将源对象序列化为 JSON 字节，再反序列化为目标类型。因此**同名字段会自动拷贝**。

#### Entity → VO 示例

```rust
use genies::copy;

// 从数据库查出 Entity 后，转为 VO 返回
let entity: DeviceEntity = DeviceEntity::select_by_column(rb, "id", id).await?;
let vo: DeviceVO = copy!(&entity, DeviceVO);
```

#### DTO → Entity 示例

```rust
use genies::copy;

// 接收请求 DTO，转为 Entity 写入数据库
let req: DeviceCreateRequest = req.parse_json().await.unwrap();
let mut entity: DeviceEntity = copy!(&req, DeviceEntity);
entity.id = Some(uuid::Uuid::new_v4().to_string());
entity.created_at = Some(chrono::Utc::now().timestamp_millis());
DeviceEntity::insert(rb, &entity).await.unwrap();
```

#### 与 Java BeanUtils.copyProperties 对比

| 特性 | Java `BeanUtils.copyProperties` | Genies `copy!` 宏 |
|------|-------------------------------|-------------------|
| 同名字段自动拷贝 | 支持 | 支持（基于 serde 序列化名匹配） |
| 字段名映射 | 不支持（需手动赋值） | 通过 `#[serde(rename = "...")]` 或 `rename_all` 控制 |
| 类型自动转换 | 支持（如 `int` → `long`） | 不支持（类型不兼容会 panic） |
| 忽略多余字段 | 自动忽略 | 目标类型需加 `#[serde(default)]` 或字段用 `Option<T>` |
| 性能 | 反射，较慢 | serde 序列化/反序列化，编译期优化 |

#### 注意事项

- **源类型**必须实现 `Serialize`（`#[derive(Serialize)]`）
- **目标类型**必须实现 `Deserialize`（`#[derive(Deserialize)]`）
- 字段类型不兼容时会在运行时 **panic**（不是编译期错误），建议在测试中覆盖
- 如果源和目标的 `#[serde(rename_all = "...")]` 不一致（如一个是 camelCase，一个是 snake_case），同名字段将无法匹配。确保两边的 serde 序列化名一致
- 对于简单的转换场景，手动实现 `From<Source> for Target` 性能更好且更安全；`copy!` 适合字段多、频繁变动的场景

## 6. 接口层 (Interface/API Layer)

### 6.1 Salvo Endpoint

对应 Java 的 `@RestController`。使用 `#[endpoint]` 而非 `#[handler]`，以自动生成 OpenAPI 文档。配合 OpenAPI 提取器（`PathParam`、`QueryParam`、`JsonBody`）可自动生成参数文档：

```rust
use salvo::prelude::*;
use salvo::oapi::extract::*;
use genies::core::RespVO;

/// 写操作 — 使用 JsonBody 提取请求体，自动生成 OpenAPI 请求文档
#[endpoint]
pub async fn bind_device(body: JsonBody<DeviceBindRequest>) -> Json<RespVO<String>> {
    let dto = body.into_inner();
    match DeviceAppService::bind_device(&dto.serial_number).await {
        Ok(id) => Json(RespVO::from(&Ok::<_, String>(id))),
        Err(e) => Json(RespVO::<String>::from_error(&e)),
    }
}

/// 读操作 — 使用 PathParam 提取路径参数
/// 返回 #[casbin] 标记的 VO，Writer 自动过滤敏感字段
#[endpoint]
pub async fn get_device(id: PathParam<String>) -> DeviceVO {
    let device_id = id.into_inner();
    DeviceAppService::get_device(&device_id).await.unwrap()
    // #[casbin] Writer 自动从 Depot 提取 enforcer + subject 并过滤字段
}

/// 列表查询 — 使用 QueryParam 提取分页参数
#[endpoint]
pub async fn list_devices(
    page_index: QueryParam<u64, false>,
    page_size: QueryParam<u64, false>,
) -> Json<Vec<DeviceVO>> {
    let idx = page_index.into_inner().unwrap_or(0);
    let size = page_size.into_inner().unwrap_or(20);
    let devices = DeviceAppService::list_devices(idx, size).await;
    Json(devices)
}
```

> **关键区别**：Java 中 Controller 返回的所有字段对所有角色可见；Genies 中 `#[casbin]` VO 的字段会根据当前请求者的角色动态隐藏，无需额外代码。

### 6.3 分页查询 Endpoint（Spring Data Page 兼容）

当需要与 Java 服务保持分页响应格式完全一致时，使用 `SpringPage` 将 RBatis 分页结果转换为 Spring Data `Page<T>` 兼容格式：

```rust
use salvo::prelude::*;
use salvo::oapi::extract::*;
use genies::core::ResultDTO;
use genies::context::CONTEXT;
use genies_core::page::SpringPage;
use rbatis::plugin::page::PageRequest;

/// 分页查询 — 使用 QueryParam 提取参数，返回 Spring Data Page 兼容格式
#[endpoint]
pub async fn page_search_devices(
    page: QueryParam<u64, false>,
    size: QueryParam<u64, false>,
    name: QueryParam<String, false>,
) -> Json<ResultDTO<SpringPage<DeviceVO>>> {
    let page_val = page.into_inner().unwrap_or(0);
    let size_val = size.into_inner().unwrap_or(20);
    let name_val = name.into_inner().unwrap_or_default();

    let rb = &CONTEXT.rbatis;
    // 转换为 RBatis PageRequest（page_no 从 1 开始，即 page + 1）
    let page_req = PageRequest::new(page_val + 1, size_val);

    // 执行分页查询
    let rbatis_page = DeviceEntity::select_page(
        rb, &page_req, &name_val
    ).await.unwrap();

    // 转换为 SpringPage（同时将 Entity 转为 VO）
    let spring_page: SpringPage<DeviceVO> = SpringPage::from_rbatis_page(rbatis_page, |e| e.into());

    Json(ResultDTO::success("查询成功", spring_page))
}
```

### 6.2 路由配置

```rust
use salvo::prelude::*;
use genies_auth::middleware::casbin_auth;

pub fn device_router() -> Router {
    Router::with_path("device")
        // casbin_auth 中间件：API 级访问控制 + 注入 enforcer/subject 到 Depot
        // 下游 #[casbin] VO 的 Writer 依赖此中间件注入的上下文
        .hoop(casbin_auth())
        .push(Router::with_path("bind").post(bind_device))
        .push(Router::with_path("unbind").post(unbind_device))
        .push(Router::with_path("list").get(list_devices))
        .push(Router::with_path("{id}").get(get_device))
}
```

### 6.4 OpenAPI 集成与 Schema 同步

#### 为什么必须用 `#[endpoint]`

Genies 项目中所有 HTTP handler 必须使用 `#[endpoint]` 而非 `#[handler]`，原因：

1. **OpenAPI 文档自动生成** — `#[endpoint]` 会将函数签名（参数类型、返回类型）自动注册到 OpenAPI spec，`#[handler]` 不会
2. **权限 Schema 注册** — `extract_and_sync_schemas(&doc)` 依赖 OpenAPI doc 中的 Schema 信息，将所有 DTO 字段同步到 `auth_api_schemas` 表，供 `#[casbin]` 字段级权限使用

> `#[endpoint]` 的函数签名与 `#[handler]` 完全相同，迁移只需替换宏名即可。

#### OpenAPI 参数提取方式

使用 `#[endpoint]` 时，推荐使用 OpenAPI 提取器替代手动从 `Request` 提取参数。提取器会自动在 OpenAPI 文档中生成参数描述：

| `#[handler]` 旧写法 | `#[endpoint]` 新写法 | 说明 |
|---------------------|---------------------|------|
| `req.param::<T>("name")` | `name: PathParam<T>` | 路径参数 `/device/{id}` |
| `req.query::<T>("name")` | `name: QueryParam<T, REQUIRED>` | 查询参数 `?page=0&size=20` |
| `req.parse_json::<T>().await` | `body: JsonBody<T>` | JSON 请求体 |
| `res.render(Json(data))` | `-> Json<T>` 返回值 | 响应体（自动生成响应 Schema） |

> `QueryParam<T, false>` 表示可选参数，`QueryParam<T, true>` 表示必填参数。

**迁移示例：**

```rust
// ❌ 旧写法 — 不生成 OpenAPI 参数文档
#[handler]
async fn find_by_id(req: &mut Request, res: &mut Response) {
    let id: String = req.param::<String>("id").unwrap();
    let result = DeviceAppService::get_device(&id).await.unwrap();
    res.render(Json(result));
}

// ✅ 新写法 — 自动生成 OpenAPI 参数 + 响应文档
use salvo::oapi::extract::*;

#[endpoint]
async fn find_by_id(id: PathParam<String>) -> Json<DeviceVO> {
    let result = DeviceAppService::get_device(&id.into_inner()).await.unwrap();
    Json(result)
}
```

#### DTO 必须 derive ToSchema

所有请求/响应 DTO 必须添加 `#[derive(ToSchema)]`，否则 OpenAPI 文档中不会包含其 Schema 定义：

```rust
use salvo::oapi::ToSchema;
use serde::{Deserialize, Serialize};
use genies_derive::casbin;

/// 请求 DTO
#[derive(Debug, Deserialize, ToSchema)]
pub struct DeviceBindRequest {
    pub serial_number: String,
    pub department_id: String,
}

/// 响应 DTO — #[casbin] + ToSchema 联合使用
#[casbin]
#[derive(Debug, Deserialize, ToSchema)]
pub struct DeviceVO {
    pub id: String,
    pub name: String,
    pub serial_number: String,
    pub status: i32,
}
```

#### Schema 同步步骤

在 `main()` 中构建路由后，调用 `extract_and_sync_schemas` 将 OpenAPI Schema 同步到权限系统数据库：

```rust
use genies_auth::extract_and_sync_schemas;
use salvo::oapi::OpenApi;

// 1. 构建业务路由
let router = Router::new()
    .push(Router::with_path("api/v1")
        .push(device_router()));

// 2. 生成 OpenAPI 文档
let doc = OpenApi::new("my-service", "1.0.0").merge_router(&router);

// 3. 同步 Schema 到数据库（auth_api_schemas 表）
extract_and_sync_schemas(&doc).await.ok();

// 4. 挂载 Enforcer 中间件（在 Schema 同步之后）
let mgr = Arc::new(EnforcerManager::new().await.unwrap());
let router = router
    .hoop(genies::context::auth::salvo_auth)
    .hoop(affix_state::inject(mgr.clone()))
    .hoop(casbin_auth);
```

#### 与 `#[casbin]` 字段级权限的配合

完整流程：

1. DTO 添加 `#[derive(ToSchema)]` → OpenAPI 文档包含其字段定义
2. `extract_and_sync_schemas(&doc)` → 字段信息写入 `auth_api_schemas` 表
3. Admin UI 中基于 Schema 配置字段级 deny 策略
4. 响应 DTO 添加 `#[casbin]` → Writer 层根据策略自动过滤字段

## 7. 基础设施层 (Infrastructure Layer)

### 7.1 Flyway 迁移配置

```rust
use flyway::MigrationRunner;
use flyway_rbatis::RbatisMigrationDriver;
use std::sync::Arc;

#[flyway::migrations("migrations")]
pub struct Migrations {}

pub async fn run_migrations() {
    let rbatis = Arc::new(CONTEXT.rbatis.clone());
    let driver = Arc::new(RbatisMigrationDriver::new(rbatis, None));
    let runner = MigrationRunner::new(
        Migrations {},
        driver.clone(),
        driver.clone(),
        false,
    );
    runner.migrate().await.expect("数据库迁移失败");
}
```

### 7.2 SQL 迁移文件

命名规则：`V<版本号>__<描述>.sql`（双下划线）

```sql
-- V1__create_device.sql
CREATE TABLE IF NOT EXISTS device (
    id VARCHAR(36) PRIMARY KEY,
    name VARCHAR(255) NOT NULL,
    serial_number VARCHAR(100) UNIQUE NOT NULL,
    status INT DEFAULT 0,
    created_at BIGINT,
    updated_at BIGINT
);

-- 消息表（领域事件 Outbox 模式必需）
CREATE TABLE IF NOT EXISTS message (
    id VARCHAR(36) PRIMARY KEY,
    destination VARCHAR(255),
    headers TEXT,
    payload TEXT NOT NULL,
    published INT DEFAULT 0,
    creation_time BIGINT
);
```

SQL 注解语法：`--! may_fail: true`（`--!` 前缀 + YAML 格式）

## 8. 远程服务调用层 (Remote Layer)

远程调用模块**独立于 DDD 四层架构**，用于跨微服务的 HTTP 远程调用（类似 Java 的 FeignClient）。使用 `#[remote]` 属性宏实现声明式 HTTP 调用，自动管理 Keycloak Token。

### 8.1 目录结构

```
src/remote/
├── mod.rs                  # 模块导出
├── patient_service.rs      # 患者服务远程调用
├── baseinfo_service.rs     # 基础信息服务远程调用
└── his_service.rs          # HIS 系统远程调用
```

在 `lib.rs` 中声明 `pub mod remote;`。

### 8.2 基本用法

```rust
use once_cell::sync::Lazy;
use genies_derive::remote;
use serde::{Deserialize, Serialize};

/// 使用 config_gateway! 宏定义外部服务的基础 URL（从 application.yml 的 gateway 配置读取）
pub static BaseInfo: Lazy<String> = genies::config_gateway!("/baseinfo");
pub static Patient: Lazy<String> = genies::config_gateway!("/patient");

/// 远程调用返回的数据模型
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct CustomConfigModel {
    pub id: Option<String>,
    pub name: Option<String>,
}

/// 查询参数示例
#[remote]
#[get(url = BaseInfo, path = "/customconfig/depttypename")]
pub async fn findByDepartmentIdAndTypeName(
    #[query] departmentId: &str,
    #[query] typeName: &str,
) -> feignhttp::Result<Vec<CustomConfigModel>> { impled!() }

/// 路径参数示例
#[remote]
#[get(url = Patient, path = "/api/patient/id/{id}")]
pub async fn get_patient_by_id(
    #[path] id: &str,
) -> feignhttp::Result<PatientInfo> { impled!() }

/// POST 请求体示例
#[remote]
#[post(url = Patient, path = "/api/patient/create")]
pub async fn create_patient(
    #[body] patient: PatientCreateDTO,
) -> feignhttp::Result<String> { impled!() }
```

### 8.3 关键要点

- **`config_gateway!` 宏**：`genies::config_gateway!("/service-prefix")` 生成 `Lazy<String>`，值为 `${gateway}/service-prefix`，gateway 从 application.yml 配置读取
- **`url` + `path` 分离**：`url` 引用 `Lazy<String>` 静态变量（服务基础路径），`path` 是具体端点路径
- **参数注解**：`#[query]` 查询参数、`#[path]` 路径参数、`#[body]` 请求体
- **函数体**：必须写 `impled!()`（feignhttp 宏要求）
- **参数类型**：用 `&str` 而非 `String`
- **自动 Token 管理**：`#[remote]` 自动从 `REMOTE_TOKEN` 获取 Bearer token，遇 401 自动刷新 Keycloak token 并重试
- **生成两个函数**：`{func_name}_feignhttp`（带 Authorization 参数的原始版本）和 `{func_name}`（自动注入 token 的包装版本）

### 8.4 与 Java FeignClient 的对照

| Java FeignClient | Rust/Genies `#[remote]` |
|------------------|------------------------|
| `@FeignClient(name = "patient-service")` | `pub static Patient: Lazy<String> = genies::config_gateway!("/patient");` |
| `@GetMapping("/api/patient/{id}")` | `#[get(url = Patient, path = "/api/patient/{id}")]` |
| `@RequestParam String name` | `#[query] name: &str` |
| `@PathVariable String id` | `#[path] id: &str` |
| `@RequestBody UserDTO body` | `#[body] body: UserDTO` |
| Spring Security OAuth2 Token 自动传递 | `#[remote]` 自动管理 Keycloak Token（401 自动刷新重试） |

## 9. 微服务启动入口

```rust
use salvo::prelude::*;
use genies::context::CONTEXT;
use genies_auth::{EnforcerManager, casbin_auth, extract_and_sync_schemas};

#[tokio::main]
async fn main() {
    // 1. 初始化配置和上下文
    CONTEXT.init_default().await;

    // 2. 运行数据库迁移
    my_service::infrastructure::migration::run_migrations().await;

    // 3. 构建路由
    let router = Router::new()
        .push(Router::with_path("api/v1")
            .push(my_service::interfaces::router::device_router())
        );

    // 4. 生成 OpenAPI 文档并同步 Schema 到权限系统
    let doc = OpenApi::new("my-service", "1.0.0").merge_router(&router);
    extract_and_sync_schemas(&doc).await.ok();

    // 5. Dapr 订阅路由（自动收集 #[topic] 注册的处理器）
    let router = router.push(genies::dapr::dapr_router());

    // 6. 启动服务
    let acceptor = TcpListener::new("0.0.0.0:8080").bind().await;
    Server::new(acceptor).serve(router).await;
}
```

## 10. Cargo.toml 模板

```toml
[package]
name = "my_service"
version = "0.1.0"
edition = "2021"

[dependencies]
genies = { workspace = true }
genies_derive = { workspace = true }
genies_ddd = { workspace = true }
genies_core = { workspace = true }
genies_config = { workspace = true }
genies_context = { workspace = true }
genies_cache = { workspace = true }
genies_dapr = { workspace = true }
genies_auth = { workspace = true }
flyway = { workspace = true }
flyway-rbatis = { workspace = true }
rbatis = { workspace = true }
salvo = { workspace = true, features = ["oapi"] }
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
tokio = { version = "1", features = ["full"] }
chrono = "0.4"
uuid = { version = "1", features = ["v4"] }
log = "0.4"
anyhow = "1.0"
```

## 11. Java DDD → Rust/Genies 完整对照表

| Java 模式 | Rust/Genies 实现 |
|----------|-----------------|
| `@Entity` 聚合根 | `#[derive(Aggregate)]` + 业务方法 |
| `ResultWithDomainEvents<E, Evt>` | 返回 `(Entity, Event)` 元组 |
| `AbstractAggregateDomainEventPublisher` | `DomainEventPublisher::publish()` |
| `DomainEventHandlersBuilder.forAggregateType()` | `#[topic(name = "AggType")]` |
| `@Service` 应用服务 | `pub struct XxxAppService` + `async fn` |
| `@Service` 域服务 | `pub struct XxxDomainService` + `async fn` |
| `@RestController` | `#[endpoint]` + `Router`（自动生成 OpenAPI 文档） |
| `@Autowired Repository` | `CONTEXT.rbatis` + RBatis CRUD 宏 |
| `Spring @Transactional` | `rb.acquire_begin()` ... `tx.commit()` |
| `ResultDTO<T>` | `RespVO<T>` |
| `@RequestBody` | `body: JsonBody<T>`（OpenAPI 提取器） |
| `@RequestParam` | `name: QueryParam<T, REQUIRED>`（OpenAPI 提取器） |
| `@PathVariable` | `id: PathParam<T>`（OpenAPI 提取器） |
| `Pageable / Page<T>` | `PageRequest::new(page_no, page_size)` + RBatis `select_page` |
| `JpaSpecificationExecutor` | RBatis `#[py_sql]` 动态 SQL |
| `DomainEventDispatcher` + `ConsumersConfig` | `#[topic]` 宏自动注册 |
| `Flyway SQL 迁移` | `flyway-rs #[migrations("path")]` (v0.5.0) |
| `@Configuration` Bean 注册 | `genies_config::ApplicationConfig` |
| `application.yml` | `application.yml` + `genies_config` |
| `Keycloak SSO` | `genies_auth` Casbin 中间件 |
| `BeanUtils.copyProperties(src, dest)` | `copy!(&src, DestType)` 宏（基于 serde 序列化） |
| `@ApiModelProperty` | `#[derive(ToSchema)]` (Salvo OpenAPI) |
| `@FeignClient` + `@GetMapping/@PostMapping` | `#[remote]` + `#[get(url=..., path=...)]` 声明式远程调用 |
| **Java 无对应** | `#[casbin]` 响应 DTO 字段级动态权限过滤 |
| **Java 无对应** | `casbin_auth()` 中间件 API 级访问控制 |

## 12. 进阶 DDD 模式

以下模式来自复杂业务领域（如医嘱管理），适用于聚合根数量多、业务规则复杂、跨服务交互频繁的场景。

### 12.1 命令模式 (Command Pattern)

复杂业务操作应封装为独立的命令对象，而非直接传递 Model：

```rust
/// 命令 trait（可选，用于统一约束）
pub trait Command: Send + Sync {}

/// 血袋暂停命令
#[derive(Debug, Serialize, Deserialize)]
pub struct SuspendBloodBagCommand {
    pub blood_bag_number: String,
    pub reason: String,
    pub operator_id: String,      // 继承自 UserOperationModel 模式
    pub operator_name: String,
}

impl Command for SuspendBloodBagCommand {}

/// 批量操作命令
#[derive(Debug, Serialize, Deserialize)]
pub struct BatchDeleteWorkItemCommand {
    pub ids: Vec<String>,
}

/// 聚合根接收命令执行业务逻辑
impl BloodBag {
    pub fn suspend(&mut self, cmd: SuspendBloodBagCommand) -> Result<BloodBagSuspendedEvent, String> {
        if self.status != BloodBagStatus::InUse {
            return Err("只有使用中的血袋可以暂停".to_string());
        }
        self.status = BloodBagStatus::Suspended;
        Ok(BloodBagSuspendedEvent {
            id: self.id.clone(),
            reason: cmd.reason,
        })
    }
}
```

### 12.2 多类型聚合根加载器

当一个领域有多种聚合根子类型时（如药物医嘱、检查医嘱、手术医嘱），使用统一加载器：

```rust
/// 医嘱类型枚举
pub enum DoctorAdviceType {
    Drug,       // 药物医嘱
    Examine,    // 检查医嘱
    Operation,  // 手术医嘱
    Blood,      // 输血医嘱
    Medical,    // 医疗医嘱
}

/// 聚合根加载器 — 类似 Java 中的 AggregateRepository
pub struct DoctorAdviceAggregateLoader;

impl DoctorAdviceAggregateLoader {
    pub async fn load(id: &str, advice_type: DoctorAdviceType) -> Result<Box<dyn DoctorAdvice>, String> {
        let rb = &CONTEXT.rbatis;
        match advice_type {
            DoctorAdviceType::Drug => {
                let entity = DrugAdviceEntity::select_by_column(rb, "id", id).await
                    .map_err(|e| e.to_string())?;
                Ok(Box::new(entity.into_iter().next().ok_or("未找到")?))
            }
            DoctorAdviceType::Examine => {
                let entity = ExamineAdviceEntity::select_by_column(rb, "id", id).await
                    .map_err(|e| e.to_string())?;
                Ok(Box::new(entity.into_iter().next().ok_or("未找到")?))
            }
            // ... 其他类型
        }
    }
}
```

### 12.3 事件分层体系

复杂领域应建立事件继承层次，便于消费者按粒度订阅：

```rust
/// 事件基类 — 使用 enum 实现 Java 中的事件继承
#[derive(DomainEvent, Debug, Serialize, Deserialize, Clone)]
#[event_type_version("V1")]
#[event_source("com.example.doctoradvice")]
pub enum DoctorAdviceEvent {
    /// 医嘱生命周期事件
    #[event_type("DoctorAdvice.Created")]
    Created(DoctorAdviceCreatedPayload),
    #[event_type("DoctorAdvice.Verified")]
    Verified(DoctorAdviceVerifiedPayload),
    #[event_type("DoctorAdvice.Stopped")]
    Stopped(DoctorAdviceStoppedPayload),
    #[event_type("DoctorAdvice.Discarded")]
    Discarded(DoctorAdviceDiscardedPayload),
}

/// 工作流事件（Saga 触发器）
#[derive(DomainEvent, Debug, Serialize, Deserialize, Default, Clone)]
#[event_type_version("V1")]
#[event_source("com.example.doctoradvice")]
#[event_type("DoctorAdvice.WorkItemSagaStarted")]
pub struct WorkItemSagaStartedEvent {
    pub advice_id: String,
    pub work_items: Vec<WorkItemPayload>,
    pub cycle_begin_time: i64,
}
```

### 12.4 跨聚合根事件消费

一个消费者可以监听来自多个不同聚合根的事件（对应 Java 中 `andForAggregateType` 模式）：

```rust
/// 在 Genies 中，每个 #[topic] 订阅一个聚合根的 Topic
/// 跨聚合根消费需要注册多个 handler

// 监听 WorkItem 聚合根的事件
#[topic(name = "WorkItem", pubsub = "messagebus")]
pub async fn on_work_item_executed(
    tx: &mut dyn Executor,
    event: WorkItemExecutedEvent,
) -> anyhow::Result<u64> {
    // 更新血袋状态
    BloodBagDomainService::handle_work_item_executed(tx, &event).await?;
    Ok(0)
}

// 同一个模块也监听 BloodBag 聚合根的事件
#[topic(name = "BloodBag", pubsub = "messagebus")]
pub async fn on_blood_bag_audited(
    tx: &mut dyn Executor,
    event: BloodBagAuditedEvent,
) -> anyhow::Result<u64> {
    BloodBagDomainService::handle_audited(tx, &event).await?;
    Ok(0)
}
```

### 12.5 域服务编排模式

复杂业务领域的域服务应按层次组织，主服务聚合子服务：

```rust
/// 主域服务 — 聚合所有医嘱类型的子服务
pub struct DoctorAdviceDomainService;

impl DoctorAdviceDomainService {
    /// 根据类型分派到子服务处理
    pub async fn create_advice(
        tx: &mut dyn Executor,
        advice_type: DoctorAdviceType,
        cmd: CreateAdviceCommand,
    ) -> Result<String, String> {
        match advice_type {
            DoctorAdviceType::Drug => DrugAdviceDomainService::create(tx, cmd).await,
            DoctorAdviceType::Examine => ExamineAdviceDomainService::create(tx, cmd).await,
            DoctorAdviceType::Operation => OperationAdviceDomainService::create(tx, cmd).await,
            // ...
        }
    }
}

/// 子域服务 — 专注于特定医嘱类型的业务逻辑
pub struct DrugAdviceDomainService;

impl DrugAdviceDomainService {
    pub async fn create(tx: &mut dyn Executor, cmd: CreateAdviceCommand) -> Result<String, String> {
        let (advice, event) = DrugAdvice::create(cmd);
        DrugAdviceEntity::insert(tx, &(&advice).into()).await.map_err(|e| e.to_string())?;
        publish(tx, &advice, Box::new(event)).await;
        Ok(advice.id)
    }
}
```

### 12.6 聚合根生成多事件

复杂业务操作可能需要在一次聚合根方法调用中生成多个事件：

```rust
impl DoctorAdvice {
    /// 医嘱分解 — 生成多个工作项事件（类似 Java 中返回 List<Event>）
    pub fn explode(&mut self, schedule: &[i64]) -> Result<Vec<Box<dyn DomainEvent>>, String> {
        if self.status != AdviceStatus::Verified {
            return Err("只有已审核的医嘱可以分解".to_string());
        }
        self.status = AdviceStatus::Exploded;

        let mut events: Vec<Box<dyn DomainEvent>> = Vec::new();
        for &time in schedule {
            events.push(Box::new(WorkItemSagaStartedEvent {
                advice_id: self.id.clone(),
                work_items: vec![],
                cycle_begin_time: time,
            }));
        }
        Ok(events)
    }
}

/// 发布多个事件
pub async fn publish_multiple(
    tx: &mut dyn Executor,
    aggregate: &impl AggregateType,
    events: Vec<Box<dyn DomainEvent>>,
) {
    for event in events {
        publish(tx, aggregate, event).await;
    }
}
```

### 12.7 查询仓储与聚合仓储分离

对应 Java 中 QueryRepository 和 AggregateRepository 的分离模式：

```rust
/// 查询仓储 — 只读查询，支持复杂条件（对应 Java QueryRepository）
impl DrugAdviceEntity {
    #[py_sql("
        SELECT * FROM drug_advice
        WHERE patient_id = #{patient_id}
        if status != null:
            AND status = #{status}
        if dept_id != '':
            AND department_id = #{dept_id}
        ORDER BY created_at DESC
    ")]
    pub async fn query_by_patient(
        rb: &dyn Executor,
        patient_id: &str,
        status: Option<i32>,
        dept_id: &str,
    ) -> rbatis::Result<Vec<DrugAdviceEntity>> { impled!() }
}

/// 聚合仓储 — 用于加载和保存聚合根，封装复杂的加载逻辑
/// 查询仓储方法用于列表、搜索等读操作
/// 聚合仓储方法用于业务操作前的聚合根加载
```

## 13. 最佳实践

- **聚合根**应包含业务规则验证，工厂方法返回 `(Entity, Event)` 元组
- **域服务**处理跨实体逻辑，不直接暴露给接口层
- **应用服务**编排用例，管理事务边界（`acquire_begin` / `commit`）
- **Handler** 只做参数解析和响应格式化，不含业务逻辑；必须使用 `#[endpoint]` 以生成 OpenAPI 文档
- **所有请求/响应 DTO 必须 `#[derive(ToSchema)]`** — 确保 OpenAPI Schema 正确注册
- **所有响应 DTO 必须加 `#[casbin]`** — 实现字段级动态权限过滤（Genies 独有能力）
- **路由必须挂载 `casbin_auth()` 中间件** — 提供 API 级访问控制，并为 `#[casbin]` 注入 enforcer/subject
- 每个聚合根对应独立的 `migrations/` 目录
- 使用 `RespVO<T>` 统一 HTTP 响应格式
- 事件消费者使用 `#[topic]` 宏，自动幂等（基于 Redis）
- 消息表 (`message`) 采用 Outbox 模式，由 Dapr CDC 异步投递
- 参考现有 `crates/auth/` 作为完整业务模块示范
- SQL 迁移文件使用 `V<N>__<desc>.sql` 双下划线命名

## 14. Related Skills

- **ddd-usage** — DDD 核心 API（聚合根、领域事件、事件发布）
- **derive-usage** — 过程宏详解（Aggregate、DomainEvent、topic、casbin 等）
- **flyway-usage** — 数据库迁移（flyway-rs v0.5.0 用法）
- **rbatis-usage** — ORM 操作（CRUD 宏、动态 SQL、分页）
- **dapr-usage** — 事件驱动（Dapr PubSub、CloudEvent、CDC）
- **core-usage** — 响应模型（RespVO、错误处理、JWT）
- **auth-usage** — 权限管理（Casbin 中间件、字段级过滤、Admin UI）
- **context-usage** — 全局上下文（CONTEXT、RBATIS、缓存）
- **config-usage** — 配置管理（ApplicationConfig、YAML + ENV 加载）
