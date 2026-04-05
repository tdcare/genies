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
| 接口层 | Salvo `#[handler]` + `Router` 配置 |
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
│   └── infrastructure/                 # 基础设施层
│       ├── mod.rs
│       └── migration.rs                # Flyway 迁移配置
```

**目录划分原则：**
- `domain/` 下按职责（aggregate、event、service 等）分子目录，每个子目录内按业务实体分文件
- 简单服务（仅 1-2 个聚合根）可将子目录退化为单文件（如 `aggregate.rs` 代替 `aggregate/`）
- `command/` 目录仅在有复杂命令模式时创建（参见第 11.1 节）

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

/// 请求 DTO — 普通 Deserialize 即可
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct DeviceBindRequest {
    pub serial_number: String,
    pub department_id: String,
}

/// 响应 DTO — 必须加 #[casbin] 实现字段级动态权限过滤
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

### 6.1 Salvo Handler

对应 Java 的 `@RestController`：

```rust
use salvo::prelude::*;
use genies::core::RespVO;

/// 写操作 — 返回 RespVO（无需字段过滤）
#[handler]
pub async fn bind_device(req: &mut Request, res: &mut Response) {
    let dto: DeviceBindRequest = req.parse_json().await.unwrap();
    match DeviceAppService::bind_device(&dto.serial_number).await {
        Ok(id) => { res.render(Json(RespVO::from(&Ok::<_, String>(id)))); }
        Err(e) => { res.render(Json(RespVO::<String>::from_error(&e))); }
    }
}

/// 读操作 — 返回 #[casbin] 标记的 VO，自动过滤敏感字段
/// #[casbin] 生成的 Writer trait 会从 Depot 提取 enforcer + subject，
/// 根据 Casbin 策略动态隐藏 deny 的字段后再序列化为 JSON
#[handler]
pub async fn get_device(req: &mut Request, depot: &mut Depot, res: &mut Response) {
    let id: String = req.param("id").unwrap();
    match DeviceAppService::get_device(&id).await {
        Ok(vo) => { res.render(vo); }   // 直接 render，#[casbin] Writer 自动过滤字段
        Err(e) => { res.render(Json(RespVO::<String>::from_error(&e))); }
    }
}

/// 列表查询 — Vec<T> 中每个元素都会自动过滤
#[handler]
pub async fn list_devices(req: &mut Request, depot: &mut Depot, res: &mut Response) {
    let page_index: u64 = req.query("page_index").unwrap_or(0);
    let page_size: u64 = req.query("page_size").unwrap_or(20);
    let devices: Vec<DeviceVO> = DeviceAppService::list_devices(page_index, page_size).await;
    res.render(Json(devices));  // Vec<#[casbin]> 每个元素独立过滤
}
```

> **关键区别**：Java 中 Controller 返回的所有字段对所有角色可见；Genies 中 `#[casbin]` VO 的字段会根据当前请求者的角色动态隐藏，无需额外代码。

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

## 8. 微服务启动入口

```rust
use salvo::prelude::*;
use genies::context::CONTEXT;

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

    // 4. Dapr 订阅路由（自动收集 #[topic] 注册的处理器）
    let router = router.push(genies::dapr::dapr_router());

    // 5. 启动服务
    let acceptor = TcpListener::new("0.0.0.0:8080").bind().await;
    Server::new(acceptor).serve(router).await;
}
```

## 9. Cargo.toml 模板

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

## 10. Java DDD → Rust/Genies 完整对照表

