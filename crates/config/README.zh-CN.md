# genies_config

Genies 框架的配置管理模块，提供基于 YAML 的配置加载和派生宏支持。

## 概述

genies_config 为 Genies 微服务提供简单而强大的配置系统：

- **派生宏集成**：使用 `#[derive(ConfigCore)]` 自动加载配置
- **YAML 配置**：从 `application.yml` 文件加载设置
- **环境变量**：通过环境变量覆盖配置值
- **类型安全访问**：强类型配置结构体
- **默认值**：支持 `#[config(default = "...")]` 属性
- **验证**：内置 `#[config(validate(...))]` 验证支持

## 核心组件

| 组件 | 文件 | 说明 |
|------|------|------|
| `ApplicationConfig` | app_config.rs | 主应用配置结构体，包含所有字段 |
| `LogConfig` | log_config.rs | 日志配置，含 `init_log()` 函数 |
| `ConfigCore` 宏 | genies_derive | 自动配置加载的派生宏 |

## ApplicationConfig 字段说明

| 字段 | 类型 | 说明 |
|------|------|------|
| `debug` | `bool` | 调试模式标志 |
| `server_name` | `String` | 微服务名称 |
| `servlet_path` | `String` | 服务路由前缀 |
| `server_url` | `String` | 服务器绑定地址（如 "0.0.0.0:5800"） |
| `gateway` | `Option<String>` | 跨服务调用的网关 URL |
| `redis_url` | `String` | Redis 连接 URL（业务缓存） |
| `redis_save_url` | `String` | 可持久化 Redis URL |
| `database_url` | `String` | MySQL 连接 URL |
| `max_connections` | `u32` | 数据库连接池最大连接数 |
| `min_connections` | `u32` | 数据库连接池最小连接数 |
| `wait_timeout` | `u64` | 数据库连接等待超时（秒） |
| `create_timeout` | `u64` | 数据库连接创建超时（秒） |
| `max_lifetime` | `u64` | 数据库连接最大生命周期（秒） |
| `log_level` | `String` | 日志级别过滤器（如 "debug,flyway=info"） |
| `white_list_api` | `Vec<String>` | API 白名单（无需认证） |
| `cache_type` | `String` | 缓存后端："redis" 或 "mem" |
| `keycloak_auth_server_url` | `String` | Keycloak 服务器 URL |
| `keycloak_realm` | `String` | Keycloak realm 名称 |
| `keycloak_resource` | `String` | Keycloak 客户端 ID |
| `keycloak_credentials_secret` | `String` | Keycloak 客户端密钥 |
| `dapr_pubsub_name` | `String` | Dapr pubsub 组件名称 |
| `dapr_pub_message_limit` | `i64` | 每批次发布消息的最大数量 |
| `dapr_cdc_message_period` | `i64` | CDC 消息周期（毫秒） |
| `processing_expire_seconds` | `i64` | 消息处理超时时间 |
| `record_reserve_minutes` | `i64` | 消息记录保留时间 |

## 快速开始

### 1. 添加依赖

```toml
[dependencies]
genies_config = { path = "../path/to/genies_config" }
genies_derive = { path = "../path/to/genies_derive" }
serde = { version = "1.0", features = ["derive"] }
```

### 2. 创建配置文件

在项目根目录创建 `application.yml`：

```yaml
debug: true
server_name: "my-service"
servlet_path: "/api"
server_url: "0.0.0.0:5800"

# Redis
cache_type: "redis"
redis_url: "redis://:password@127.0.0.1:6379"
redis_save_url: "redis://:password@127.0.0.1:6379"

# 数据库
database_url: "mysql://user:pass@127.0.0.1:3306/mydb"
max_connections: 20
min_connections: 0
wait_timeout: 60
create_timeout: 120
max_lifetime: 1800

# 日志
log_level: "debug,flyway=info"

# Keycloak
keycloak_auth_server_url: "http://keycloak.example.com/auth/"
keycloak_realm: "my-realm"
keycloak_resource: "my-client"
keycloak_credentials_secret: "client-secret"

# Dapr
dapr_pubsub_name: "messagebus"
dapr_pub_message_limit: 50
dapr_cdc_message_period: 5000
processing_expire_seconds: 60
record_reserve_minutes: 10080

# API 白名单
white_list_api:
  - "/"
  - "/actuator/*"
  - "/dapr/*"
```

