---
name: api-conventions
description: Genies 前后端接口规范。Use when designing API contracts, defining field naming conventions, date/time formats, response models, pagination, error handling, or ID strategies between Genies backend and frontend (Web/Android/OHOS).
---

# Genies 前后端接口规范

本文档定义 Genies 微服务后端与前端（Web / Android / OHOS 等）之间的 HTTP 接口约定。所有微服务必须遵循本规范，确保多端一致性。

## 1. 字段命名约定

| 层级 | 命名风格 | 示例 |
|------|---------|------|
| Rust 源码（struct 字段） | snake_case | `created_at`, `app_name` |
| JSON 传输（HTTP 请求/响应） | snake_case | `created_at`, `app_name` |
| URL 路径 | kebab-case | `/api/auth-admin/user-roles` |
| 数据库列名 | snake_case | `created_at`, `app_name` |

Rust 端 serde 默认使用 snake_case，无需额外 `rename_all` 属性，与 JSON 字段名天然一致：

```rust
#[derive(Serialize, Deserialize)]
pub struct UserVO {
    pub user_name: String,       // JSON: "user_name"
    pub department_id: String,   // JSON: "department_id"
    pub created_at: Option<rbdc::DateTime>, // JSON: "created_at"
}
```

**前端（Web/Android/OHOS）统一使用 snake_case** 进行字段访问，与后端 Rust 命名一致。

## 2. 响应模型

项目保留两套响应模型，按场景选用：

### RespVO<T> — Rust 微服务原生响应（推荐）

```json
{
  "code": "SUCCESS",
  "msg": null,
  "data": { ... }
}
```

| 字段 | 类型 | 说明 |
|------|------|------|
| `code` | `String` | `"SUCCESS"` 成功，`"FAIL"` 失败，或自定义业务码 |
| `msg` | `String?` | 错误描述，成功时为 null |
| `data` | `T?` | 业务数据，失败时为 null |

**适用场景**：所有 Rust 微服务直接对前端的接口。

**前端判断成功**：
```javascript
if (response.code === "SUCCESS") { /* 成功 */ }
```

### ResultDTO<T> — Java 兼容响应

```json
{
  "status": 1,
  "message": null,
  "data": { ... }
}
```

| 字段 | 类型 | 说明 |
|------|------|------|
| `status` | `i32` | `1` 成功，`0` 失败 |
| `message` | `String?` | 错误描述 |
| `data` | `T?` | 业务数据 |

**适用场景**：需要与 Java 端 Spring 接口保持一致的混合部署微服务。

**前端判断成功**：
```javascript
if (response.status === 1) { /* 成功 */ }
```

## 3. 日期时间格式

### 3.1 统一规则

**所有日期时间字段以毫秒时间戳（i64）传输**，与 Java `Long` 类型一致。

```json
{
  "created_at": 1719484800000,
  "updated_at": 1719571200000
}
```

**时区**：时间戳始终表示 UTC 毫秒数，各端按需转换为本地时区显示。

**仅日期（无时间）** 的场景（如生日、入职日期），使用 `yyyy-MM-dd` 字符串格式：
```json
{
  "birthday": "2024-06-27"
}
```

### 3.2 各端解析方式

| 平台 | 解析代码 |
|------|---------|
| Web (JS/TS) | `new Date(1719484800000)` |
| Android (Kotlin) | `Date(1719484800000L)` 或 `Instant.ofEpochMilli(1719484800000L)` |
| OHOS (ArkTS) | `new Date(1719484800000)` |

### 3.3 Rust 端实现

在项目中创建 `date_format` 共享模块（如 `src/model/date_format.rs`），提供序列化/反序列化函数：

