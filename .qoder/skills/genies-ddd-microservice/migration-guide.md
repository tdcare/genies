# Java DDD 微服务 → Rust/Genies DDD 微服务迁移指南

## 1. 迁移总览

### 1.1 目标

- **REST 接口与 Java 100% 兼容**：路径、HTTP 方法、请求体、响应体完全一致，一字不差
- **共用同一个数据库和 Redis**：Rust 服务直接连接 Java 服务正在使用的 MySQL 和 Redis
- **兼容性优先于代码风格**：迁移的目标不是写出"好的 Rust 代码"，而是写出"与 Java 接口完全一致的 Rust 代码"

### 1.2 迁移原则

- **逐接口迁移**：每次只迁移一个接口，不批量迁移
- **逐接口测试**：每迁移一个接口，立即进行 Java vs Rust 对比测试
- **渐进式切换**：通过网关逐步将流量从 Java 切换到 Rust

### 1.3 迁移顺序

```
1. 基础设施层（配置、数据库连接、迁移脚本复用）
2. 领域层（Entity → Aggregate + RBatis Entity）
3. 应用层（Service → AppService、DTO 定义）
4. 接口层（Controller → Handler + Router）
5. 对比测试（Java vs Rust 响应对比）
6. 切换流量
```

---

## 2. 环境准备

### 2.1 Rust 项目结构

参考 SKILL.md 的标准目录结构：

```
crates/my_service/
├── Cargo.toml
├── migrations/                         # 直接复用 Java 的 Flyway SQL
├── src/
│   ├── lib.rs
│   ├── main.rs                        # 启动入口
│   ├── domain/
│   │   ├── mod.rs
│   │   ├── aggregate/                 # 聚合根
│   │   ├── entity/                    # RBatis 持久化实体
│   │   ├── repository/               # 仓储（py_sql 动态查询）
│   │   └── event/                    # 领域事件
│   ├── application/
│   │   ├── mod.rs
│   │   ├── service.rs                # 应用服务
│   │   ├── dto.rs                    # 请求/响应 DTO
│   │   └── consumer/                 # 事件消费者
│   └── interfaces/
│       ├── mod.rs
│       ├── handler/                  # Salvo HTTP handlers
│       └── router.rs                # 路由配置
```

### 2.2 关键配置：application.yml

数据库和 Redis 必须指向与 Java 相同的实例：

```yaml
# application.yml
server:
  name: my-service
  url: "0.0.0.0:8081"        # 注意：开发阶段用不同端口，避免与 Java 冲突

database:
  url: "mysql://root:password@localhost:3306/my_database"  # 与 Java 完全相同的数据库

redis:
  url: "redis://localhost:6379"    # 与 Java 完全相同的 Redis
```

### 2.3 Cargo.toml 依赖配置

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
rbs = { workspace = true }
salvo = { workspace = true, features = ["oapi"] }
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
tokio = { version = "1", features = ["full"] }
chrono = "0.4"
uuid = { version = "1", features = ["v4"] }      # 可移除，已由 genies::next_id() 替代
log = "0.4"
anyhow = "1.0"
```

---

## 3. JSON 字段名兼容性（最关键！）

### 3.1 核心问题

Java 使用 camelCase 命名（`serialNumber`、`departmentId`、`createdAt`），Rust 默认使用 snake_case（`serial_number`、`department_id`、`created_at`）。

**如果不处理，JSON 输出将完全不兼容！**

### 3.2 请求 DTO 的反序列化（接收 camelCase）

```java
// Java 请求 DTO
public class DeviceCreateRequest {
    private String serialNumber;
    private String departmentId;
    private String deviceName;
}
// Java 接收的 JSON: {"serialNumber": "SN001", "departmentId": "D001", "deviceName": "设备A"}
```

```rust
// Rust 请求 DTO — 必须加 #[serde(rename_all = "camelCase")]
#[derive(Debug, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]   // ← 关键！接收 camelCase JSON
pub struct DeviceCreateRequest {
    pub serial_number: String,       // 自动从 "serialNumber" 反序列化
    pub department_id: String,       // 自动从 "departmentId" 反序列化
    pub device_name: String,         // 自动从 "deviceName" 反序列化
}
```

### 3.3 响应 VO 的序列化（输出 camelCase）

```java
// Java 响应 VO
public class DeviceVO {
    private Long id;
    private String serialNumber;
    private String departmentId;
    private String createdAt;
}
// Java 输出的 JSON: {"id": 123, "serialNumber": "SN001", "departmentId": "D001", "createdAt": "2024-01-01"}
```

```rust
// Rust 响应 VO — 必须加 #[serde(rename_all = "camelCase")]
#[derive(Debug, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]   // ← 关键！输出 camelCase JSON
pub struct DeviceVO {
    pub id: i64,                     // 序列化为 "id"（无变化）
    pub serial_number: String,       // 序列化为 "serialNumber"
    pub department_id: String,       // 序列化为 "departmentId"
    pub created_at: String,          // 序列化为 "createdAt"
}
```

### 3.4 ORM Entity 的字段映射（数据库 snake_case → JSON camelCase）

RBatis Entity 的字段名默认对应数据库列名（snake_case），但 JSON 输出需要 camelCase。

**方案：ORM Entity 和 VO 分离**

```rust
// ORM Entity — 字段名与数据库列名一致（snake_case），不加 rename
#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub struct DeviceEntity {
    pub id: Option<i64>,
    pub serial_number: Option<String>,
    pub department_id: Option<String>,
    pub device_name: Option<String>,
    pub created_at: Option<String>,
}
crud!(DeviceEntity {}, "device");

// 响应 VO — 加 rename_all，用于 JSON 输出
#[derive(Debug, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct DeviceVO {
    pub id: i64,
    pub serial_number: String,
    pub department_id: String,
    pub device_name: String,
    pub created_at: String,
}

// Entity → VO 转换
impl From<DeviceEntity> for DeviceVO {
    fn from(e: DeviceEntity) -> Self {
        Self {
            id: e.id.unwrap_or_default(),
            serial_number: e.serial_number.unwrap_or_default(),
            department_id: e.department_id.unwrap_or_default(),
            device_name: e.device_name.unwrap_or_default(),
            created_at: e.created_at.unwrap_or_default(),
        }
    }
}
```

### 3.5 特殊字段名处理

当 Java 字段名不符合标准 camelCase 规则时，使用 `#[serde(rename = "...")]` 单独指定：

```rust
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SpecialVO {
    pub id: i64,
    pub serial_number: String,          // → "serialNumber" ✓

    #[serde(rename = "IP")]             // Java 字段名就是 "IP"，不是 camelCase
    pub ip: String,

    #[serde(rename = "isOK")]           // Java 字段名是 "isOK"
    pub is_ok: bool,

    #[serde(rename = "HTMLContent")]    // Java 字段名是 "HTMLContent"
    pub html_content: String,
}
```

---

## 4. 响应格式精确复现

Java 项目有多种返回模式，Rust 必须逐一精确匹配。

### 模式 A：ResultDTO 包装

```java
// Java: POST /device/create → ResultDTO<String>
@PostMapping("/device/create")
public ResultDTO<String> createDevice(@RequestBody DeviceCreateRequest req) {
    String id = deviceService.create(req);
    return ResultDTO.success("创建成功", id);
}
// 响应 JSON:
// {"status": 1, "message": "创建成功", "data": "123"}
```

```rust
// Rust: 使用 genies_core::ResultDTO 直接兼容
use genies_core::ResultDTO;

#[handler]
pub async fn create_device(req: &mut Request, res: &mut Response) {
    let dto: DeviceCreateRequest = req.parse_json().await.unwrap();
    match DeviceAppService::create(&dto).await {
        Ok(id) => {
            // ResultDTO { status: Some(1), message: Some("创建成功"), data: Some("123") }
            res.render(Json(ResultDTO::success("创建成功", id)));
        }
        Err(e) => {
            // ResultDTO { status: Some(0), message: Some("错误信息"), data: None }
            res.render(Json(ResultDTO::<String>::error(&e.to_string())));
        }
    }
}
// 响应 JSON: {"status": 1, "message": "创建成功", "data": "123"} ← 与 Java 完全一致
```

### 模式 B：直接返回 Entity（不用 ResultDTO 包装）

```java
// Java: GET /device/read?id=123 → DeviceEntity（不用 ResultDTO）
@GetMapping("/device/read")
public DeviceModel readDevice(@RequestParam Long id) {
    return deviceService.findById(id);
}
// 响应 JSON:
// {"id": 123, "serialNumber": "SN001", "deviceName": "设备A", "departmentId": "D001"}
```

```rust
// Rust: 直接返回 struct，不能包装在 ResultDTO 里
#[handler]
pub async fn read_device(req: &mut Request, res: &mut Response) {
    let id: i64 = req.query("id").unwrap();
    match DeviceAppService::find_by_id(id).await {
        Ok(vo) => {
            // 直接序列化 DeviceVO，不包 ResultDTO
            res.render(Json(vo));
        }
        Err(e) => {
            res.status_code(StatusCode::NOT_FOUND);
        }
    }
}
// 响应 JSON: {"id": 123, "serialNumber": "SN001", "deviceName": "设备A", "departmentId": "D001"}
// ← 注意必须是 camelCase，通过 #[serde(rename_all = "camelCase")] 保证
```

### 模式 C：直接返回 List

```java
// Java: GET /device/all → Iterable<DeviceEntity>
@GetMapping("/device/all")
public Iterable<DeviceModel> allDevices() {
    return deviceService.findAll();
}
// 响应 JSON:
// [{"id": 1, "serialNumber": "SN001", ...}, {"id": 2, "serialNumber": "SN002", ...}]
```

```rust
// Rust: 返回 Vec<T>，直接序列化为 JSON 数组
#[handler]
pub async fn all_devices(req: &mut Request, res: &mut Response) {
    let entities = DeviceAppService::find_all().await;
    let vos: Vec<DeviceVO> = entities.into_iter().map(|e| e.into()).collect();
    res.render(Json(vos));
}
// 响应 JSON: [{"id": 1, "serialNumber": "SN001", ...}, {"id": 2, "serialNumber": "SN002", ...}]
```

### 模式 D：ResultDTO 包装的 Page（最复杂）

```java
// Java: GET /dic/type/pagesearch → ResultDTO<Page<DicTypeModel>>
@GetMapping("/dic/type/pagesearch")
public ResultDTO<Page<DicTypeModel>> pageSearch(
    @RequestParam(defaultValue = "0") int page,
    @RequestParam(defaultValue = "20") int size
) {
    Page<DicTypeModel> result = dicTypeService.findAll(PageRequest.of(page, size));
    return ResultDTO.success("查询成功", result);
}
```

**Spring Data Page 的 JSON 格式非常复杂**，必须精确复现。详见第 5 章。

---

## 5. 分页结构兼容（Spring Data Page → Rust）

### 5.1 Spring Data Page 的 JSON 格式

Java Spring Data 的 `Page<T>` 序列化后包含以下字段：

```json
{
  "content": [...],
  "pageable": {
    "pageNumber": 0,
    "pageSize": 20,
    "sort": { "empty": true, "sorted": false, "unsorted": true },
    "offset": 0,
    "paged": true,
    "unpaged": false
  },
  "last": true,
  "totalPages": 1,
  "totalElements": 5,
  "first": true,
  "size": 20,
  "number": 0,
  "sort": { "empty": true, "sorted": false, "unsorted": true },
  "numberOfElements": 5,
  "empty": false
}
```

