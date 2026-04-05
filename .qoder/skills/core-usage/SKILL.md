---
name: core-usage
description: Guide for using genies_core response models, error handling, and JWT utilities. Use when building HTTP handlers with RespVO/ResultDTO responses, implementing JWT authentication, or handling errors in the Genies framework.
---

# Core Module (genies_core)

## Overview

genies_core 是 Genies 框架的核心工具库，提供 HTTP 响应模型、错误处理、JWT 认证和条件表达式求值功能。纯库 crate，无 binary。

**核心特性：**
- 双响应模型：`RespVO<T>`（字符串 code）和 `ResultDTO<T>`（数字 status）
- Salvo Writer 集成，支持直接作为 Handler 返回值
- Keycloak JWT 验证（`get_keycloak_keys`、`verify_with_keycloak`）
- 统一错误处理（`Error` 类型 + `Result<T>` 别名）
- JSON 条件表达式求值（`ConditionTree`、`obj_test`）

## Response Models

### RespVO<T> - 主响应模型

字符串状态码格式，适合新 Rust 服务：

```rust
use genies_core::{RespVO, CODE_SUCCESS, CODE_FAIL};

// 结构
pub struct RespVO<T> {
    pub code: Option<String>,  // "SUCCESS" | "FAIL" | custom
    pub msg: Option<String>,
    pub data: Option<T>,
}

// 创建成功响应
let resp = RespVO::from(&user);

// 从 Result 创建
let resp = RespVO::from_result(&result);

// 创建错误响应
let resp = RespVO::from_error_info("VALIDATION_ERROR", "参数无效");

// Dapr pubsub 确认
resp.is_success()  // {"status": "SUCCESS"}
resp.is_retry()    // {"status": "RETRY"}
```

### ResultDTO<T> - Java 兼容模型

数字状态码格式，适合与 Java 服务互操作：

```rust
use genies_core::{ResultDTO, CODE_SUCCESS_I32, CODE_FAIL_I32};

// 结构
pub struct ResultDTO<T> {
    pub status: Option<i32>,   // 1 (成功) | 0 (失败)
    pub message: Option<String>,
    pub data: Option<T>,
}

// 创建成功响应
let resp = ResultDTO::success("操作完成", data);
let resp = ResultDTO::success_empty("完成");

// 创建错误响应
let resp = ResultDTO::error("参数不能为空");
let resp = ResultDTO::from_error_info(400, "验证失败");

// 自定义状态码
let resp = ResultDTO::from_code_message(200, "OK", &data);
```

### 在 Salvo Handler 中使用

```rust
use salvo::prelude::*;
use genies_core::{RespVO, ResultDTO};

#[endpoint]
async fn get_user(id: PathParam<i64>) -> RespVO<User> {
    match find_user(id.into_inner()).await {
        Ok(user) => RespVO::from(&user),
        Err(e) => RespVO::from_error("NOT_FOUND", &e),
    }
}

#[endpoint]
async fn create_user(body: JsonBody<CreateUserDto>) -> ResultDTO<String> {
    match create_user_service(body.into_inner()).await {
        Ok(id) => ResultDTO::success("用户创建成功", id),
        Err(e) => ResultDTO::from_error(400, &e),
    }
}
```

## Error Handling

```rust
use genies_core::error::Error;
use genies_core::Result;  // = std::result::Result<T, Error>

// 从字符串创建
let err = Error::from("发生错误");

// 从其他错误类型转换
let err: Error = io_error.into();      // std::io::Error
let err: Error = rbdc_error.into();    // rbdc::Error
let err: Error = poison_error.into();  // PoisonError<T>

// 在函数中使用
fn my_function() -> Result<String> {
    // 自动转换错误
    let data = std::fs::read_to_string("file.txt")?;
    Ok(data)
}
```

### ConfigError 类型

用于配置相关操作的专用错误类型：

```rust
use genies_core::error::ConfigError;

// 可用变体
ConfigError::ValidationError("message".into())
ConfigError::ParseError("message".into())
ConfigError::EnvError("message".into())
ConfigError::BuildError("message".into())
ConfigError::ReloadError("message".into())
ConfigError::FileError("message".into())
ConfigError::ConversionError("message".into())
```

## JWT Authentication

### 获取 Keycloak 公钥

```rust
use genies_core::jwt::{get_keycloak_keys, Keys};

let keys: Keys = get_keycloak_keys(
    "http://keycloak.example.com/auth/",
    "my-realm"
).await?;
// 调用 /realms/{realm}/protocol/openid-connect/certs
```

### 获取服务账号令牌

```rust
use genies_core::jwt::get_temp_access_token;

let token = get_temp_access_token(
    "http://keycloak.example.com/auth/",
    "my-realm",
    "my-client",         // client_id
    "client-secret"      // client_secret
).await?;
// 使用 client_credentials grant type
```

### 验证 JWT 令牌

```rust
use genies_core::jwt::{JWTToken, Keys};

// 方式1: 使用 Keycloak 公钥验证（推荐）
let jwt = JWTToken::verify_with_keycloak(&keys, &token)?;

// 方式2: 使用自定义 secret 验证
let jwt = JWTToken::verify("my-secret", &token)?;

// 访问令牌声明
println!("用户ID: {:?}", jwt.user_id);
println!("用户名: {:?}", jwt.preferred_username);
println!("角色: {:?}", jwt.roles);
println!("部门: {:?}", jwt.department_name);
println!("过期时间: {:?}", jwt.exp);
```

### JWTToken 字段