```rust
// date_format.rs
use serde::{self, Deserialize, Deserializer, Serializer};

/// 序列化 Option<rbdc::DateTime> 为毫秒时间戳数字
pub fn serialize_option_datetime<S>(
    value: &Option<rbdc::DateTime>,
    serializer: S,
) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    match value {
        Some(dt) => {
            let millis = dt.unix_timestamp_millis() as i64;
            serializer.serialize_i64(millis)
        }
        None => serializer.serialize_none(),
    }
}

/// 反序列化 Option<rbdc::DateTime>，支持毫秒时间戳或 ISO 8601 字符串
pub fn deserialize_option_datetime<'de, D>(
    deserializer: D,
) -> Result<Option<rbdc::DateTime>, D::Error>
where
    D: Deserializer<'de>,
{
    use serde::de;

    #[derive(Deserialize)]
    #[serde(untagged)]
    enum DateTimeRepr {
        Millis(i64),
        Str(String),
    }

    let repr = Option::<DateTimeRepr>::deserialize(deserializer)?;
    match repr {
        None => Ok(None),
        Some(DateTimeRepr::Millis(ms)) => {
            let secs = ms / 1000;
            let nanos = ((ms % 1000) * 1_000_000) as u32;
            let dt_utc = chrono::DateTime::from_timestamp(secs, nanos)
                .ok_or_else(|| de::Error::custom(format!("invalid timestamp millis: {}", ms)))?;
            let s = dt_utc.naive_utc().format("%Y-%m-%dT%H:%M:%S").to_string();
            let dt: rbdc::DateTime = s.parse().map_err(de::Error::custom)?;
            Ok(Some(dt))
        }
        Some(DateTimeRepr::Str(s)) => {
            if s.is_empty() {
                return Ok(None);
            }
            let dt: rbdc::DateTime = s.parse().map_err(de::Error::custom)?;
            Ok(Some(dt))
        }
    }
}
```

在 VO/DTO 中使用：

```rust
use crate::model::date_format::{serialize_option_datetime, deserialize_option_datetime};

pub struct UserVO {
    pub id: Option<i64>,
    pub name: Option<String>,
    #[serde(
        serialize_with = "serialize_option_datetime",
        deserialize_with = "deserialize_option_datetime"
    )]
    pub created_at: Option<rbdc::DateTime>,
    #[serde(
        serialize_with = "serialize_option_datetime",
        deserialize_with = "deserialize_option_datetime"
    )]
    pub updated_at: Option<rbdc::DateTime>,
}
```

## 4. 分页约定

### 4.1 响应格式（SpringPage）

统一使用 `SpringPage<T>` 格式，兼容 Spring Data `Page<T>`：

```json
{
  "content": [ ... ],
  "totalElements": 100,
  "totalPages": 10,
  "size": 10,
  "number": 0,
  "first": true,
  "last": false,
  "numberOfElements": 10,
  "empty": false,
  "pageable": {
    "pageNumber": 0,
    "pageSize": 10,
    "sort": { "empty": true, "sorted": false, "unsorted": true },
    "offset": 0,
    "paged": true,
    "unpaged": false
  },
  "sort": { "empty": true, "sorted": false, "unsorted": true }
}
```

| 字段 | 说明 |
|------|------|
| `content` | 当前页数据列表 |
| `totalElements` | 总记录数 |
| `totalPages` | 总页数 |
| `size` | 每页条数 |
| `number` | 当前页码（从 0 开始） |
| `first` | 是否第一页 |
| `last` | 是否最后一页 |

### 4.2 请求参数

通过 query string 传递：

| 参数 | 类型 | 说明 |
|------|------|------|
| `page` | `u64` | 页码（从 0 开始，与 Spring Data 一致） |
| `size` | `u64` | 每页条数，默认 20 |
| `sort` | `String` | 可选，格式 `field,direction`（如 `created_at,desc`） |

示例请求：`GET /api/v1/users?page=0&size=20&sort=created_at,desc`

## 5. 错误响应约定

### 5.1 HTTP 状态码

| 状态码 | 语义 |
|--------|------|
| `200` | 请求成功（业务结果由 code/status 判断） |
| `400` | 参数校验失败 |
| `401` | 未认证（缺少/过期 Token） |
| `403` | 无权限（已认证但无权访问） |
| `404` | 资源不存在 |
| `500` | 服务器内部错误 |