### 5.2 Rust 兼容结构体定义

> **注意**：以下结构体已内置在 `genies_core::page` 模块中，无需在每个微服务项目中重复定义。直接引用即可：
>
> ```rust
> use genies_core::page::{SpringPage, Pageable, Sort};
> ```

```rust
use serde::{Deserialize, Serialize};

/// 兼容 Spring Data Page 的 JSON 结构
#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct SpringPage<T> {
    pub content: Vec<T>,
    pub pageable: Pageable,
    pub last: bool,
    pub total_pages: i64,
    pub total_elements: i64,
    pub first: bool,
    pub size: i64,
    pub number: i64,
    pub sort: Sort,
    pub number_of_elements: i64,
    pub empty: bool,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Pageable {
    pub page_number: i64,
    pub page_size: i64,
    pub sort: Sort,
    pub offset: i64,
    pub paged: bool,
    pub unpaged: bool,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Sort {
    pub empty: bool,
    pub sorted: bool,
    pub unsorted: bool,
}

impl Sort {
    /// 创建默认的 unsorted Sort（与 Spring Data 的默认行为一致）
    pub fn unsorted() -> Self {
        Self {
            empty: true,
            sorted: false,
            unsorted: true,
        }
    }
}
```

### 5.3 从 RBatis Page 转换到 SpringPage

> **注意**：`SpringPage` 已内置 `From<rbatis::plugin::page::Page<T>>` 实现和 `from_rbatis_page` 方法，无需手动编写以下转换代码。以下仅展示内部实现原理供参考。

```rust
use rbatis::plugin::page::Page as RbatisPage;

impl<T> From<RbatisPage<T>> for SpringPage<T> {
    fn from(p: RbatisPage<T>) -> Self {
        let page_no = p.page_no as i64;          // RBatis 页码从 1 开始
        let page_size = p.page_size as i64;
        let total = p.total as i64;
        let total_pages = if page_size > 0 {
            (total + page_size - 1) / page_size
        } else {
            0
        };
        let number = page_no - 1;  // Spring Data 页码从 0 开始
        let number_of_elements = p.records.len() as i64;
        let sort = Sort::unsorted();

        SpringPage {
            content: p.records,
            pageable: Pageable {
                page_number: number,
                page_size,
                sort: sort.clone(),
                offset: number * page_size,
                paged: true,
                unpaged: false,
            },
            last: number >= total_pages - 1,
            total_pages,
            total_elements: total,
            first: number == 0,
            size: page_size,
            number,
            sort,
            number_of_elements,
            empty: number_of_elements == 0,
        }
    }
}
```

### 5.4 使用示例

```java
// Java Controller
@GetMapping("/dic/type/pagesearch")
public ResultDTO<Page<DicTypeModel>> pageSearch(
    @RequestParam(defaultValue = "0") int page,
    @RequestParam(defaultValue = "20") int size,
    @RequestParam(required = false) String name
) {
    Pageable pageable = PageRequest.of(page, size);
    Page<DicTypeModel> result = dicTypeRepository.findByNameContaining(name, pageable);
    return ResultDTO.success("查询成功", result);
}
```

```rust
// Rust Handler
use genies_core::page::SpringPage;

#[handler]
pub async fn page_search(req: &mut Request, res: &mut Response) {
    // Spring Data 的 page 参数从 0 开始，RBatis 从 1 开始，需要 +1
    let page: u64 = req.query("page").unwrap_or(0);
    let size: u64 = req.query("size").unwrap_or(20);
    let name: Option<String> = req.query("name");

    let rb = &CONTEXT.rbatis;
    let page_req = PageRequest::new(page + 1, size);  // Spring page=0 → RBatis page_no=1

    let rbatis_page: Page<DicTypeEntity> = DicTypeEntity::select_page(
        rb, &page_req, &name.unwrap_or_default()
    ).await.unwrap();

    // 使用内置的 from_rbatis_page 转换为 Spring Data 兼容格式（同时转换 Entity → VO）
    let spring_page: SpringPage<DicTypeVO> = SpringPage::from_rbatis_page(rbatis_page, |e| e.into());

    // 包装在 ResultDTO 中
    res.render(Json(ResultDTO::success("查询成功", spring_page)));
}
```

### 5.5 JSON 输出对比

**Java 输出：**
```json
{
  "status": 1,
  "message": "查询成功",
  "data": {
    "content": [
      {"id": 1, "typeName": "血型", "typeCode": "blood_type"},
      {"id": 2, "typeName": "科室", "typeCode": "department"}
    ],
    "pageable": {
      "pageNumber": 0, "pageSize": 20,
      "sort": {"empty": true, "sorted": false, "unsorted": true},
      "offset": 0, "paged": true, "unpaged": false
    },
    "last": true, "totalPages": 1, "totalElements": 2,
    "first": true, "size": 20, "number": 0,
    "sort": {"empty": true, "sorted": false, "unsorted": true},
    "numberOfElements": 2, "empty": false
  }
}
```

**Rust 输出（使用 SpringPage 后）：**
```json
{
  "status": 1,
  "message": "查询成功",
  "data": {
    "content": [
      {"id": 1, "typeName": "血型", "typeCode": "blood_type"},
      {"id": 2, "typeName": "科室", "typeCode": "department"}
    ],
    "pageable": {
      "pageNumber": 0, "pageSize": 20,
      "sort": {"empty": true, "sorted": false, "unsorted": true},
      "offset": 0, "paged": true, "unpaged": false
    },
    "last": true, "totalPages": 1, "totalElements": 2,
    "first": true, "size": 20, "number": 0,
    "sort": {"empty": true, "sorted": false, "unsorted": true},
    "numberOfElements": 2, "empty": false
  }
}
```

**两者完全一致 ✓**

---

## 6. 逐层迁移步骤

### 6.1 基础设施层

#### 6.1.1 数据库迁移脚本（直接复用 Java 的 SQL）

Java Flyway 的 SQL 文件可以直接复制到 Rust 项目的 `migrations/` 目录：

```
# Java 项目
src/main/resources/db/migration/
├── V1__create_device.sql
├── V2__create_blood_bag.sql
└── V3__seed_data.sql

# 直接复制到 Rust 项目
crates/my_service/migrations/
├── V1__create_device.sql      ← 完全相同的 SQL
├── V2__create_blood_bag.sql   ← 完全相同的 SQL
└── V3__seed_data.sql          ← 完全相同的 SQL
```

> **注意**：Java 和 Rust 使用不同的迁移历史表（Java 用 `flyway_schema_history`，Rust 用 `flyway_migrations`），因此 Rust 的 flyway-rs 会独立执行这些迁移。由于 DDL 语句通常使用 `CREATE TABLE IF NOT EXISTS` 等幂等写法，实际执行时已存在的对象不会重复创建。对于 MySQL 不支持 `IF NOT EXISTS` 的语句（如 `CREATE INDEX`），应使用 `--! may_fail: true` 注解容错。

#### 6.1.2 迁移模块配置

```rust
// src/infrastructure/migration.rs
use std::sync::Arc;
use flyway::MigrationRunner;
use flyway_rbatis::RbatisMigrationDriver;
use genies::context::CONTEXT;

#[flyway::migrations("migrations")]
pub struct Migrations {}

pub async fn run_migrations() {
    let rbatis = Arc::new(CONTEXT.rbatis.clone());
    let driver = Arc::new(RbatisMigrationDriver::new(rbatis, None));
    let runner = MigrationRunner::new(Migrations {}, driver.clone(), driver.clone(), false);
    runner.migrate().await.expect("数据库迁移失败");
}
```

#### 6.1.3 ID 生成：UUID → Snowflake

Java 项目中常用 `UUID.randomUUID().toString()` 生成主键 ID，迁移到 Rust 后统一使用 Genies 内置的雪花 ID 生成器替代。

**Java 原代码：**

```java
import java.util.UUID;

String id = UUID.randomUUID().toString();
entity.setId(id);
```

**Rust 迁移后：**

```rust
// 业务代码中直接调用（推荐）
let id = genies::next_id();
entity.id = Some(id);

// 在核心库内部（无法依赖 genies crate 时）
let id = genies_core::id_gen::next_id();
```

**迁移要点：**

| 对比项 | Java UUID | Rust Snowflake |
|--------|-----------|----------------|
| 生成方式 | `UUID.randomUUID().toString()` | `genies::next_id()` |
| ID 格式 | 36 字符（含 `-`），如 `550e8400-e29b-41d4-a716-446655440000` | 纯数字字符串，如 `7446616570199150889` |
| 存储类型 | VARCHAR(36) | VARCHAR(20) 即可 |
| 排序性 | 无序 | 趋势递增（按时间） |
| 唯一性保证 | 随机碰撞概率极低 | 分布式唯一（worker_id + 时间戳 + 序列号） |
| 依赖配置 | 无 | 自动（Redis 槽位 → K8s HOSTNAME → 配置 → 兜底） |
| Cargo 依赖 | 不需要添加 uuid crate | 已内置于 genies 框架，无需额外依赖 |

> **注意事项：**
> - 雪花 ID 是 **64 位整数**，以 `String` 形式返回，避免 JavaScript 精度丢失
> - 如果 Java 和 Rust 服务**共享同一数据库**，需确保两端生成的 ID 不冲突：Java 使用 UUID（36 字符），Rust 使用雪花 ID（纯数字），格式天然不同，不会冲突
> - 数据库字段类型保持 `VARCHAR` 兼容，无需修改表结构
> - `ApplicationContext` 启动时自动初始化 ID 生成器，业务代码**无需手动初始化**

### 6.2 领域层

#### 6.2.1 Entity/Model → RBatis Entity

```java
// Java JPA Entity
@Entity
@Table(name = "device")
public class DeviceModel {
    @Id
    private Long id;
    private String serialNumber;
    private String deviceName;
    private String departmentId;
    private Integer status;
    private Date createdAt;
    // getters/setters...
}
```

```rust
// Rust RBatis Entity
// 注意：字段名必须与数据库列名一致（snake_case）
// 所有字段用 Option<T>，RBatis 要求
#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub struct DeviceEntity {
    pub id: Option<i64>,
    pub serial_number: Option<String>,
    pub device_name: Option<String>,
    pub department_id: Option<String>,
    pub status: Option<i32>,
    pub created_at: Option<rbatis::rbdc::datetime::DateTime>,
}

crud!(DeviceEntity {}, "device");
```

#### 6.2.2 Repository → crud! 宏 + py_sql

```java
// Java Repository
public interface DeviceRepository extends JpaRepository<DeviceModel, Long> {
    List<DeviceModel> findByDepartmentId(String departmentId);
    List<DeviceModel> findByStatusAndDepartmentId(Integer status, String departmentId);
    Page<DeviceModel> findByDeviceNameContaining(String name, Pageable pageable);
}
```

```rust
// Rust 简单查询 — 用 select_by_map
use rbs::value;

// findByDepartmentId → select_by_map
let devices = DeviceEntity::select_by_map(
    &rb, value!{"department_id": department_id}
).await?;

// findByStatusAndDepartmentId → select_by_map 多条件
let devices = DeviceEntity::select_by_map(
    &rb, value!{"status": status, "department_id": department_id}
).await?;
```