### 3. 加载配置

```rust
use genies_config::app_config::ApplicationConfig;

fn main() {
    let config = ApplicationConfig::from_sources("./application.yml").unwrap();
    
    println!("服务器: {}", config.server_url);
    println!("调试模式: {}", config.debug);
}
```

## 自定义配置结构体

### 基本用法

```rust
use genies_derive::ConfigCore;
use serde::Deserialize;

#[derive(ConfigCore, Debug, Deserialize)]
pub struct MyConfig {
    pub host: String,
    pub port: u16,
    pub app_name: String,
}

// 从文件加载
let config = MyConfig::from_sources("./config.yml")?;
```

### 带默认值

```rust
#[derive(ConfigCore, Debug, Deserialize)]
pub struct ServerConfig {
    #[config(default = "localhost")]
    pub host: String,
    
    #[config(default = 8080)]
    pub port: u16,
}
```

### 带验证

```rust
#[derive(ConfigCore, Debug, Deserialize)]
pub struct ValidatedConfig {
    #[config(default = 8080)]
    #[config(validate(range(min = 1, max = 65535)))]
    pub port: u16,
}
```

### 数组字段

```rust
#[derive(ConfigCore, Debug, Deserialize)]
pub struct ArrayConfig {
    #[config(default = "topic1,topic2,topic3")]
    pub topics: Vec<String>,
    
    #[config(default = "1,2,3")]
    pub numbers: Vec<i32>,
}
```

### 可选字段

```rust
#[derive(ConfigCore, Debug, Deserialize)]
pub struct OptionalConfig {
    #[config(default = "guest")]
    pub username: Option<String>,
    
    pub password: Option<String>,  // 无默认值，真正可选
}
```

## 日志配置

```rust
use genies_config::log_config::{LogConfig, init_log};

// 从 application.yml 初始化日志
init_log();

// 或手动加载 LogConfig
let log_config = LogConfig::from_sources("./application.yml")?;
println!("日志级别: {}", log_config.log_level);
```

### 日志级别格式

`log_level` 字段支持 tracing-subscriber 过滤器语法：

```yaml
# 简单级别
log_level: "debug"

# 按模块设置级别
log_level: "debug,flyway=info,sqlx=warn"

# 带 span 过滤
log_level: "debug,flyway=info,[my_span]=trace"
```

## 环境变量覆盖

环境变量自动覆盖 YAML 值。变量名从字段名转换为 SCREAMING_SNAKE_CASE：

```bash
# 覆盖 server_url
export SERVER_URL="0.0.0.0:9000"

# 覆盖 database_url
export DATABASE_URL="mysql://prod:pass@prod-db:3306/app"

# 覆盖数组（逗号分隔）
export WHITE_LIST_API="/,/health,/api/*"
```

## Gateway 配置

`gateway` 字段控制跨服务通信方式：

```yaml
# 使用 HTTP 网关
gateway: "http://gateway.example.com:6002"

# 使用 Dapr sidecar（任何非 HTTP 值）
gateway: "dapr"
```

当 `gateway` 以 `http://` 或 `https://` 开头时，所有跨服务调用通过网关进行。否则使用 Dapr 服务调用。

## 依赖项

- **genies_derive** - `ConfigCore` 派生宏
- **serde** - 反序列化
- **tracing-subscriber** - 日志初始化
- **config**（可选）- 高级配置加载

## 与其他 Crate 集成

- **genies_context**：`CONTEXT.config()` 返回 `ApplicationConfig`
- **genies_cache**：使用 `redis_url`、`redis_save_url`、`cache_type`
- **genies_core**：JWT 函数使用 Keycloak 配置字段
- **genies_auth**：使用 `white_list_api` 进行认证绕过

## 配置示例

参见 `crates/config/examples/` 获取更多示例：

- `config_examples.rs` - 各种配置模式
- `config/basic.yml` - 基础配置
- `config/array.yml` - 数组字段配置
- `config/optional.yml` - 可选字段处理
- `config/complex.yml` - 复杂配置场景

## 许可证

请参阅项目根目录的许可证信息。