### 5.2 错误响应体

错误时统一返回格式：

```json
{
  "code": "FAIL",
  "msg": "具体错误描述信息",
  "data": null
}
```

前端统一拦截器示例（Axios）：
```typescript
axios.interceptors.response.use(
  (res) => {
    if (res.data.code === "FAIL") {
      // 业务错误提示
      showMessage(res.data.msg);
    }
    return res;
  },
  (err) => {
    if (err.response?.status === 401) {
      // 跳转登录
    }
    return Promise.reject(err);
  }
);
```

## 6. ID 字段约定

- ID 由雪花算法生成（`genies::next_id()`），底层类型为 `i64`，输出 ≤15 位十进制数字
- **JSON 中直接以数字类型传输**，`genies::next_id()` 生成的 ID 已控制在 JavaScript 安全整数范围内（≤2^53-1），前端 `Number` 无精度风险
- 前端以 `number` 类型接收和传递 ID
- Rust 端 VO 层直接使用 `i64`，无需转为 `String`

```rust
// Entity 层 — i64
pub struct UserEntity {
    pub id: Option<i64>,
}

// VO 层 — 直接使用 i64，serde 序列化为 JSON 数字
pub struct UserVO {
    pub id: Option<i64>,
}
```

前端接口类型定义（TypeScript）：
```typescript
interface UserVO {
  id: number;            // genies::next_id() 输出 ≤15 位，Number 安全
  name: string;
  department_id: string;
}
```

## 7. 请求体约定

- **Content-Type**：统一使用 `application/json`
- **字段命名**：snake_case（与 Rust 字段名保持完全一致）
- **可选字段**：可省略或传 `null`
- **布尔字段**：使用 `true/false`，不使用 `0/1`
- **枚举字段**：使用数字编码（与 Java 端保持一致），在文档中注明各值含义

请求示例：
```json
{
  "user_name": "zhangsan",
  "department_id": "1234567890",
  "is_active": true,
  "role_ids": ["111", "222"]
}
```

## 8. 认证约定

- Token 通过 `Authorization: Bearer <token>` 请求头传递
- Token 格式为 JWT，由 auth-admin 服务签发
- Token 过期后前端收到 `401` 响应，应跳转登录页或刷新 Token
- 需要认证的接口，前端必须在请求头中携带 Token

```
GET /api/v1/users HTTP/1.1
Authorization: Bearer eyJhbGciOiJIUzI1NiIs...
```

## 9. 多端通用接口类型定义参考

### TypeScript (Web)

```typescript
// 响应包装
interface RespVO<T> {
  code: string | null;
  msg: string | null;
  data: T | null;
}

// 分页响应
interface SpringPage<T> {
  content: T[];
  total_elements: number;
  total_pages: number;
  size: number;
  number: number;
  first: boolean;
  last: boolean;
  number_of_elements: number;
  empty: boolean;
}

// 分页请求参数
interface PageParams {
  page: number;   // 从 0 开始
  size: number;   // 默认 20
  sort?: string;  // "field,direction"
}
```

### Kotlin (Android)

```kotlin
data class RespVO<T>(
    val code: String?,
    val msg: String?,
    val data: T?
)

data class SpringPage<T>(
    val content: List<T>,
    val total_elements: Long,
    val total_pages: Int,
    val size: Int,
    val number: Int,
    val first: Boolean,
    val last: Boolean,
    val number_of_elements: Int,
    val empty: Boolean
)
```

### ArkTS (OHOS)

```typescript
// 与 TypeScript 定义一致，使用 interface 即可
```

## Related Skills

- **core-usage** — RespVO / ResultDTO 响应模型、错误处理
- **genies-ddd-microservice** — DDD 微服务开发完整指南
- **auth-usage** — JWT 认证与 Casbin 权限管理
- **rbatis-usage** — RBatis ORM 与分页查询