```rust
// Rust 复杂查询 — 用 py_sql
#[py_sql(
    "`select * from device where 1=1`
      if name != '':
        ` and device_name like #{name}`
      if department_id != '':
        ` and department_id = #{department_id}`
      if status != null:
        ` and status = #{status}`
      ` order by created_at desc`"
)]
pub async fn search_devices(
    rb: &dyn Executor,
    name: &str,
    department_id: &str,
    status: &Option<i32>,
) -> rbatis::Result<Vec<DeviceEntity>> {
    impled!()
}
```

```rust
// Rust 分页查询 — 用 html_sql
use rbatis::plugin::page::{Page, PageRequest};

#[html_sql(
    r#"<select id="select_page">
        `select * from device`
        <where>
            <if test="name != ''">`and device_name like #{name}`</if>
        </where>
        ` order by created_at desc`
    </select>"#
)]
pub async fn select_device_page(
    rb: &dyn Executor,
    page_req: &PageRequest,
    name: &str,
) -> rbatis::Result<Page<DeviceEntity>> {
    impled!()
}
```

#### 6.2.3 聚合根（可选，如需事件发布）

```java
// Java 聚合根
public class Device extends ResultWithDomainEvents<Device, DeviceEvent> {
    public static Device create(String name, String serialNumber) {
        Device device = new Device();
        device.setName(name);
        device.setSerialNumber(serialNumber);
        device.registerDomainEvent(new DeviceCreatedEvent(device.getId()));
        return device;
    }
}
```

```rust
// Rust 聚合根
#[derive(Aggregate, Debug, Clone, Serialize, Deserialize)]
#[aggregate_type("Device")]
pub struct Device {
    pub id: String,
    pub name: String,
    pub serial_number: String,
    pub status: i32,
}

impl Device {
    pub fn create(name: String, serial_number: String) -> (Self, DeviceCreatedEvent) {
        let id = genies::next_id();  // 使用雪花 ID 替代 UUID
        let device = Self { id: id.clone(), name: name.clone(), serial_number, status: 0 };
        let event = DeviceCreatedEvent { id, name, created_at: chrono::Utc::now().timestamp_millis() };
        (device, event)
    }
}
```

### 6.3 应用层

#### 6.3.1 Service → AppService

```java
// Java Service
@Service
public class DeviceService {
    @Autowired
    private DeviceRepository deviceRepository;

    public DeviceModel findById(Long id) {
        return deviceRepository.findById(id).orElse(null);
    }

    public String create(DeviceCreateRequest req) {
        DeviceModel device = new DeviceModel();
        device.setSerialNumber(req.getSerialNumber());
        device.setDeviceName(req.getDeviceName());
        device.setDepartmentId(req.getDepartmentId());
        device.setStatus(0);
        device.setCreatedAt(new Date());
        deviceRepository.save(device);
        return device.getId().toString();
    }

    public List<DeviceModel> findAll() {
        return deviceRepository.findAll();
    }
}
```

```rust
// Rust AppService
use genies::context::CONTEXT;

pub struct DeviceAppService;

impl DeviceAppService {
    pub async fn find_by_id(id: i64) -> Result<DeviceVO, String> {
        let rb = &CONTEXT.rbatis;
        let list = DeviceEntity::select_by_map(rb, rbs::value!{"id": id})
            .await.map_err(|e| e.to_string())?;
        list.into_iter().next()
            .map(|e| e.into())
            .ok_or("设备不存在".to_string())
    }

    pub async fn create(req: &DeviceCreateRequest) -> Result<String, String> {
        let rb = &CONTEXT.rbatis;
        let entity = DeviceEntity {
            id: None,  // 自增
            serial_number: Some(req.serial_number.clone()),
            device_name: Some(req.device_name.clone()),
            department_id: Some(req.department_id.clone()),
            status: Some(0),
            created_at: Some(rbatis::rbdc::datetime::DateTime::now()),
        };
        DeviceEntity::insert(rb, &entity).await.map_err(|e| e.to_string())?;
        Ok(entity.id.unwrap_or_default().to_string())
    }

    pub async fn find_all() -> Vec<DeviceVO> {
        let rb = &CONTEXT.rbatis;
        let list = DeviceEntity::select_by_map(rb, rbs::value!{})
            .await.unwrap_or_default();
        list.into_iter().map(|e| e.into()).collect()
    }
}
```

#### 6.3.2 DTO 定义

```rust
// src/application/dto.rs
use serde::{Deserialize, Serialize};
use salvo::oapi::ToSchema;

/// 请求 DTO — 接收 Java camelCase JSON
#[derive(Debug, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct DeviceCreateRequest {
    pub serial_number: String,
    pub device_name: String,
    pub department_id: String,
}

/// 响应 VO — 输出 Java camelCase JSON
#[derive(Debug, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct DeviceVO {
    pub id: i64,
    pub serial_number: String,
    pub device_name: String,
    pub department_id: String,
    pub status: i32,
    pub created_at: String,
}

impl From<DeviceEntity> for DeviceVO {
    fn from(e: DeviceEntity) -> Self {
        Self {
            id: e.id.unwrap_or_default(),
            serial_number: e.serial_number.unwrap_or_default(),
            device_name: e.device_name.unwrap_or_default(),
            department_id: e.department_id.unwrap_or_default(),
            status: e.status.unwrap_or_default(),
            created_at: e.created_at
                .map(|d| d.to_string())
                .unwrap_or_default(),
        }
    }
}
```

#### 6.3.3 对象字段拷贝：`copy!` 替代 `BeanUtils.copyProperties`

Java 中常用 `BeanUtils.copyProperties(source, target)` 在 Entity、DTO、VO 之间拷贝同名字段。Rust/Genies 中使用 `copy!` 宏替代。

**Java 写法：**

```java
DeviceModel entity = deviceRepository.findById(id).orElse(null);
DeviceVO vo = new DeviceVO();
BeanUtils.copyProperties(entity, vo);  // 同名字段自动拷贝
return vo;
```

**Rust 写法：**

```rust
use genies::copy;

let entity = DeviceEntity::select_by_column(rb, "id", id).await?;
let vo: DeviceVO = copy!(&entity, DeviceVO);  // 同名字段自动拷贝
```

**DTO → Entity 的对照：**

```java
// Java
DeviceModel entity = new DeviceModel();
BeanUtils.copyProperties(request, entity);
entity.setId(UUID.randomUUID().toString());
deviceRepository.save(entity);
```

```rust
// Rust
use genies::copy;

let mut entity: DeviceEntity = copy!(&request, DeviceEntity);
entity.id = Some(genies::next_id());  // 使用雪花 ID 替代 UUID
DeviceEntity::insert(rb, &entity).await.unwrap();
```

> **注意 `#[serde(rename_all = "camelCase")]` 的影响**：`copy!` 基于 serde 序列化名匹配字段。如果源类型加了 `#[serde(rename_all = "camelCase")]`（如请求 DTO），而目标类型没有加（如 ORM Entity），则序列化名不一致（`serialNumber` vs `serial_number`），字段将**无法匹配**。解决方案：
> - 确保 `copy!` 的源和目标使用相同的 serde rename 策略
> - 或者对 ORM Entity 不加 `rename_all`，对 DTO 也不加，只在最终响应 VO 上加 `rename_all`
> - 需要跨 rename 策略拷贝时，改用手动 `From` 实现

#### 6.3.4 事件消费者 → #[topic]

```java
// Java 事件消费者
@Configuration
public class DeviceConsumersConfig {
    @Bean
    public DomainEventHandlers domainEventHandlers() {
        return DomainEventHandlersBuilder
            .forAggregateType("Device")
            .onEvent(DeviceCreatedEvent.class, this::handleDeviceCreated)
            .build();
    }

    public void handleDeviceCreated(DomainEventEnvelope<DeviceCreatedEvent> envelope) {
        DeviceCreatedEvent event = envelope.getEvent();
        // 处理逻辑
    }
}
```

```rust
// Rust 事件消费者
use genies_derive::topic;
use rbatis::executor::Executor;

#[topic(name = "Device", pubsub = "messagebus")]
pub async fn on_device_created(
    tx: &mut dyn Executor,
    event: DeviceCreatedEvent,
) -> anyhow::Result<u64> {
    log::info!("收到设备创建事件: {:?}", event);
    // 处理逻辑
    Ok(0)
}
```

### 6.4 接口层

#### 6.4.1 Controller → Handler

```java
// Java Controller
@RestController
@RequestMapping("/device")
public class DeviceController {

    @PostMapping("/create")
    public ResultDTO<String> createDevice(@RequestBody DeviceCreateRequest req) {
        String id = deviceService.create(req);
        return ResultDTO.success("创建成功", id);
    }

    @GetMapping("/read")
    public DeviceModel readDevice(@RequestParam Long id) {
        return deviceService.findById(id);
    }

    @GetMapping("/all")
    public Iterable<DeviceModel> allDevices() {
        return deviceService.findAll();
    }

    @PostMapping("/delete")
    public ResultDTO<String> deleteDevice(@RequestParam Long id) {
        deviceService.delete(id);
        return ResultDTO.success("删除成功", "");
    }
}
```

```rust
// Rust Handler — 路径必须与 Java 完全一致
use salvo::prelude::*;
use genies_core::ResultDTO;

/// POST /device/create → ResultDTO<String>（模式 A）
#[handler]
pub async fn create_device(req: &mut Request, res: &mut Response) {
    let dto: DeviceCreateRequest = req.parse_json().await.unwrap();
    match DeviceAppService::create(&dto).await {
        Ok(id) => res.render(Json(ResultDTO::success("创建成功", id))),
        Err(e) => res.render(Json(ResultDTO::<String>::error(&e))),
    }
}

/// GET /device/read?id=123 → DeviceModel（模式 B，不包 ResultDTO）
#[handler]
pub async fn read_device(req: &mut Request, res: &mut Response) {
    let id: i64 = req.query("id").unwrap();
    match DeviceAppService::find_by_id(id).await {
        Ok(vo) => res.render(Json(vo)),
        Err(_) => res.status_code(StatusCode::NOT_FOUND),
    }
}

/// GET /device/all → List<DeviceModel>（模式 C，不包 ResultDTO）
#[handler]
pub async fn all_devices(req: &mut Request, res: &mut Response) {
    let vos = DeviceAppService::find_all().await;
    res.render(Json(vos));
}

/// POST /device/delete → ResultDTO<String>（模式 A）
#[handler]
pub async fn delete_device(req: &mut Request, res: &mut Response) {
    let id: i64 = req.query("id").unwrap();
    match DeviceAppService::delete(id).await {
        Ok(_) => res.render(Json(ResultDTO::success("删除成功", "".to_string()))),
        Err(e) => res.render(Json(ResultDTO::<String>::error(&e))),
    }
}
```

#### 6.4.2 路由配置 — 路径必须完全一致

```java
// Java 路径：@RequestMapping("/device") + @PostMapping("/create")
// 完整路径：/device/create
```

```rust
// Rust 路由 — 路径与 Java 完全一致
use salvo::prelude::*;

pub fn device_router() -> Router {
    Router::with_path("device")
        .push(Router::with_path("create").post(create_device))
        .push(Router::with_path("read").get(read_device))
        .push(Router::with_path("all").get(all_devices))
        .push(Router::with_path("delete").post(delete_device))
}
```

