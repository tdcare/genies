# Genies 模块

`genies` 模块是应用程序的主模块，负责整合各个子模块的功能，并提供应用程序的主要逻辑。该模块帮助开发者在应用程序中轻松管理全局状态、用户会话和权限。

## 功能概述
- **应用上下文**: 管理应用程序的全局状态和配置。
- **认证上下文**: 管理用户认证信息，包括用户身份和权限。
- **授权管理**: 提供权限验证功能，确保用户只能访问其有权访问的资源。

## 使用说明

### 应用上下文
`app_context.rs` 中定义了应用上下文结构体 `ApplicationContext`，用于管理应用程序的全局状态和配置。

```rust
use genies::app_context::ApplicationContext;

let app_context = ApplicationContext::new();
app_context.set_config("key", "value").unwrap();
let config_value = app_context.get_config("key").unwrap();
println!("Config value: {}", config_value);
```

### 认证上下文
`auth.rs` 中定义了认证上下文结构体 `AuthContext`，用于管理用户认证信息。

```rust
use genies::auth::AuthContext;

let auth_context = AuthContext::new();
let user = auth_context.authenticate("token").unwrap();
println!("Authenticated user: {:?}", user);
```

### 授权管理
`auth.rs` 中提供了权限验证功能，确保用户只能访问其有权访问的资源。

```rust
use genies::auth::AuthContext;

let auth_context = AuthContext::new();
let has_access = auth_context.has_permission("user_id", "resource_id").unwrap();
println!("Has access: {}", has_access);
```

## 详细说明

### `app_context.rs`
- **ApplicationContext**: 应用上下文结构体，包含以下字段：
  - `config`: 应用程序的配置项，类型为 `ApplicationConfig`。
  - `rbatis`: 数据库操作对象，类型为 `RBatis`。
  - `cache_service`: 缓存服务对象，类型为 `CacheService`。
  - `redis_save_service`: 可持久化的 Redis 缓存服务对象，类型为 `CacheService`。
  - `keycloak_keys`: Keycloak 认证密钥，类型为 `Keys`。

### `auth.rs`
- **AuthContext**: 认证上下文结构体，包含以下功能：
  - `authenticate`: 验证用户身份并返回用户信息。
  - `has_permission`: 检查用户是否具有访问特定资源的权限。

### `lib.rs`
- **全局变量初始化**: 在 `lib.rs` 中定义了全局变量 `APP_CONTEXT`，并使用 `lazy_static` 宏进行初始化。`lazy_static` 宏确保全局变量在首次访问时进行初始化，并且在整个应用程序生命周期中保持单例。

```rust
use genies::app_context::ApplicationContext;
use lazy_static::lazy_static;

lazy_static! {
    pub static ref APP_CONTEXT: ApplicationContext = ApplicationContext::new();
}
```

### `ApplicationContext` 的 `new()` 方法实现
`ApplicationContext` 的 `new()` 方法用于创建一个新的 `ApplicationContext` 实例。以下是 `new()` 方法的详细实现过程：

1. **创建实例**: `new()` 方法首先创建一个新的 `ApplicationContext` 实例。
2. **初始化字段**:
   - `config`: 初始化 `ApplicationConfig` 实例，用于存储应用程序的配置项。
   - `rbatis`: 初始化 `RBatis` 实例，用于数据库操作。
   - `cache_service`: 初始化 `CacheService` 实例，用于缓存服务。
   - `redis_save_service`: 初始化 `CacheService` 实例，用于可持久化的 Redis 缓存服务。
   - `keycloak_keys`: 初始化 `Keys` 实例，用于 Keycloak 认证密钥。
3. **返回实例**: 最后，`new()` 方法返回初始化后的 `ApplicationContext` 实例。

```rust
impl ApplicationContext {
    pub fn new() -> Self {
        ApplicationContext {
            config: ApplicationConfig::new(),
            rbatis: RBatis::new(),
            cache_service: CacheService::new(),
            redis_save_service: CacheService::new(),
            keycloak_keys: Keys::new(),
        }
    }
}
```

### 全局变量中存储的内容
- **配置项**: 存储在 `config` 字段中，类型为 `ApplicationConfig`。
- **数据库操作对象**: 存储在 `rbatis` 字段中，类型为 `RBatis`。
- **缓存服务对象**: 存储在 `cache_service` 字段中，类型为 `CacheService`。
- **可持久化的 Redis 缓存服务对象**: 存储在 `redis_save_service` 字段中，类型为 `CacheService`。
- **Keycloak 认证密钥**: 存储在 `keycloak_keys` 字段中，类型为 `Keys`。

### 使用全局变量
你可以在应用程序的任何地方使用 `APP_CONTEXT` 全局变量来访问应用上下文。

```rust
use genies::APP_CONTEXT;

let config_value = APP_CONTEXT.get_config("key").unwrap();
println!("Config value: {}", config_value);
```

## 贡献
欢迎提交 Pull Request 或 Issue 来改进本项目。

## 许可证
本项目采用 MIT 许可证，详情请参阅 `LICENSE` 文件。 