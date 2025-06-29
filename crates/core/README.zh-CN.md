# Core 模块

`core` 模块是应用程序的核心模块，包含基础功能如错误处理、JWT 认证、条件判断等。该模块为其他模块提供基础支持，确保应用程序的稳定性和安全性。

## 功能概述
- **错误处理**: 提供统一的错误处理机制，方便开发者捕获和处理异常。
- **JWT 认证**: 提供 JWT 认证功能，确保用户身份的安全验证。
- **条件判断**: 提供条件判断功能，帮助开发者在代码中进行逻辑判断。

## 使用说明

### 错误处理
`error.rs` 中定义了错误处理结构体 `AppError`，用于统一处理应用程序中的错误。

```rust
use core::error::AppError;

fn some_function() -> Result<(), AppError> {
    if some_condition {
        Ok(())
    } else {
        Err(AppError::new("An error occurred"))
    }
}
```

### JWT 认证
`jwt.rs` 中定义了 JWT 认证功能，包括生成和验证 JWT 令牌。

```rust
use core::jwt::JwtService;

let jwt_service = JwtService::new("secret");
let token = jwt_service.generate_token("user_id").unwrap();
let claims = jwt_service.verify_token(&token).unwrap();
println!("Claims: {:?}", claims);
```

### 条件判断
`condition.rs` 中提供了条件判断功能，帮助开发者在代码中进行逻辑判断。

```rust
use core::condition::Condition;

let condition = Condition::new();
if condition.is_true() {
    println!("Condition is true");
} else {
    println!("Condition is false");
}
```

## 详细说明

### `error.rs`
- **AppError**: 错误处理结构体，包含以下功能：
  - `new`: 创建一个新的错误实例。
  - `message`: 获取错误信息。

### `jwt.rs`
- **JwtService**: JWT 认证服务，包含以下功能：
  - `new`: 创建一个新的 JWT 服务实例。
  - `generate_token`: 生成 JWT 令牌。
  - `verify_token`: 验证 JWT 令牌并返回声明。

### `condition.rs`
- **Condition**: 条件判断结构体，包含以下功能：
  - `new`: 创建一个新的条件判断实例。
  - `is_true`: 判断条件是否为真。

## 贡献
欢迎提交 Pull Request 或 Issue 来改进本项目。

## 许可证
本项目采用 MIT 许可证，详情请参阅 `LICENSE` 文件。 