> **关键**：Java 的 `@RequestMapping("/device")` + `@PostMapping("/create")` 对应 Rust 的 `Router::with_path("device").push(Router::with_path("create").post(...))`。路径的每一级都必须完全一致。

### 6.5 跨微服务远程调用（Java Remote → Rust #[remote]）

Java 微服务间调用通常使用 `@FeignClient` 定义远程接口，Genies 框架提供 `#[remote]` 宏 + `feignhttp` 实现等价功能，并自动管理 Token 刷新。

#### 6.5.1 Java Remote 接口（FeignClient）

```java
// Java — 典型的 FeignClient 远程调用接口
@FeignClient(name = "patient-service", url = "${GATEWAY}")
public interface PatientRemote {

    @GetMapping("/api/patients/{id}")
    PatientDTO getPatient(@PathVariable String id);

    @PostMapping("/api/patients/search")
    ResultDTO<List<PatientDTO>> searchPatients(@RequestBody PatientSearchRequest request);

    @GetMapping("/api/patients")
    ResultDTO<List<PatientDTO>> listPatients(@RequestParam String wardId,
                                              @RequestParam Integer status);

    @DeleteMapping("/api/patients/{id}")
    ResultDTO<String> deletePatient(@PathVariable String id);
}
```

#### 6.5.2 Rust #[remote] 宏对应写法

```rust
// Rust — 使用 #[remote] + feignhttp 宏实现等价的远程调用
use genies_derive::remote;
use feignhttp::{get, post, delete};

/// GET /api/patients/{id} → PatientDTO
#[remote]
#[get("${GATEWAY}/api/patients/{id}")]
pub async fn get_patient(#[path] id: String) -> feignhttp::Result<PatientDTO> {}

/// POST /api/patients/search → ResultDTO<Vec<PatientDTO>>
#[remote]
#[post("${GATEWAY}/api/patients/search")]
pub async fn search_patients(#[body] request: PatientSearchRequest) -> feignhttp::Result<ResultDTO<Vec<PatientDTO>>> {}

/// GET /api/patients?wardId=xxx&status=xxx → ResultDTO<Vec<PatientDTO>>
#[remote]
#[get("${GATEWAY}/api/patients")]
pub async fn list_patients(
    #[param] ward_id: String,
    #[param] status: i32,
) -> feignhttp::Result<ResultDTO<Vec<PatientDTO>>> {}

/// DELETE /api/patients/{id} → ResultDTO<String>
#[remote]
#[delete("${GATEWAY}/api/patients/{id}")]
pub async fn delete_patient(#[path] id: String) -> feignhttp::Result<ResultDTO<String>> {}
```

> **注意**：每个 remote 函数的函数体为空 `{}`，实际的 HTTP 调用由 `feignhttp` 宏自动生成。`#[remote]` 必须放在 feignhttp 宏（`#[get]`/`#[post]`/`#[delete]` 等）的**上方**。

#### 6.5.3 #[remote] 宏的工作机制

`#[remote]` 宏展开后会生成两个函数：

**1. 底层 feignhttp 函数 `{fn_name}_feignhttp`**

```rust
// 自动生成：带 Authorization header 的底层 HTTP 调用函数
pub async fn get_patient_feignhttp(
    #[header] Authorization: &str,
    #[path] id: String,
) -> feignhttp::Result<PatientDTO> {}
```

**2. 包装函数 `{fn_name}`（开发者实际调用的函数）**

```rust
// 自动生成：Token 管理 + 401 自动重试的包装函数
pub async fn get_patient(id: String) -> feignhttp::Result<PatientDTO> {
    // 1. 从全局 REMOTE_TOKEN 获取当前 access_token
    let bearer = genies::context::REMOTE_TOKEN.lock().unwrap().access_token.clone();
    let bearer = format!("Bearer {}", &bearer);

    // 2. 使用 Bearer Token 发起 HTTP 请求
    let mut result = get_patient_feignhttp(&bearer, id).await;

    // 3. 如果返回 401 Unauthorized，自动从 Keycloak 刷新 Token 并重试
    if result.is_err() && result.as_ref().err().unwrap().to_string().contains("401 Unauthorized") {
        if let Ok(new_token) = genies::core::jwt::get_temp_access_token(
            &genies::context::CONTEXT.config.keycloak_auth_server_url,
            &genies::context::CONTEXT.config.keycloak_realm,
            &genies::context::CONTEXT.config.keycloak_resource,
            &genies::context::CONTEXT.config.keycloak_credentials_secret,
        ).await {
            // 更新全局 Token 并重试
            genies::context::REMOTE_TOKEN.lock().unwrap().access_token = new_token.clone();
            let bearer = format!("Bearer {}", &new_token);
            result = get_patient_feignhttp(&bearer, id).await;
        }
    }
    result
}
```

> 开发者只需调用 `get_patient(id).await`，**无需手动传递 Authorization header**，Token 获取、刷新和重试都是自动完成的。

#### 6.5.4 配置要求

**1. application.yml — Keycloak 配置**

`#[remote]` 宏的 Token 刷新依赖 Keycloak 客户端凭证模式（Client Credentials Grant），需要在 `application.yml` 中配置：

```yaml
# application.yml
keycloak:
  auth_server_url: "http://keycloak:8080/auth"     # Keycloak 服务地址
  realm: "my-realm"                                  # Realm 名称
  resource: "my-client"                              # Client ID
  credentials_secret: "my-secret"                    # Client Secret
```

**2. GATEWAY 环境变量**

`feignhttp` 宏中的 `${GATEWAY}` 会从环境变量读取，启动前需要设置：

```bash
# Linux/Mac
export GATEWAY=http://api-gateway:8080

# Windows PowerShell
$env:GATEWAY = "http://api-gateway:8080"
```

也可以在代码中使用 `config_gateway!` 宏来配置：

```rust
use genies::config_gateway;
use lazy_static::lazy_static;

lazy_static! {
    pub static ref GATEWAY: String = config_gateway!("/patient");
}
```

> `config_gateway!` 会根据 gateway 配置值自动判断：如果是 `http://` 或 `https://` 开头则使用网关地址，否则使用 Dapr sidecar 地址。

#### 6.5.5 Java → Rust 迁移对照表

