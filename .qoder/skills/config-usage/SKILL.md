---
name: config-usage
description: Guide for using genies_config configuration management. Use when setting up application configuration, defining YAML config files, customizing ApplicationConfig fields, or configuring logging in Genies projects.
---

# Config Module (genies_config)

## Overview

genies_config 是 Genies 框架的配置管理库，提供基于 YAML 的配置加载和 `#[derive(ConfigCore)]` 宏支持。纯库 crate，无 binary。

**核心特性：**
- `ConfigCore` 派生宏自动生成 `from_sources()` 方法
- YAML 配置文件加载（`application.yml`）
- 环境变量覆盖
- 默认值支持（`#[config(default = "...")]`）
- 字段验证（`#[config(validate(...))]`）
- 日志初始化（`init_log()`）

## ApplicationConfig 完整字段

```rust
#[derive(ConfigCore, Debug, Deserialize)]
pub struct ApplicationConfig {
    // === 基础设置 ===
    pub debug: bool,                    // 调试模式
    pub server_name: String,            // 微服务名称
    pub servlet_path: String,           // 路由前缀
    pub server_url: String,             // 绑定地址 "0.0.0.0:5800"
    pub gateway: Option<String>,        // 网关 URL（HTTP 或 Dapr）

    // === Redis ===
    pub cache_type: String,             // "redis" | "mem"
    pub redis_url: String,              // 业务缓存 Redis
    pub redis_save_url: String,         // 持久化 Redis

    // === 数据库 ===
    pub database_url: String,           // MySQL URL
    pub max_connections: u32,           // 连接池最大
    pub min_connections: u32,           // 连接池最小
    pub wait_timeout: u64,              // 等待超时（秒）
    pub create_timeout: u64,            // 创建超时（秒）
    pub max_lifetime: u64,              // 连接生命周期（秒）

    // === 日志 ===
    pub log_level: String,              // "debug,flyway=info"

    // === 认证 ===
    pub white_list_api: Vec<String>,    // 白名单接口
    pub keycloak_auth_server_url: String,
    pub keycloak_realm: String,
    pub keycloak_resource: String,
    pub keycloak_credentials_secret: String,

    // === Dapr ===
    pub dapr_pubsub_name: String,
    pub dapr_pub_message_limit: i64,
    pub dapr_cdc_message_period: i64,
    pub processing_expire_seconds: i64,
    pub record_reserve_minutes: i64,
}
```

## YAML 配置文件模板

```yaml
# application.yml

# 基础设置
debug: true
server_name: "my-service"
servlet_path: "/api"
server_url: "0.0.0.0:5800"
gateway: "http://gateway.example.com:6002"

# 缓存
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
log_level: "debug,flyway=info,sqlx=warn"

# Keycloak
keycloak_auth_server_url: "http://keycloak.example.com/auth/"
keycloak_realm: "my-realm"
keycloak_resource: "my-client"
keycloak_credentials_secret: "your-secret"

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
  - "/daprsub/*"
```

## 加载配置

```rust
use genies_config::app_config::ApplicationConfig;

// 方式1: 直接加载
let config = ApplicationConfig::from_sources("./application.yml").unwrap();

// 方式2: 通过 CONTEXT（推荐）
use genies::context::CONTEXT;
let config = CONTEXT.config();
```

## 自定义配置结构体

### ConfigCore 宏使用

```rust
use genies_derive::ConfigCore;
use serde::Deserialize;

#[derive(ConfigCore, Debug, Deserialize)]
pub struct MyConfig {
    // 必填字段
    pub host: String,
    
    // 带默认值
    #[config(default = 8080)]
    pub port: u16,
    
    // 带验证
    #[config(default = 3600)]
    #[config(validate(range(min = 60, max = 86400)))]
    pub timeout: u64,
    
    // 数组字段，默认值用逗号分隔
    #[config(default = "topic1,topic2")]
    pub topics: Vec<String>,
    
    // 可选字段
    pub password: Option<String>,
}

// 使用
let config = MyConfig::from_sources("./config.yml")?;
```

