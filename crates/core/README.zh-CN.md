# genies_core

Genies 框架的核心工具库，提供响应模型、错误处理、JWT 认证和条件表达式求值功能。

## 概述

genies_core 为基于 Genies 的微服务提供基础构建模块：

- **响应模型**：`RespVO<T>` 和 `ResultDTO<T>` 用于 HTTP 响应格式化
- **错误处理**：统一的 `Error` 类型和 `Result<T>` 类型别名
- **JWT 工具**：Keycloak 集成，支持令牌验证
- **条件引擎**：基于 JSON 的条件表达式求值

## 核心特性

- **双响应模型**：字符串状态码（`RespVO`）和数字状态码（`ResultDTO`）格式可选
- **Salvo Writer 集成**：两种响应类型均实现 `Writer` trait，支持直接渲染
- **Keycloak JWT 支持**：从 Keycloak 服务器获取密钥并验证令牌
- **灵活错误处理**：支持从多种错误类型转换（`io::Error`、`rbdc::Error` 等）
- **Dapr Pubsub 支持**：内置 Dapr 消息确认响应辅助方法

## 核心 API 参考

### 响应模型

| 类型 | 状态字段 | 说明 |
|------|----------|------|
| `RespVO<T>` | `code: String` | 主模型，使用 `CODE_SUCCESS` / `CODE_FAIL` |
| `ResultDTO<T>` | `status: i32` | Java 兼容模型，使用 `1`（成功）/ `0`（失败） |

### 常量

```rust
pub const CODE_SUCCESS: &str = "SUCCESS";
pub const CODE_FAIL: &str = "FAIL";
pub const CODE_SUCCESS_I32: i32 = 1;
pub const CODE_FAIL_I32: i32 = 0;
```

### RespVO<T> 方法

```rust
// 从成功数据创建
RespVO::from(&data)

// 从 Result 创建
RespVO::from_result(&result)

// 创建带自定义 code 的错误响应
RespVO::from_error(code, &error)
RespVO::from_error_info(code, "错误信息")

// Dapr pubsub 响应
resp.is_success()  // {"status": "SUCCESS"}
resp.is_retry()    // {"status": "RETRY"}
```

### ResultDTO<T> 方法

```rust
// 创建成功响应
ResultDTO::success("操作完成", data)
ResultDTO::success_empty("完成")

// 创建错误响应
ResultDTO::error("参数不能为空")
ResultDTO::from_error(code, &error)
ResultDTO::from_error_info(code, "消息")

// 创建自定义 code 和消息
ResultDTO::from_code_message(200, "OK", &data)
```

### Error 类型

```rust
use genies_core::error::Error;

// 从字符串创建
let err = Error::from("发生错误");

// 从其他错误转换
let err: Error = io_error.into();
let err: Error = rbdc_error.into();
```

### JWT 模块

```rust
use genies_core::jwt::{get_keycloak_keys, get_temp_access_token, JWTToken, Keys};

// 获取 Keycloak 公钥
let keys: Keys = get_keycloak_keys(
    "http://keycloak.example.com/auth/", 
    "my-realm"
).await?;

// 获取服务账号访问令牌
let token = get_temp_access_token(
    "http://keycloak.example.com/auth/",
    "my-realm",
    "my-client",
    "client-secret"
).await?;

// 使用 Keycloak 密钥验证令牌
let jwt = JWTToken::verify_with_keycloak(&keys, &token)?;

// 访问令牌声明
println!("用户: {}", jwt.preferred_username.unwrap_or_default());
println!("角色: {:?}", jwt.roles);
```

### 条件模块

```rust
use genies_core::condition::{ConditionTree, obj_test};
use serde_json::json;

// 定义条件树
let condition = ConditionTree {
    operator: Some("and".to_string()),
    propertyName: None,
    value: None,
    conditionTrees: Some(vec![
        ConditionTree {
            operator: Some("=".to_string()),
            propertyName: Some("status".to_string()),
            value: Some("active".to_string()),
            conditionTrees: None,
        },
        ConditionTree {
            operator: Some(">".to_string()),
            propertyName: Some("age".to_string()),
            value: Some("18".to_string()),
            conditionTrees: None,
        },
    ]),
};

// 测试对象是否满足条件
let obj = json!({"status": "active", "age": 25});
let matches = obj_test(&obj, &condition);
```

**支持的操作符：**

| 类别 | 操作符 |
|------|--------|
| 逻辑 | `and`, `or` |
| 比较 | `=`, `<>`, `!=`, `<`, `<=`, `>`, `>=` |
| 字符串 | `contain`, `!contain` |
| 数组 | `arr_size_*`, `arr_exist_*`, `arr_each_*` |

## 快速开始

### 1. 添加依赖

```toml
[dependencies]
genies_core = { path = "../path/to/genies_core" }
```

### 2. 在 Salvo Handler 中使用

```rust
use salvo::prelude::*;
use genies_core::{RespVO, ResultDTO};

#[endpoint]
async fn get_user() -> RespVO<User> {
    let user = User { id: 1, name: "张三".into() };
    RespVO::from(&user)
}

#[endpoint]
async fn create_user() -> ResultDTO<String> {
    // 业务逻辑...
    ResultDTO::success("用户创建成功", "user_id_123".into())
}

#[endpoint]
async fn handle_error() -> ResultDTO<()> {
    ResultDTO::error("参数无效")
}
```

### 3. JWT 认证

```rust
use genies_core::jwt::{get_keycloak_keys, JWTToken};

async fn verify_request(token: &str) -> Result<JWTToken, Error> {
    let keys = get_keycloak_keys(
        &config.keycloak_auth_server_url,
        &config.keycloak_realm
    ).await?;
    
    JWTToken::verify_with_keycloak(&keys, token)
}
```

## RespVO 与 ResultDTO 选择指南

| 场景 | 推荐 |
|------|------|
| 新的纯 Rust 服务 | `RespVO<T>` |
| 与 Java 服务互操作 | `ResultDTO<T>` |
| 需要字符串错误码 | `RespVO<T>` |
| 需要数字状态码 | `ResultDTO<T>` |
| Dapr pubsub 处理器 | `RespVO<T>`（有 `is_success()`, `is_retry()`） |

## 配置说明

JWT 函数需要 `ApplicationConfig` 中的 Keycloak 参数：

```yaml
keycloak_auth_server_url: "http://keycloak.example.com/auth/"
keycloak_realm: "my-realm"
keycloak_resource: "my-client"
keycloak_credentials_secret: "your-client-secret"
```

## 依赖项

- **salvo** - Web 框架（Writer trait）
- **serde** / **serde_json** - 序列化
- **jsonwebtoken** - JWT 编解码
- **reqwest** - Keycloak HTTP 客户端
- **thiserror** - 错误派生宏

## 与其他 Crate 集成

- **genies_config**：JWT 函数使用 `ApplicationConfig` 参数
- **genies_context**：`CONTEXT.config()` 提供 Keycloak 配置
- **genies_auth**：`salvo_auth` 中间件使用 `JWTToken` 进行认证

## 许可证

请参阅项目根目录的许可证信息。