| Java (FeignClient) | Rust (#[remote] + feignhttp) | 说明 |
|---|---|---|
| `@FeignClient(url = "${GATEWAY}")` | `#[remote]` + `#[get("${GATEWAY}/...")]` | 接口级 → 函数级 |
| `@GetMapping("/path/{id}")` | `#[get("${GATEWAY}/path/{id}")]` | URL 中必须包含 `${GATEWAY}` 前缀 |
| `@PostMapping("/path")` | `#[post("${GATEWAY}/path")]` | POST 请求 |
| `@PutMapping("/path/{id}")` | `#[put("${GATEWAY}/path/{id}")]` | PUT 请求 |
| `@DeleteMapping("/path/{id}")` | `#[delete("${GATEWAY}/path/{id}")]` | DELETE 请求 |
| `@PathVariable String id` | `#[path] id: String` | 路径参数 |
| `@RequestBody XXX body` | `#[body] body: XXX` | 请求体（自动 JSON 序列化） |
| `@RequestParam String name` | `#[param] name: String` | 查询参数 |
| `@RequestHeader String token` | `#[header] token: String` | 请求头（`#[remote]` 已自动处理 Authorization） |
| `ResultDTO<T>` 返回类型 | `feignhttp::Result<ResultDTO<T>>` | 返回类型需要包装在 `feignhttp::Result` 中 |
| Spring 自动注入 `@Autowired` | 直接调用函数 `get_patient(id).await` | 无需依赖注入，直接 `use` 后调用 |

#### 6.5.6 注意事项

1. **所有 remote 函数必须是 `async fn`** — `#[remote]` 宏在编译期会检查，非 async 函数会直接 panic
2. **返回类型使用 `feignhttp::Result<T>`** — 其中 `T` 必须实现 `serde::Deserialize`
3. **`${GATEWAY}` 环境变量必须在启动前配置** — feignhttp 在运行时从环境变量读取
4. **Token 管理完全自动** — 不需要手动获取或传递 Token，`#[remote]` 宏自动处理
5. **函数体为空** — remote 函数的函数体写 `{}` 即可，不要写任何逻辑
6. **一个函数对应一个接口** — Java 中一个 `@FeignClient` 接口包含多个方法，Rust 中拆成独立的函数，通常放在同一个 `remote.rs` 模块中
7. **Cargo.toml 需要添加 feignhttp 依赖**：
   ```toml
   [dependencies]
   feignhttp = "0.5"
   ```

---

## 7. 接口对比测试方案

### 7.1 测试环境搭建

```
┌─────────────────────────────────────────────────────────┐
│                    同一个 MySQL + Redis                    │
│                                                           │
│  ┌──────────────┐           ┌──────────────┐             │
│  │ Java 服务     │           │ Rust 服务     │             │
│  │ :8080        │           │ :8081        │             │
│  └──────┬───────┘           └──────┬───────┘             │
│         │                          │                      │
│         └──────────┬───────────────┘                      │
│                    │                                      │
│              MySQL :3306                                  │
│              Redis :6379                                  │
└─────────────────────────────────────────────────────────┘
```

**环境要求**：
- Java 服务运行在 `http://localhost:8080`
- Rust 服务运行在 `http://localhost:8081`
- **两者必须连接同一个 MySQL 和 Redis**（这是对比测试的前提，业务提交接口测试依赖同一数据库来做变更对比）
- 测试工具：Rust + reqwest + rbatis + serde_json + tokio（标准 `#[cfg(test)]` 测试框架）
- 测试脚本需要直接连接 MySQL（通过 RBatis）来读取表数据做快照和还原

### 7.2 测试文件按接口领域划分

测试代码必须按接口所属的业务领域（聚合根/模块）拆分为独立的测试文件，禁止将所有测试写在同一个文件中。

##### 文件组织规范

```
tests/
├── common/
│   └── mod.rs              # 共享测试基础设施（配置、HTTP客户端、DB Diff工具函数等）
├── sickbed_comparison_tests.rs   # Sickbed 领域的对比测试
├── ward_comparison_tests.rs      # Ward 领域的对比测试
└── ...                           # 每个业务领域一个文件
```

##### 拆分原则

1. **一个聚合根/业务模块对应一个测试文件** — 如 Sickbed 和 Ward 是两个不同的业务领域，各自独立文件
2. **共享基础设施提取到 `tests/common/mod.rs`** — 包括配置函数、HTTP 客户端初始化、`deep_diff`、`db_snapshot`/`db_diff`/`db_restore`、`test_mutation_with_db_diff` 等工具函数，所有项标记为 `pub`
3. **每个测试文件通过 `mod common;` 引用共享模块**
4. **文件命名规范**：`{领域名}_comparison_tests.rs`，如 `sickbed_comparison_tests.rs`、`ward_comparison_tests.rs`

### 7.3 查询接口对比测试

对每个查询接口（GET 或查询类 POST）：

1. 用相同参数**先调用 Java 接口**，拿到 Java 的 JSON 响应
2. 再用相同参数**调用 Rust 接口**，拿到 Rust 的 JSON 响应
3. 对比两个响应的 JSON：
   - 字段名是否一致（camelCase vs snake_case）
   - 字段值是否一致（数字类型、字符串、null 处理）
   - 字段顺序可以不同，但字段集合和值必须一致
   - 日期/时间格式是否一致
4. 不一致时高亮显示差异字段

> **为什么先 Java 后 Rust？** 因为 Java 是已上线的正确实现，作为基准；Rust 是待验证的新实现。

### 7.4 业务提交接口测试（基于数据库变更对比）

业务提交接口（create / update / delete）的测试**不对比 HTTP 响应**，而是对比 **Java 和 Rust 对数据库造成的变更是否一致**。

#### 7.4.1 核心思路

同一个提交操作，Java 执行一次、Rust 执行一次，两次执行对数据库产生的变更（INSERT / UPDATE / DELETE）应该完全一致。

#### 7.4.2 测试流程（7 步）

```
┌───────────────────────────────────────────────────────────────────┐
│                  业务提交接口 DB 变更对比流程                        │
│                                                                    │
│  1. 分析 Java 代码，确定该操作影响哪些表                              │
│                     │                                              │
│                     ▼                                              │
│  2. 调用 Java 接口提交业务数据                                       │
│                     │                                              │
│                     ▼                                              │
│  3. 读取 DB 变更：对受影响的表做 SELECT 快照，与提交前对比             │
│          得到 java_diff（新增/修改/删除了哪些记录）                    │
│                     │                                              │
│                     ▼                                              │
│  4. 还原 DB：将数据库恢复到提交前的状态（回滚 Java 的变更）            │
│                     │                                              │
│                     ▼                                              │
│  5. 用相同参数调用 Rust 接口提交                                     │
│                     │                                              │
│                     ▼                                              │
│  6. 读取 DB 变更：对受影响的表再做 SELECT 快照，与提交前对比            │
│          得到 rust_diff（新增/修改/删除了哪些记录）                    │
│                     │                                              │
│                     ▼                                              │
│  7. 对比 java_diff 和 rust_diff 是否一致                             │
│          一致 → 测试通过 ✓                                          │
│          不一致 → 输出差异，测试失败 ✗                                │
└───────────────────────────────────────────────────────────────────┘
```

#### 7.4.3 接口-表变更矩阵

迁移每个接口前，先分析 Java 代码确定影响范围，填写以下矩阵：

| 提交接口 | HTTP 方法 | 影响的数据库表 | 预期变更内容 | 快照 WHERE 条件 |
|---------|----------|--------------|------------|----------------|
| /device/create | POST | device | INSERT 一条新记录 | `WHERE serial_number = '{请求中的serialNumber}'` |
| /device/delete | POST | device | DELETE 一条记录 | `WHERE id = {请求中的id}` |
| /device/update | POST | device | UPDATE 指定记录的字段 | `WHERE id = {请求中的id}` |
| /device/bind | POST | device, bindlog | UPDATE device.status；INSERT bindlog 一条记录 | device: `WHERE id = {请求中的deviceId}`；bindlog: `WHERE device_id = {请求中的deviceId}` |

> 一个接口可能影响多张表（如绑定操作同时更新设备状态和插入绑定日志），需逐一列出。

#### 7.4.4 DB 变更捕获方式

通过**快照对比**（snapshot diff）捕获数据库变更：

1. **提交前**：对每张受影响的表执行带 WHERE 条件的 `SELECT`，只查询可能受影响的记录子集，保存为 `before_snapshot`
2. **提交后**：用相同的 WHERE 条件再次执行 `SELECT`，保存为 `after_snapshot`
3. **diff 计算**：对比两个快照得到变更：
   - `after` 中有但 `before` 中没有的记录 → **INSERT**
   - `before` 中有但 `after` 中没有的记录 → **DELETE**
   - 两个快照中主键相同但字段值不同的记录 → **UPDATE**

> **重要**：快照**不是** `SELECT * FROM table_name` 读取全表数据！生产数据库中数据量可能非常大，全表扫描不现实。正确做法是根据分析 Java 代码的业务逻辑，加 WHERE 条件只查询可能受影响的记录。

**WHERE 条件推导示例**：

| 业务操作 | 推导依据 | 快照 WHERE 条件 |
|---------|---------|----------------|
| 创建设备 | 按请求参数过滤（提交的序列号） | `WHERE serial_number = '提交的序列号'` |
| 删除设备 | 按主键过滤（要删除的 ID） | `WHERE id = 要删除的id` |
| 按科室操作 | 按业务维度过滤（操作的科室） | `WHERE department_id = '操作的科室ID'` |

> **注意**：快照使用主键（通常是 `id` 列）来匹配记录。WHERE 条件传空字符串表示无条件查询（不推荐用于大表）。

#### 7.4.5 领域事件（Outbox 模式）与多表操作规则

业务提交接口可能同时修改多个数据库表，测试时**每个受影响的表都必须包含在 `AffectedTable` 列表中**。

##### 规则一：领域事件 → message 表必须验证

如果业务代码通过 `publish()` 发布了领域事件（Outbox 模式会将事件写入 `message` 表），`AffectedTable` **必须**包含 `message` 表：

```rust
AffectedTable {
    table: "message",
    pk_field: "id",
    order_by: "creation_time",
    where_clause: "destination LIKE '%具体事件类型名%'".to_string(),
}
```

`message` 表的 `ignore_fields` 必须包含以下动态字段（Java 和 Rust 生成的值不同属于正常行为）：
- `id` — 主键由不同框架生成
- `creation_time` — 时间戳不同
- `headers` — 可能包含框架特有的元信息
- `published` — 发布状态可能有时序差异

但 `destination`（事件目标主题）和 `payload`（事件体 JSON）**必须一致**，这是验证业务正确性的核心。

##### 规则二：多表修改 → 逐表验证

若业务操作同时修改了多个表（如新增床位同时更新病房计数），每个被修改的表都必须列出：

```rust
// 示例：新增床位 → 同时影响 SickbedEntity 和 WardEntity
&[
    AffectedTable {
        table: "SickbedEntity",
        pk_field: "id",
        order_by: "id",
        where_clause: format!("id = '{}'", new_sickbed_id),
    },
    AffectedTable {
        table: "WardEntity",
        pk_field: "id",
        order_by: "id",
        where_clause: format!("id = '{}'", ward_id),
    },
],
```

##### 规则三：接口-表变更矩阵必须完整

编写测试前，先分析 Java 源码确定接口涉及的所有表操作，形成完整的"接口-表变更矩阵"。典型场景：

| 场景 | 额外涉及的表 |
|------|-------------|
| 发布领域事件（`publish`/`DomainEventPublisher`） | `message` 表（INSERT） |
| 新增子实体时更新父实体计数 | 父实体表（UPDATE） |
| 级联删除 | 关联子实体表（DELETE） |
| 批量操作内部调用单条操作 | 与单条操作相同的表集合 |

#### 7.4.6 DB 还原方式

Java 提交后需要还原数据库，再让 Rust 在相同初始状态下提交。还原策略：

**方式 A：基于 diff 反向操作（推荐）**

| 变更类型 | 还原操作 |
|---------|---------|
| INSERT（新增了记录） | 执行 `DELETE FROM table WHERE id = ?` 删除新增的记录 |
| UPDATE（修改了记录） | 用 `before_snapshot` 中的原始数据执行 `UPDATE` 还原 |
| DELETE（删除了记录） | 用 `before_snapshot` 中的数据执行 `INSERT` 重新插入 |

**方式 B：数据库事务（更简单，但有限制）**

```
BEGIN TRANSACTION
  → 提交前快照（SELECT *）
  → 调用 Java 接口
  → 提交后快照（SELECT *）
  → 计算 diff
ROLLBACK
```

> **方式 B 的限制**：需要测试脚本能控制数据库事务，而 Java 接口内部通常自己管理事务，外部 ROLLBACK 不一定能回滚 Java 已提交的事务。因此**推荐使用方式 A**。

### 7.5 Cargo.toml 依赖配置

在项目的 `Cargo.toml` 中添加以下 `dev-dependencies`：

```toml
[dev-dependencies]
tokio = { version = "1", features = ["full"] }
reqwest = { version = "0.12", features = ["json"] }
serde_json = "1"
serde = { version = "1", features = ["derive"] }
```

> **说明**：数据库快照和还原使用项目已有的 `rbatis` 依赖（在 `[dependencies]` 中），无需额外引入 sqlx。测试代码通过 `RBatis` 直接连接 MySQL 执行原生 SQL。

### 7.6 完整 Rust 测试代码

将以下内容保存为 `tests/api_compare_test.rs`（集成测试）或放在业务 crate 的 `#[cfg(test)]` 模块中：

```rust
//! Java vs Rust 接口对比测试（基于数据库变更对比）
//!
//! 运行方式: cargo test --test api_compare_test -- --nocapture
//! 需要 Java(:8080) 和 Rust(:8081) 服务同时运行，连接同一数据库。

#[cfg(test)]
mod api_compare {
    use rbatis::RBatis;
    use rbdc_mysql::MysqlDriver;
    use reqwest::{Client, Response, StatusCode};
    use serde_json::Value;
    use std::collections::{BTreeMap, BTreeSet};

    // ============================================================
    // 配置常量
    // ============================================================

    /// Java 服务基地址
    const JAVA_BASE_URL: &str = "http://localhost:8080";
    /// Rust 服务基地址
    const RUST_BASE_URL: &str = "http://localhost:8081";
    /// MySQL 连接地址（与 Java/Rust 服务共用同一个数据库）
    const DATABASE_URL: &str = "mysql://root:password@localhost:3306/my_database";

    /// Bearer Token 认证（如不需要认证可留空字符串）
    const AUTH_TOKEN: &str = "";

    // ============================================================
    // RBatis 初始化
    // ============================================================

    /// 创建一个独立的 RBatis 连接，用于测试脚本直接读写数据库
    async fn init_test_rbatis() -> RBatis {
        let rb = RBatis::new();
        rb.init(MysqlDriver {}, DATABASE_URL).unwrap();
        rb
    }

    // ============================================================
    // 差异描述结构体
    // ============================================================

    /// 单条差异记录
    #[derive(Debug, Clone)]
    struct Diff {
        path: String,
        detail: String,
    }

    impl std::fmt::Display for Diff {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            write!(f, "  {}: {}", self.path, self.detail)
        }
    }

    // ============================================================
    // deep_diff —— 递归对比两个 serde_json::Value
    // ============================================================

    /// 递归对比两个 JSON Value，返回所有差异的列表。
    ///
    /// - 对象（Map）按 key 集合取并集逐一比较；
    /// - 数组按索引逐元素比较，长度不同时也记录差异；
    /// - Number 统一转 f64 再比较，避免整数/浮点误判；
    /// - 其余类型直接 `==` 比较。
    fn deep_diff(java: &Value, rust: &Value, path: &str) -> Vec<Diff> {
        let mut diffs = Vec::new();

        match (java, rust) {
            // ---------- 两边都是 Object ----------
            (Value::Object(jm), Value::Object(rm)) => {
                let java_keys: BTreeSet<&String> = jm.keys().collect();
                let rust_keys: BTreeSet<&String> = rm.keys().collect();

                // Java 有但 Rust 缺少的字段
                for key in java_keys.difference(&rust_keys) {
                    diffs.push(Diff {
                        path: format!("{}.{}", path, key),
                        detail: format!("Rust 缺少此字段 (Java 值={})", jm[key.as_str()]),
                    });
                }
                // Rust 多出的字段
                for key in rust_keys.difference(&java_keys) {
                    diffs.push(Diff {
                        path: format!("{}.{}", path, key),
                        detail: format!("Rust 多出此字段 (Rust 值={})", rm[key.as_str()]),
                    });
                }
                // 共有字段递归比较
                for key in java_keys.intersection(&rust_keys) {
                    diffs.extend(deep_diff(
                        &jm[key.as_str()],
                        &rm[key.as_str()],
                        &format!("{}.{}", path, key),
                    ));
                }
            }

            // ---------- 两边都是 Array ----------
            (Value::Array(ja), Value::Array(ra)) => {
                if ja.len() != ra.len() {
                    diffs.push(Diff {
                        path: path.to_string(),
                        detail: format!(
                            "数组长度不同 Java={} Rust={}",
                            ja.len(),
                            ra.len()
                        ),
                    });
                }
                let min_len = ja.len().min(ra.len());
                for i in 0..min_len {
                    diffs.extend(deep_diff(
                        &ja[i],
                        &ra[i],
                        &format!("{}[{}]", path, i),
                    ));
                }
            }

            // ---------- 两边都是 Number ----------
            (Value::Number(jn), Value::Number(rn)) => {
                // 统一转 f64 比较，避免 i64 vs f64 类型差异
                let jf = jn.as_f64().unwrap_or(f64::NAN);
                let rf = rn.as_f64().unwrap_or(f64::NAN);
                if (jf - rf).abs() > f64::EPSILON {
                    diffs.push(Diff {
                        path: path.to_string(),
                        detail: format!("值不同 Java={} Rust={}", jn, rn),
                    });
                }
            }

            // ---------- 类型不一致 ----------
            _ if std::mem::discriminant(java) != std::mem::discriminant(rust) => {
                diffs.push(Diff {
                    path: path.to_string(),
                    detail: format!("类型不同 Java={} Rust={}", java, rust),
                });
            }

            // ---------- 同类型直接比较（String / Bool / Null）----------
            _ => {
                if java != rust {
                    diffs.push(Diff {
                        path: path.to_string(),
                        detail: format!("值不同 Java={} Rust={}", java, rust),
                    });
                }
            }
        }

        diffs
    }

    // ============================================================
    // HTTP 辅助函数
    // ============================================================

    /// 构建带认证的 reqwest Client
    fn build_client() -> Client {
        Client::builder()
            .timeout(std::time::Duration::from_secs(10))
            .build()
            .expect("构建 HTTP Client 失败")
    }

    /// 为请求添加公共 Header（Content-Type + Bearer Token）
    fn authorized_get(client: &Client, url: &str) -> reqwest::RequestBuilder {
        let rb = client.get(url).header("Content-Type", "application/json");
        if !AUTH_TOKEN.is_empty() {
            rb.bearer_auth(AUTH_TOKEN)
        } else {
            rb
        }
    }

    /// 为 POST 请求添加公共 Header（Content-Type + Bearer Token）
    fn authorized_post(client: &Client, url: &str) -> reqwest::RequestBuilder {
        let rb = client.post(url).header("Content-Type", "application/json");
        if !AUTH_TOKEN.is_empty() {
            rb.bearer_auth(AUTH_TOKEN)
        } else {
            rb
        }
    }

    // ============================================================
    // compare_get —— 对比 GET 接口（先调 Java，再调 Rust）
    // ============================================================

    /// 先调用 Java 的 GET 接口，再调用 Rust 的 GET 接口，对比 JSON 响应。
    ///
    /// - `name`：测试名称，用于日志输出
    /// - `path`：接口路径，如 `/device/all`
    /// - `query`：可选的查询参数列表
    async fn compare_get(
        client: &Client,
        name: &str,
        path: &str,
        query: &[(&str, &str)],
    ) -> Vec<Diff> {
        println!("[compare_get] {name}");

        // ---- 1. 先调用 Java 接口（基准） ----
        let java_url = format!("{}{}", JAVA_BASE_URL, path);
        let java_resp: Response = authorized_get(client, &java_url)
            .query(query)
            .send()
            .await
            .unwrap_or_else(|e| panic!("{name}: Java 请求失败: {e}"));

        // ---- 2. 再调用 Rust 接口（待验证） ----
        let rust_url = format!("{}{}", RUST_BASE_URL, path);
        let rust_resp: Response = authorized_get(client, &rust_url)
            .query(query)
            .send()
            .await
            .unwrap_or_else(|e| panic!("{name}: Rust 请求失败: {e}"));

        // 先比状态码
        assert_eq!(
            java_resp.status(),
            rust_resp.status(),
            "{name}: HTTP 状态码不同 Java={} Rust={}",
            java_resp.status(),
            rust_resp.status(),
        );

        let java_json: Value = java_resp.json().await.expect("Java 响应非 JSON");
        let rust_json: Value = rust_resp.json().await.expect("Rust 响应非 JSON");

        let diffs = deep_diff(&java_json, &rust_json, "");
        if !diffs.is_empty() {
            println!("  [DIFF] {name} 发现 {} 处差异：", diffs.len());
            for d in &diffs {
                println!("    {d}");
            }
        } else {
            println!("  [OK] {name} 无差异");
        }
        diffs
    }

    // ============================================================
    // compare_post —— 对比查询类 POST 接口（先调 Java，再调 Rust）
    // ============================================================

    /// 先调用 Java 的 POST 接口，再调用 Rust 的 POST 接口，对比 JSON 响应。
    /// 仅用于**查询类** POST（不修改数据库），业务提交类 POST 使用 test_mutation_with_db_diff。
    async fn compare_post(
        client: &Client,
        name: &str,
        path: &str,
        body: &Value,
        query: &[(&str, &str)],
    ) -> Vec<Diff> {
        println!("[compare_post] {name}");

        // ---- 1. 先调用 Java 接口（基准） ----
        let java_url = format!("{}{}", JAVA_BASE_URL, path);
        let java_resp = authorized_post(client, &java_url)
            .query(query)
            .json(body)
            .send()
            .await
            .unwrap_or_else(|e| panic!("{name}: Java 请求失败: {e}"));

        // ---- 2. 再调用 Rust 接口（待验证） ----
        let rust_url = format!("{}{}", RUST_BASE_URL, path);
        let rust_resp = authorized_post(client, &rust_url)
            .query(query)
            .json(body)
            .send()
            .await
            .unwrap_or_else(|e| panic!("{name}: Rust 请求失败: {e}"));

        assert_eq!(
            java_resp.status(),
            rust_resp.status(),
            "{name}: HTTP 状态码不同 Java={} Rust={}",
            java_resp.status(),
            rust_resp.status(),
        );

        let java_json: Value = java_resp.json().await.expect("Java 响应非 JSON");
        let rust_json: Value = rust_resp.json().await.expect("Rust 响应非 JSON");

        let diffs = deep_diff(&java_json, &rust_json, "");
        if !diffs.is_empty() {
            println!("  [DIFF] {name} 发现 {} 处差异：", diffs.len());
            for d in &diffs {
                println!("    {d}");
            }
        } else {
            println!("  [OK] {name} 无差异");
        }
        diffs
    }

    // ============================================================
    // DB 快照与 Diff 工具函数
    // ============================================================

    /// 对指定表执行带 WHERE 条件的 SELECT，返回匹配的行（每行是一个 JSON Object）。
    /// 使用 RBatis 的 query_decode 执行原生 SQL。
    ///
    /// # 参数
    /// - `rb`: RBatis 实例
    /// - `table`: 表名
    /// - `where_clause`: WHERE 条件（不含 WHERE 关键字本身，传空字符串表示无条件）
    /// - `order_by`: 排序字段（通常是主键 "id"），确保快照顺序稳定
    async fn db_snapshot(rb: &RBatis, table: &str, where_clause: &str, order_by: &str) -> Vec<Value> {
        let sql = if where_clause.is_empty() {
            format!("SELECT * FROM {} ORDER BY {}", table, order_by)
        } else {
            format!("SELECT * FROM {} WHERE {} ORDER BY {}", table, where_clause, order_by)
        };
        rb.query_decode::<Vec<Value>>(&sql, vec![])
            .await
            .unwrap_or_else(|e| panic!("db_snapshot 查询 {} 失败: {}", table, e))
    }

    /// 数据库变更类型
    #[derive(Debug, Clone, PartialEq)]
    enum ChangeType {
        /// 新增记录
        Insert(Value),
        /// 删除记录
        Delete(Value),
        /// 更新记录：(旧值, 新值)
        Update { before: Value, after: Value },
    }

    /// 数据库变更记录
    #[derive(Debug, Clone)]
    struct DbChange {
        /// 变更类型
        change_type: ChangeType,
        /// 记录主键值
        pk_value: Value,
    }

    impl std::fmt::Display for DbChange {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            match &self.change_type {
                ChangeType::Insert(row) => write!(f, "INSERT pk={}: {}", self.pk_value, row),
                ChangeType::Delete(row) => write!(f, "DELETE pk={}: {}", self.pk_value, row),
                ChangeType::Update { before, after } => {
                    write!(f, "UPDATE pk={}: {} -> {}", self.pk_value, before, after)
                }
            }
        }
    }

    /// 对比两个快照，找出增删改变更。
    ///
    /// # 参数
    /// - `before`: 提交前快照
    /// - `after`: 提交后快照
    /// - `pk_field`: 主键字段名（如 "id"）
    ///
    /// # 返回
    /// 变更列表（INSERT / DELETE / UPDATE）
    fn db_diff(before: &[Value], after: &[Value], pk_field: &str) -> Vec<DbChange> {
        let mut changes = Vec::new();

        // 将 before 和 after 按主键建索引
        let before_map: BTreeMap<String, &Value> = before
            .iter()
            .filter_map(|row| {
                row.get(pk_field)
                    .map(|pk| (pk.to_string(), row))
            })
            .collect();

        let after_map: BTreeMap<String, &Value> = after
            .iter()
            .filter_map(|row| {
                row.get(pk_field)
                    .map(|pk| (pk.to_string(), row))
            })
            .collect();

        // after 中有但 before 中没有 → INSERT
        for (pk_str, row) in &after_map {
            if !before_map.contains_key(pk_str) {
                changes.push(DbChange {
                    change_type: ChangeType::Insert((*row).clone()),
                    pk_value: row.get(pk_field).cloned().unwrap_or(Value::Null),
                });
            }
        }

        // before 中有但 after 中没有 → DELETE
        for (pk_str, row) in &before_map {
            if !after_map.contains_key(pk_str) {
                changes.push(DbChange {
                    change_type: ChangeType::Delete((*row).clone()),
                    pk_value: row.get(pk_field).cloned().unwrap_or(Value::Null),
                });
            }
        }

        // 两边都有但内容不同 → UPDATE
        for (pk_str, before_row) in &before_map {
            if let Some(after_row) = after_map.get(pk_str) {
                if before_row != after_row {
                    changes.push(DbChange {
                        change_type: ChangeType::Update {
                            before: (*before_row).clone(),
                            after: (*after_row).clone(),
                        },
                        pk_value: before_row.get(pk_field).cloned().unwrap_or(Value::Null),
                    });
                }
            }
        }

        changes
    }

    /// 根据 db_diff 的结果，将数据库还原到提交前的状态。
    ///
    /// - INSERT 的记录 → DELETE
    /// - DELETE 的记录 → 重新 INSERT
    /// - UPDATE 的记录 → 用 before 的值还原
    ///
    /// # 参数
    /// - `rb`: RBatis 实例
    /// - `table`: 表名
    /// - `pk_field`: 主键字段名
    /// - `changes`: db_diff 计算出的变更列表
    async fn db_restore(rb: &RBatis, table: &str, pk_field: &str, changes: &[DbChange]) {
        for change in changes {
            match &change.change_type {
                // INSERT 的记录 → 删除它
                ChangeType::Insert(_) => {
                    let sql = format!(
                        "DELETE FROM {} WHERE {} = ?",
                        table, pk_field
                    );
                    let pk = rbs::to_value(&change.pk_value).unwrap();
                    rb.exec(&sql, vec![pk])
                        .await
                        .unwrap_or_else(|e| panic!("db_restore DELETE 失败: {}", e));
                }

                // DELETE 的记录 → 重新插入
                ChangeType::Delete(row) => {
                    if let Value::Object(map) = row {
                        let columns: Vec<&str> = map.keys().map(|k| k.as_str()).collect();
                        let placeholders: Vec<&str> = columns.iter().map(|_| "?").collect();
                        let sql = format!(
                            "INSERT INTO {} ({}) VALUES ({})",
                            table,
                            columns.join(", "),
                            placeholders.join(", ")
                        );
                        let values: Vec<rbs::Value> = columns
                            .iter()
                            .map(|col| rbs::to_value(&map[*col]).unwrap())
                            .collect();
                        rb.exec(&sql, values)
                            .await
                            .unwrap_or_else(|e| panic!("db_restore INSERT 失败: {}", e));
                    }
                }

                // UPDATE 的记录 → 用 before 的值还原
                ChangeType::Update { before, .. } => {
                    if let Value::Object(map) = before {
                        let set_clauses: Vec<String> = map
                            .keys()
                            .filter(|k| k.as_str() != pk_field)
                            .map(|k| format!("{} = ?", k))
                            .collect();
                        let sql = format!(
                            "UPDATE {} SET {} WHERE {} = ?",
                            table,
                            set_clauses.join(", "),
                            pk_field
                        );
                        let mut values: Vec<rbs::Value> = map
                            .keys()
                            .filter(|k| k.as_str() != pk_field)
                            .map(|k| rbs::to_value(&map[k]).unwrap())
                            .collect();
                        values.push(rbs::to_value(&change.pk_value).unwrap());
                        rb.exec(&sql, values)
                            .await
                            .unwrap_or_else(|e| panic!("db_restore UPDATE 失败: {}", e));
                    }
                }
            }
        }
    }

    /// 对比两组 DbChange 是否一致（忽略自增 ID 等不可控字段）。
    ///
    /// # 参数
    /// - `java_changes`: Java 提交产生的变更
    /// - `rust_changes`: Rust 提交产生的变更
    /// - `ignore_fields`: 对比时忽略的字段（如 "id"、"created_at" 等自增/时间字段）
    fn compare_db_changes(
        java_changes: &[DbChange],
        rust_changes: &[DbChange],
        ignore_fields: &[&str],
    ) -> Vec<Diff> {
        let mut diffs = Vec::new();

        // 先比变更数量
        if java_changes.len() != rust_changes.len() {
            diffs.push(Diff {
                path: "changes.len".to_string(),
                detail: format!(
                    "变更数量不同 Java={} Rust={}",
                    java_changes.len(),
                    rust_changes.len()
                ),
            });
            return diffs;
        }

        // 逐条对比（按顺序）
        for (i, (jc, rc)) in java_changes.iter().zip(rust_changes.iter()).enumerate() {
            let prefix = format!("changes[{}]", i);

            // 比较变更类型
            let j_type = match &jc.change_type {
                ChangeType::Insert(_) => "INSERT",
                ChangeType::Delete(_) => "DELETE",
                ChangeType::Update { .. } => "UPDATE",
            };
            let r_type = match &rc.change_type {
                ChangeType::Insert(_) => "INSERT",
                ChangeType::Delete(_) => "DELETE",
                ChangeType::Update { .. } => "UPDATE",
            };
            if j_type != r_type {
                diffs.push(Diff {
                    path: format!("{}.type", prefix),
                    detail: format!("变更类型不同 Java={} Rust={}", j_type, r_type),
                });
                continue;
            }

            // 取出行数据进行字段级对比（过滤掉 ignore_fields）
            let (j_row, r_row) = match (&jc.change_type, &rc.change_type) {
                (ChangeType::Insert(j), ChangeType::Insert(r)) => (j, r),
                (ChangeType::Delete(j), ChangeType::Delete(r)) => (j, r),
                (
                    ChangeType::Update { after: j, .. },
                    ChangeType::Update { after: r, .. },
                ) => (j, r),
                _ => continue,
            };

            // 过滤掉忽略字段后对比
            let j_filtered = filter_fields(j_row, ignore_fields);
            let r_filtered = filter_fields(r_row, ignore_fields);
            let field_diffs = deep_diff(&j_filtered, &r_filtered, &prefix);
            diffs.extend(field_diffs);
        }

        diffs
    }

    /// 从 JSON Object 中移除指定字段，返回过滤后的 Value
    fn filter_fields(value: &Value, ignore_fields: &[&str]) -> Value {
        if let Value::Object(map) = value {
            let filtered: serde_json::Map<String, Value> = map
                .iter()
                .filter(|(k, _)| !ignore_fields.contains(&k.as_str()))
                .map(|(k, v)| (k.clone(), v.clone()))
                .collect();
            Value::Object(filtered)
        } else {
            value.clone()
        }
    }

    // ============================================================
    // test_mutation_with_db_diff —— 业务提交 DB 变更对比
    // ============================================================

    /// 受影响的表描述
    struct AffectedTable {
        /// 表名
        table: &'static str,
        /// 主键字段名
        pk_field: &'static str,
        /// 排序字段（用于快照时稳定排序）
        order_by: &'static str,
        /// 快照 WHERE 条件（不含 WHERE 关键字，空字符串表示无条件——不推荐用于大表）
        where_clause: String,
    }

    /// 业务提交接口测试（基于数据库变更对比）：
    ///
    /// 1. 快照受影响表的数据（before）
    /// 2. 调用 Java 接口提交
    /// 3. 再次快照（after_java），计算 java_diff
    /// 4. 还原数据库到 before 状态
    /// 5. 调用 Rust 接口提交
    /// 6. 再次快照（after_rust），计算 rust_diff
    /// 7. 对比 java_diff 和 rust_diff 是否一致
    ///
    /// # 参数
    /// - `client`: HTTP Client
    /// - `rb`: RBatis 实例（连接同一数据库）
    /// - `name`: 测试名称
    /// - `mutation_path`: 提交接口路径
    /// - `mutation_body`: 提交请求体
    /// - `affected_tables`: 受影响的数据库表列表
    /// - `ignore_fields`: 对比时忽略的字段（如自增 ID、创建时间等）
    async fn test_mutation_with_db_diff(
        client: &Client,
        rb: &RBatis,
        name: &str,
        mutation_path: &str,
        mutation_body: &Value,
        affected_tables: &[AffectedTable],
        ignore_fields: &[&str],
    ) {
        println!("\n[mutation_db_diff] {name}");
        println!("  受影响的表: {:?}", affected_tables.iter().map(|t| t.table).collect::<Vec<_>>());

        // ---- 步骤 1: 提交前快照 ----
        println!("  [1/7] 快照受影响表的数据（before）...");
        let mut before_snapshots: Vec<Vec<Value>> = Vec::new();
        for t in affected_tables {
            let snapshot = db_snapshot(rb, t.table, &t.where_clause, t.order_by).await;
            println!("    {} (WHERE {}) : {} 条记录", t.table, if t.where_clause.is_empty() { "*" } else { &t.where_clause }, snapshot.len());
            before_snapshots.push(snapshot);
        }

        // ---- 步骤 2: 调用 Java 接口提交 ----
        println!("  [2/7] 调用 Java 接口提交...");
        let java_url = format!("{}{}", JAVA_BASE_URL, mutation_path);
        let java_resp = authorized_post(client, &java_url)
            .json(mutation_body)
            .send()
            .await
            .unwrap_or_else(|e| panic!("{name}: Java 提交请求失败: {e}"));
        assert_eq!(
            java_resp.status(),
            StatusCode::OK,
            "{name}: Java 提交接口返回非 200，实际={}",
            java_resp.status(),
        );
        println!("    Java 提交成功 (HTTP 200)");

        // ---- 步骤 3: 提交后快照 & 计算 java_diff ----
        println!("  [3/7] 读取 Java 提交后的 DB 变更...");
        let mut java_diffs_per_table: Vec<Vec<DbChange>> = Vec::new();
        for (i, t) in affected_tables.iter().enumerate() {
            let after_snapshot = db_snapshot(rb, t.table, &t.where_clause, t.order_by).await;
            let changes = db_diff(&before_snapshots[i], &after_snapshot, t.pk_field);
            println!("    {} : {} 条变更", t.table, changes.len());
            for c in &changes {
                println!("      {}", c);
            }
            java_diffs_per_table.push(changes);
        }

        // ---- 步骤 4: 还原数据库 ----
        println!("  [4/7] 还原数据库到提交前状态...");
        for (i, t) in affected_tables.iter().enumerate() {
            db_restore(rb, t.table, t.pk_field, &java_diffs_per_table[i]).await;
        }
        // 验证还原成功
        for (i, t) in affected_tables.iter().enumerate() {
            let restored = db_snapshot(rb, t.table, &t.where_clause, t.order_by).await;
            assert_eq!(
                before_snapshots[i].len(),
                restored.len(),
                "还原后 {} 记录数不一致: 期望={} 实际={}",
                t.table,
                before_snapshots[i].len(),
                restored.len()
            );
        }
        println!("    还原成功 ✓");

        // ---- 步骤 5: 调用 Rust 接口提交 ----
        println!("  [5/7] 用相同参数调用 Rust 接口提交...");
        let rust_url = format!("{}{}", RUST_BASE_URL, mutation_path);
        let rust_resp = authorized_post(client, &rust_url)
            .json(mutation_body)
            .send()
            .await
            .unwrap_or_else(|e| panic!("{name}: Rust 提交请求失败: {e}"));
        assert_eq!(
            rust_resp.status(),
            StatusCode::OK,
            "{name}: Rust 提交接口返回非 200，实际={}",
            rust_resp.status(),
        );
        println!("    Rust 提交成功 (HTTP 200)");

        // ---- 步骤 6: 提交后快照 & 计算 rust_diff ----
        println!("  [6/7] 读取 Rust 提交后的 DB 变更...");
        let mut rust_diffs_per_table: Vec<Vec<DbChange>> = Vec::new();
        for (i, t) in affected_tables.iter().enumerate() {
            let after_snapshot = db_snapshot(rb, t.table, &t.where_clause, t.order_by).await;
            let changes = db_diff(&before_snapshots[i], &after_snapshot, t.pk_field);
            println!("    {} : {} 条变更", t.table, changes.len());
            for c in &changes {
                println!("      {}", c);
            }
            rust_diffs_per_table.push(changes);
        }

        // ---- 步骤 7: 对比 java_diff 和 rust_diff ----
        println!("  [7/7] 对比 Java 和 Rust 的 DB 变更...");
        let mut all_ok = true;
        for (i, t) in affected_tables.iter().enumerate() {
            let diffs = compare_db_changes(
                &java_diffs_per_table[i],
                &rust_diffs_per_table[i],
                ignore_fields,
            );
            if diffs.is_empty() {
                println!("    [OK] {} : Java 和 Rust 变更一致 ✓", t.table);
            } else {
                all_ok = false;
                println!("    [DIFF] {} : 发现 {} 处差异：", t.table, diffs.len());
                for d in &diffs {
                    println!("      {d}");
                }
            }
        }

        assert!(all_ok, "{name}: Java 和 Rust 的 DB 变更不一致！");
        println!("  [PASS] {name} 测试通过 ✓\n");
    }

    // ============================================================
    // 测试用例 —— 根据实际接口修改
    // ============================================================

    /// 查询接口对比测试：先调 Java、再调 Rust，deep diff 响应
    #[tokio::test]
    async fn test_query_comparison() {
        let client = build_client();

        // 示例：对比 GET /device/all
        let diffs = compare_get(&client, "GET /device/all - 查询所有设备", "/device/all", &[]).await;
        assert!(diffs.is_empty(), "/device/all 存在差异");

        // 示例：对比 GET /device/read?id=1
        let diffs = compare_get(
            &client,
            "GET /device/read - 查询单个设备",
            "/device/read",
            &[("id", "1")],
        )
        .await;
        assert!(diffs.is_empty(), "/device/read 存在差异");

        // 示例：对比 GET /dic/type/pagesearch 分页接口
        let diffs = compare_get(
            &client,
            "GET /dic/type/pagesearch - 分页查询",
            "/dic/type/pagesearch",
            &[("page", "0"), ("size", "20")],
        )
        .await;
        assert!(diffs.is_empty(), "/dic/type/pagesearch 存在差异");
    }

    /// POST 查询接口对比测试（查询类 POST，不修改数据库）
    #[tokio::test]
    async fn test_post_query_comparison() {
        let client = build_client();

        // 示例：对比 POST /device/query（查询类 POST）
        let body = serde_json::json!({
            "page": 0,
            "size": 20,
            "keyword": "测试"
        });
        let diffs = compare_post(
            &client,
            "POST /device/query - 设备查询",
            "/device/query",
            &body,
            &[],
        )
        .await;
        assert!(diffs.is_empty(), "/device/query 存在差异");
    }

    /// 业务提交接口测试：基于数据库变更对比
    ///
    /// 完整流程：
    ///   快照 → Java 提交 → 读变更 → 还原 → Rust 提交 → 读变更 → 对比
    #[tokio::test]
    async fn test_create_device_mutation() {
        let client = build_client();
        let rb = init_test_rbatis().await;

        let body = serde_json::json!({
            "serialNumber": "TEST_SN_001",
            "deviceName": "测试设备",
            "departmentId": "DEPT_001"
        });

        // 根据请求参数构造 WHERE 条件，只查询可能受影响的记录
        let serial_number = "TEST_SN_001";

        test_mutation_with_db_diff(
            &client,
            &rb,
            "POST /device/create - 创建设备",
            "/device/create",
            &body,
            // 该接口影响的表（参考 7.3.3 接口-表变更矩阵）
            &[AffectedTable {
                table: "device",
                pk_field: "id",
                order_by: "id",
                where_clause: format!("serial_number = '{}'", serial_number),
            }],
            // 忽略自增 ID 和创建时间（Java 和 Rust 生成的值不同是正常的）
            &["id", "created_at"],
        )
        .await;
    }

    /// 业务提交接口测试：删除设备
    #[tokio::test]
    async fn test_delete_device_mutation() {
        let client = build_client();
        let rb = init_test_rbatis().await;

        // 注意：测试前确保数据库中存在 id=1 的设备记录
        let delete_id = 1;
        let body = serde_json::json!({});

        test_mutation_with_db_diff(
            &client,
            &rb,
            "POST /device/delete?id=1 - 删除设备",
            "/device/delete?id=1",
            &body,
            &[AffectedTable {
                table: "device",
                pk_field: "id",
                order_by: "id",
                where_clause: format!("id = {}", delete_id),
            }],
            &[],  // DELETE 操作不需要忽略字段
        )
        .await;
    }

    /// 业务提交接口测试：涉及多张表的操作
    #[tokio::test]
    async fn test_bind_device_mutation() {
        let client = build_client();
        let rb = init_test_rbatis().await;

        let body = serde_json::json!({
            "deviceId": "1",
            "userId": "USER_001"
        });

        let device_id = "1";

        test_mutation_with_db_diff(
            &client,
            &rb,
            "POST /device/bind - 绑定设备（多表）",
            "/device/bind",
            &body,
            // 该接口影响两张表
            &[
                AffectedTable {
                    table: "device",
                    pk_field: "id",
                    order_by: "id",
                    where_clause: format!("id = {}", device_id),
                },
                AffectedTable {
                    table: "bindlog",
                    pk_field: "id",
                    order_by: "id",
                    where_clause: format!("device_id = {}", device_id),
                },
            ],
            &["id", "created_at", "bind_time"],
        )
        .await;
    }
}
```

---

## 8. 常见兼容性陷阱

### 8.1 camelCase vs snake_case（最常见！）

**问题**：Rust 的 serde 默认使用 snake_case，Java 默认使用 camelCase。

**解决**：所有请求 DTO 和响应 VO 都必须加 `#[serde(rename_all = "camelCase")]`。

```rust
// ✗ 错误 — 输出 {"serial_number": "SN001"}
#[derive(Serialize)]
pub struct DeviceVO {
    pub serial_number: String,
}

// ✓ 正确 — 输出 {"serialNumber": "SN001"}
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DeviceVO {
    pub serial_number: String,
}
```

### 8.2 Java Integer id vs Rust 类型

**问题**：Java 的 `Long id` 返回数字 `{"id": 123}`，如果 Rust 用 `String` 类型会变成 `{"id": "123"}`。

**解决**：确保 Rust 的 id 类型与 Java 一致。

```rust
// Java Long id → Rust i64
pub struct DeviceVO {
    pub id: i64,           // 输出 {"id": 123}  ← 与 Java 一致
    // pub id: String,     // 输出 {"id": "123"} ← 与 Java 不一致！
}
```

### 8.3 null 序列化行为差异

**问题**：Java 的 Jackson 默认序列化 null 字段（`{"name": null}`），Rust 的 `Option<String>` 默认也序列化 null，但如果加了 `skip_serializing_if` 就会跳过。

**解决**：根据 Java 的行为决定是否跳过 null。

```rust
// Java 行为：null 字段仍然输出 → Rust 不要加 skip_serializing_if
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DeviceVO {
    pub id: i64,
    pub device_name: Option<String>,     // {"deviceName": null} ← 与 Java 一致
}

// 如果 Java 用了 @JsonInclude(NON_NULL) → Rust 加 skip_serializing_if
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DeviceVO {
    pub id: i64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub device_name: Option<String>,     // null 时字段不输出
}
```

### 8.4 Java Date/Long 时间戳 vs Rust

**问题**：Java 返回的日期格式可能是多种形式。

**解决**：需要逐个检查 Java 的实际输出格式。

```rust
// 如果 Java 返回 Long 时间戳（毫秒）: {"createdAt": 1704067200000}
pub struct DeviceVO {
    pub created_at: i64,
}

// 如果 Java 返回格式化字符串: {"createdAt": "2024-01-01 00:00:00"}
pub struct DeviceVO {
    pub created_at: String,
}

// 如果 Java 返回 ISO 格式: {"createdAt": "2024-01-01T00:00:00Z"}
pub struct DeviceVO {
    pub created_at: String,
}
```

### 8.5 Spring Data Page 的复杂结构

**问题**：RBatis 的 `Page<T>` 字段名和结构与 Spring Data `Page<T>` 完全不同。

**解决**：使用第 5 章定义的 `SpringPage<T>` 转换结构。

### 8.6 Java 枚举序列化

**问题**：Java 枚举可能序列化为数字或字符串。

```rust
// Java 枚举序列化为数字: {"status": 0}
pub struct DeviceVO {
    pub status: i32,        // 直接用 i32
}

// Java 枚举序列化为字符串: {"status": "ACTIVE"}
pub struct DeviceVO {
    pub status: String,     // 直接用 String
}

// 如果需要 Rust enum 映射
#[derive(Serialize, Deserialize)]
pub enum DeviceStatus {
    #[serde(rename = "ACTIVE")]
    Active,
    #[serde(rename = "INACTIVE")]
    Inactive,
}
```

### 8.7 HTTP 状态码差异

**问题**：Java 框架在某些情况下返回 200 以外的状态码。

**解决**：确保 Rust 返回与 Java 相同的 HTTP 状态码。

```rust
// Java 返回 200 + ResultDTO → Rust 也返回 200 + ResultDTO
res.render(Json(ResultDTO::<String>::error("参数错误")));
// 不要改成 res.status_code(StatusCode::BAD_REQUEST);
```

### 8.8 查询参数名的大小写

**问题**：Java 的 `@RequestParam("pageIndex")` 是 camelCase，Rust 必须匹配。

```rust
// ✗ 错误 — 参数名不匹配
let page_index: u64 = req.query("page_index").unwrap_or(0);

// ✓ 正确 — 参数名与 Java 一致
let page_index: u64 = req.query("pageIndex").unwrap_or(0);
```

### 8.9 `#[casbin]` 字段过滤对兼容性的影响

`#[casbin]` 采用**默认全部放行**（allow-all）策略：未配置 deny 规则前，不会过滤任何字段，接口响应与不加 `#[casbin]` 完全一致。因此迁移时可以直接加上 `#[casbin]`，无需分阶段。

```rust
// 迁移时直接加上 #[casbin]，默认不影响任何字段
#[casbin]
#[derive(Debug, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct DeviceVO {
    pub id: i64,
    pub serial_number: String,
    pub department_id: String,
}
```

> **说明**：只有在 Casbin 后台显式配置了某个角色对某个字段的 deny 规则后，该字段才会被过滤。迁移期间无需额外处理权限策略。

---

## 9. 迁移检查清单

每迁移一个接口后，逐项检查：

| # | 检查项 | 状态 |
|---|-------|------|
| 1 | 路径完全一致（`/device/create` 而非 `/api/device/create`） | ☐ |
| 2 | HTTP 方法一致（GET/POST） | ☐ |
| 3 | 请求参数名一致（camelCase：`serialNumber` 而非 `serial_number`） | ☐ |
| 4 | 请求体 JSON 字段名一致（`#[serde(rename_all = "camelCase")]`） | ☐ |
| 5 | 响应体 JSON 字段名一致（`#[serde(rename_all = "camelCase")]`） | ☐ |
| 6 | 响应包装方式一致（ResultDTO 包装 / 直接返回 / 返回数组） | ☐ |
| 7 | ResultDTO 的字段名一致（`status`/`message`/`data`） | ☐ |
| 8 | 字段类型一致（id 是数字还是字符串） | ☐ |
| 9 | null 处理一致（null 字段是输出还是跳过） | ☐ |
| 10 | 日期格式一致（时间戳 / 格式化字符串 / ISO） | ☐ |
| 11 | 分页结构一致（使用 SpringPage 兼容结构） | ☐ |
| 12 | HTTP 状态码一致 | ☐ |
| 13 | Python 对比测试脚本执行通过 | ☐ |
| 14 | 业务提交后影响的查询接口数据变化符合预期 | ☐ |
| 15 | `#[casbin]` 暂不启用（等所有接口测试通过后再开启） | ☐ |