### 对应 YAML 文件

```yaml
# config.yml
host: "example.com"
port: 9090
timeout: 7200
topics:
  - "events"
  - "logs"
password: "secret"
```

## 日志配置

```rust
use genies_config::log_config::init_log;

// 在 main() 开始时初始化
fn main() {
    init_log();  // 从 ./application.yml 读取 log_level
    
    // 或者手动初始化
    tracing_subscriber::fmt()
        .with_env_filter("debug,flyway=info")
        .init();
}
```

### log_level 格式

```yaml
# 全局 debug
log_level: "debug"

# 按模块设置
log_level: "info,my_crate=debug,sqlx=warn"

# 带 span 过滤
log_level: "debug,[my_span]=trace"

# 复杂配置
log_level: "debug,flyway=info,ddd_dapr=debug,[my_span]=trace"
```

## 环境变量覆盖

环境变量自动覆盖 YAML 配置。字段名转换规则：`snake_case` → `SCREAMING_SNAKE_CASE`

```bash
# 覆盖 server_url
export SERVER_URL="0.0.0.0:9000"

# 覆盖 database_url
export DATABASE_URL="mysql://prod:pass@prod-db:3306/app"

# 覆盖数组（逗号分隔）
export WHITE_LIST_API="/,/health,/api/*"
export TOPICS="prod/events,prod/logs"

# 覆盖数字
export PORT="443"
export MAX_CONNECTIONS="50"
```

## Gateway 配置策略

```yaml
# HTTP 网关模式 - 跨服务调用通过网关
gateway: "http://gateway.example.com:6002"

# Dapr 模式 - 使用 Dapr sidecar
gateway: "dapr"
gateway: ""
```

判断逻辑：
```rust
if gateway.starts_with("http://") || gateway.starts_with("https://") {
    // 通过 HTTP 网关调用
} else {
    // 通过 Dapr service invocation
}
```

## 配置验证

```rust
#[derive(ConfigCore, Debug, Deserialize)]
pub struct ValidatedConfig {
    // 范围验证
    #[config(validate(range(min = 1, max = 65535)))]
    pub port: u16,
    
    // 多个验证
    #[config(default = 100)]
    #[config(validate(range(min = 1, max = 1000)))]
    pub batch_size: u32,
}
```

## 与其他 Crate 的关系

| Crate | 使用的配置字段 |
|-------|---------------|
| genies_cache | `cache_type`, `redis_url`, `redis_save_url` |
| genies_core | `keycloak_*` 字段 |
| genies_auth | `white_list_api` |
| genies_context | 管理完整 `ApplicationConfig` |
| genies_ddd | `dapr_pubsub_name`, `dapr_*` 字段 |

## 示例代码

```rust
use genies_config::app_config::ApplicationConfig;
use genies_config::log_config::init_log;

#[tokio::main]
async fn main() {
    // 初始化日志
    init_log();
    
    // 加载配置
    let config = ApplicationConfig::from_sources("./application.yml")
        .expect("Failed to load config");
    
    println!("Starting {} on {}", config.server_name, config.server_url);
    println!("Debug mode: {}", config.debug);
    println!("Cache type: {}", config.cache_type);
    
    // 使用配置...
}
```

## Key Files

- [crates/config/src/lib.rs](file:///d:/tdcare/genies/crates/config/src/lib.rs) - 模块入口
- [crates/config/src/app_config.rs](file:///d:/tdcare/genies/crates/config/src/app_config.rs) - ApplicationConfig 定义
- [crates/config/src/log_config.rs](file:///d:/tdcare/genies/crates/config/src/log_config.rs) - LogConfig 和 init_log
- [crates/config/examples/](file:///d:/tdcare/genies/crates/config/examples/) - 配置示例
- [crates/genies_derive/src/config_core_type.rs](file:///d:/tdcare/genies/crates/genies_derive/src/config_core_type.rs) - ConfigCore 宏实现
