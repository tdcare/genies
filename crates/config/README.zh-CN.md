# Config 模块

`config` 模块提供配置管理功能，支持从 YAML 文件中读取配置，并提供了日志配置等子模块。该模块帮助开发者轻松管理应用程序的配置，确保配置的灵活性和可维护性。

## 功能概述
- **配置读取**: 支持从 YAML 文件中读取配置，并自动解析为 Rust 结构体。
- **日志配置**: 提供日志级别的配置，方便开发者根据需求调整日志输出。
- **多源配置**: 支持从多个配置源读取配置，并合并为一个统一的配置对象。

## 使用说明

### 基本配置读取
`app_config.rs` 中定义了应用程序的主要配置结构体 `ApplicationConfig`，支持从 YAML 文件中读取配置。

```rust
use config::app_config::ApplicationConfig;

let app_config = ApplicationConfig::from_sources("./application.yml").unwrap();
println!("Server name: {}", app_config.server_name);
```

### 日志配置
`log_config.rs` 中定义了日志配置结构体 `LogConfig`，支持从 YAML 文件中读取日志级别配置。

```rust
use config::log_config::LogConfig;

let log_config = LogConfig::from_sources("./application.yml").unwrap();
println!("Log level: {}", log_config.log_level);
```

### 多源配置
`lib.rs` 中提供了 `from_sources` 方法，支持从多个配置源读取配置，并合并为一个统一的配置对象。

```rust
use config::Config;

let config = Config::from_sources(&["./application.yml", "./override.yml"]).unwrap();
println!("Merged config: {:?}", config);
```

### 配置示例
你可以在 `application.yml` 中定义应用程序的配置，例如服务器名称和日志级别。

```yaml
server:
  name: "MyServer"
log:
  level: "info"
```

### 示例代码
`examples` 目录下提供了多个配置示例，帮助开发者快速上手。

```rust
use config::examples::config_examples;

config_examples::run();
```

## 详细说明

### `app_config.rs`
- **ApplicationConfig**: 应用程序的主要配置结构体，包含以下字段：
  - `debug`: 是否启用调试模式，类型为 `bool`。
  - `server_name`: 服务器的名称，类型为 `String`。
  - `servlet_path`: 当前服务的路由前缀，类型为 `String`。
  - `server_url`: 当前服务的地址，类型为 `String`。
  - `gateway`: 网关地址，如果指定了合法的 HTTP 协议（以 `http://` 或 `https://` 开头），所有跨微服务访问都将通过网关进行；否则将通过 Dapr 方式进行访问，类型为 `Option<String>`。
  - `redis_url`: Redis 地址，类型为 `String`。
  - `redis_save_url`: 可持久化的 Redis 地址，类型为 `String`。
  - `database_url`: 数据库地址，类型为 `String`。
  - `max_connections`: 数据库连接池的最大连接数，类型为 `u32`。
  - `min_connections`: 数据库连接池的最小连接数，类型为 `u32`。
  - `wait_timeout`: 数据库连接池的等待超时时间，单位为毫秒，类型为 `u64`。
  - `create_timeout`: 数据库连接池的创建超时时间，单位为毫秒，类型为 `u64`。
  - `max_lifetime`: 数据库连接池的最大生存时间，单位为毫秒，类型为 `u64`。
  - `log_level`: 日志级别，类型为 `String`。
  - `white_list_api`: 白名单接口列表，类型为 `Vec<String>`。
  - `cache_type`: 权限缓存类型，类型为 `String`。
  - `keycloak_auth_server_url`: Keycloak 认证服务器地址，类型为 `String`。
  - `keycloak_realm`: Keycloak 领域名称，类型为 `String`。
  - `keycloak_resource`: Keycloak 资源名称，类型为 `String`。
  - `keycloak_credentials_secret`: Keycloak 凭证密钥，类型为 `String`。
  - `dapr_pubsub_name`: Dapr 发布订阅名称，类型为 `String`。
  - `dapr_pub_message_limit`: Dapr 发布消息限制，类型为 `i64`。
  - `dapr_cdc_message_period`: Dapr CDC 消息周期，类型为 `i64`。
  - `processing_expire_seconds`: 事件处理的过期时间，单位为秒，类型为 `i64`。
  - `record_reserve_minutes`: 记录保留时间，单位为分钟，类型为 `i64`。

## 贡献
欢迎提交 Pull Request 或 Issue 来改进本项目。

## 许可证
本项目采用 MIT 许可证，详情请参阅 `LICENSE` 文件。 