```rust
pub struct JWTToken {
    pub id: Option<String>,
    pub exp: Option<usize>,
    pub iat: Option<usize>,
    pub sub: Option<String>,
    pub preferred_username: Option<String>,
    pub name: Option<String>,
    pub user_id: Option<String>,
    pub roles: Option<Vec<String>>,
    pub groups: Option<Vec<String>>,
    pub department_name: Option<String>,
    pub department_code: Option<String>,
    pub department_id: Option<String>,
    // ... 更多 Keycloak 标准字段
}
```

## Condition Module

用于在运行时对 JSON 对象进行条件求值：

```rust
use genies_core::condition::{ConditionTree, obj_test};
use serde_json::json;

// 简单条件
let condition = ConditionTree {
    operator: Some("=".to_string()),
    propertyName: Some("status".to_string()),
    value: Some("active".to_string()),
    conditionTrees: None,
};

let obj = json!({"status": "active"});
assert!(obj_test(&obj, &condition));

// 复合条件 (AND)
let and_condition = ConditionTree {
    operator: Some("and".to_string()),
    propertyName: None,
    value: None,
    conditionTrees: Some(vec![
        ConditionTree { operator: Some("=".into()), propertyName: Some("status".into()), value: Some("active".into()), conditionTrees: None },
        ConditionTree { operator: Some(">".into()), propertyName: Some("age".into()), value: Some("18".into()), conditionTrees: None },
    ]),
};

// 数组操作
// arr_size_>  - 数组大小比较
// arr_exist_= - 数组中存在满足条件的元素
// arr_each_=  - 数组中所有元素都满足条件
```

**支持的操作符：**

| 类别 | 操作符 | 说明 |
|------|--------|------|
| 逻辑 | `and`, `or` | 逻辑与、或 |
| 相等 | `=`, `<>`, `!=` | 等于、不等于 |
| 比较 | `<`, `<=`, `>`, `>=` | 数值比较 |
| 字符串 | `contain`, `!contain` | 包含、不包含 |
| 数组 | `arr_size_*` | 数组大小（如 `arr_size_>`) |
| 数组 | `arr_exist_*` | 存在元素满足条件 |
| 数组 | `arr_each_*` | 所有元素满足条件 |

## Pagination (Spring Data Page Compatibility)

`genies_core::page` 模块提供与 Java Spring Data `Page<T>` 完全兼容的 JSON 分页结构，用于 Java → Rust 微服务迁移时保持分页接口 JSON 格式完全一致。

### Core Types

| 类型 | 说明 |
|------|------|
| `SpringPage<T>` | 兼容 Spring Data `Page<T>` 的分页容器 |
| `Pageable` | 兼容 Spring Data `Pageable` 的分页参数 |
| `Sort` | 兼容 Spring Data `Sort` 的排序信息 |

### Usage

```rust
use genies_core::page::SpringPage;
use rbatis::plugin::page::{Page, PageRequest};

// 从 RBatis Page 转换（不转换记录类型）
let rbatis_page: Page<MyEntity> = /* 查询结果 */;
let spring_page: SpringPage<MyEntity> = SpringPage::from(rbatis_page);

// 从 RBatis Page 转换（同时转换 Entity → VO）
let rbatis_page: Page<MyEntity> = /* 查询结果 */;
let spring_page: SpringPage<MyVO> = SpringPage::from_rbatis_page(rbatis_page, |e| e.into());

// 配合 ResultDTO 使用
res.render(Json(ResultDTO::success("查询成功", spring_page)));
```

### Page Number Conversion

RBatis 页码从 **1** 开始（`page_no = 1` 表示第一页），Spring Data 页码从 **0** 开始（`number = 0` 表示第一页）。`SpringPage::from` 和 `SpringPage::from_rbatis_page` 已自动处理此转换（`number = page_no - 1`），无需手动调整。

### JSON Output

序列化后的 JSON 与 Spring Data `Page<T>` 完全一致：

```json
{
  "content": [{"id": 1, "name": "test"}],
  "pageable": {
    "pageNumber": 0,
    "pageSize": 20,
    "sort": {"empty": true, "sorted": false, "unsorted": true},
    "offset": 0,
    "paged": true,
    "unpaged": false
  },
  "last": true,
  "totalPages": 1,
  "totalElements": 1,
  "first": true,
  "size": 20,
  "number": 0,
  "sort": {"empty": true, "sorted": false, "unsorted": true},
  "numberOfElements": 1,
  "empty": false
}
```

## Constants

```rust
use genies_core::{CODE_SUCCESS, CODE_FAIL, CODE_SUCCESS_I32, CODE_FAIL_I32};

// RespVO 使用
CODE_SUCCESS  // "SUCCESS"
CODE_FAIL     // "FAIL"

// ResultDTO 使用
CODE_SUCCESS_I32  // 1
CODE_FAIL_I32     // 0
```

## Configuration Requirements

JWT 功能需要 application.yml 中的 Keycloak 配置：

```yaml
keycloak_auth_server_url: "http://keycloak.example.com/auth/"
keycloak_realm: "my-realm"
keycloak_resource: "my-client"
keycloak_credentials_secret: "your-client-secret"
```

## Key Files

- [crates/core/src/lib.rs](file:///d:/tdcare/genies/crates/core/src/lib.rs) - 模块入口、RespVO、ResultDTO
- [crates/core/src/error.rs](file:///d:/tdcare/genies/crates/core/src/error.rs) - Error 类型定义
- [crates/core/src/jwt.rs](file:///d:/tdcare/genies/crates/core/src/jwt.rs) - JWT 工具函数
- [crates/core/src/condition.rs](file:///d:/tdcare/genies/crates/core/src/condition.rs) - 条件表达式求值
- [crates/core/src/page.rs](file:///d:/tdcare/genies/crates/core/src/page.rs) - Spring Data Page 兼容分页结构