| Java 模式 | Rust/Genies 实现 |
|----------|-----------------|
| `@Entity` 聚合根 | `#[derive(Aggregate)]` + 业务方法 |
| `ResultWithDomainEvents<E, Evt>` | 返回 `(Entity, Event)` 元组 |
| `AbstractAggregateDomainEventPublisher` | `DomainEventPublisher::publish()` |
| `DomainEventHandlersBuilder.forAggregateType()` | `#[topic(name = "AggType")]` |
| `@Service` 应用服务 | `pub struct XxxAppService` + `async fn` |
| `@Service` 域服务 | `pub struct XxxDomainService` + `async fn` |
| `@RestController` | `#[handler]` + `Router` |
| `@Autowired Repository` | `CONTEXT.rbatis` + RBatis CRUD 宏 |
| `Spring @Transactional` | `rb.acquire_begin()` ... `tx.commit()` |
| `ResultDTO<T>` | `RespVO<T>` |
| `@RequestBody` | `req.parse_json::<T>()` |
| `@RequestParam` | `req.query::<T>("key")` |
| `Pageable / Page<T>` | `PageRequest::new(page_no, page_size)` + RBatis `select_page` |
| `JpaSpecificationExecutor` | RBatis `#[py_sql]` 动态 SQL |
| `DomainEventDispatcher` + `ConsumersConfig` | `#[topic]` 宏自动注册 |
| `Flyway SQL 迁移` | `flyway-rs #[migrations("path")]` (v0.5.0) |
| `@Configuration` Bean 注册 | `genies_config::ApplicationConfig` |
| `application.yml` | `application.yml` + `genies_config` |
| `Keycloak SSO` | `genies_auth` Casbin 中间件 |
| `BeanUtils.copyProperties(src, dest)` | `copy!(&src, DestType)` 宏（基于 serde 序列化） |
| `@ApiModelProperty` | `#[derive(ToSchema)]` (Salvo OpenAPI) |
| **Java 无对应** | `#[casbin]` 响应 DTO 字段级动态权限过滤 |
| **Java 无对应** | `casbin_auth()` 中间件 API 级访问控制 |

## 11. 进阶 DDD 模式

以下模式来自复杂业务领域（如医嘱管理），适用于聚合根数量多、业务规则复杂、跨服务交互频繁的场景。

### 11.1 命令模式 (Command Pattern)

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

### 11.2 多类型聚合根加载器

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

### 11.3 事件分层体系

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

### 11.4 跨聚合根事件消费

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

### 11.5 域服务编排模式

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

### 11.6 聚合根生成多事件

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

### 11.7 查询仓储与聚合仓储分离

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

## 12. 最佳实践

- **聚合根**应包含业务规则验证，工厂方法返回 `(Entity, Event)` 元组
- **域服务**处理跨实体逻辑，不直接暴露给接口层
- **应用服务**编排用例，管理事务边界（`acquire_begin` / `commit`）
- **Handler** 只做参数解析和响应格式化，不含业务逻辑
- **所有响应 DTO 必须加 `#[casbin]`** — 实现字段级动态权限过滤（Genies 独有能力）
- **路由必须挂载 `casbin_auth()` 中间件** — 提供 API 级访问控制，并为 `#[casbin]` 注入 enforcer/subject
- 每个聚合根对应独立的 `migrations/` 目录
- 使用 `RespVO<T>` 统一 HTTP 响应格式
- 事件消费者使用 `#[topic]` 宏，自动幂等（基于 Redis）
- 消息表 (`message`) 采用 Outbox 模式，由 Dapr CDC 异步投递
- 参考现有 `crates/auth/` 作为完整业务模块示范
- SQL 迁移文件使用 `V<N>__<desc>.sql` 双下划线命名

## 13. Related Skills

- **ddd-usage** — DDD 核心 API（聚合根、领域事件、事件发布）
- **derive-usage** — 过程宏详解（Aggregate、DomainEvent、topic、casbin 等）
- **flyway-usage** — 数据库迁移（flyway-rs v0.5.0 用法）
- **rbatis-usage** — ORM 操作（CRUD 宏、动态 SQL、分页）
- **dapr-usage** — 事件驱动（Dapr PubSub、CloudEvent、CDC）
- **core-usage** — 响应模型（RespVO、错误处理、JWT）
- **auth-usage** — 权限管理（Casbin 中间件、字段级过滤、Admin UI）
- **context-usage** — 全局上下文（CONTEXT、RBATIS、缓存）
- **config-usage** — 配置管理（ApplicationConfig、YAML + ENV 加载）
