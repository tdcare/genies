# genies_cache

Genies (神灯) 框架的双后端缓存服务，支持 Redis 和内存存储。

## 概述

genies_cache 提供统一的缓存接口，支持两种可互换的后端：

- **Redis 后端**：生产就绪的分布式缓存
- **内存后端**：用于测试和开发的本地内存缓存
- **统一 API**：通过 `ICacheService` trait 为两种后端提供相同接口
- **TTL 支持**：自动过期的生存时间
- **原子操作**：`set_string_ex_nx` 用于分布式锁和幂等性

## 核心特性

- **后端切换**：通过 `cache_type` 配置在 Redis 和内存之间切换
- **两种工厂方法**：`new()` 用于业务缓存，`new_saved()` 用于持久化缓存
- **异步 API**：所有操作都与 Tokio 异步兼容
- **JSON 序列化**：内置 `set_json` / `get_json` 辅助方法
- **原子 NX 操作**：用于幂等性模式的条件设置
- **TTL 管理**：设置过期时间和查询剩余时间

## 核心组件

| 组件 | 文件 | 说明 |
|------|------|------|
| `ICacheService` | cache_service.rs | 定义缓存操作的 trait |
| `CacheService` | cache_service.rs | 创建适当后端的工厂 |
| `RedisService` | redis_service.rs | Redis 实现 |
| `MemService` | mem_service.rs | 内存实现 |

## API 参考

### ICacheService Trait

```rust
#[async_trait]
pub trait ICacheService: Sync + Send {
    async fn set_string(&self, k: &str, v: &str) -> Result<String>;
    async fn get_string(&self, k: &str) -> Result<String>;
    async fn del_string(&self, k: &str) -> Result<String>;
    async fn set_string_ex(&self, k: &str, v: &str, ex: Option<Duration>) -> Result<String>;
    async fn set_string_ex_nx(&self, k: &str, v: &str, ex: Option<Duration>) -> Result<bool>;
    async fn set_value(&self, k: &str, v: &[u8]) -> Result<String>;
    async fn get_value(&self, k: &str) -> Result<Vec<u8>>;
    async fn set_value_ex(&self, k: &str, v: &[u8], ex: Option<Duration>) -> Result<String>;
    async fn ttl(&self, k: &str) -> Result<i64>;
}
```

### CacheService 方法

| 方法 | 说明 |
|------|------|
| `new(cfg)` | 创建业务缓存（使用 `redis_url`） |
| `new_saved(cfg)` | 创建持久化缓存（使用 `redis_save_url`） |
| `set_string(k, v)` | 设置字符串值（无过期） |
| `get_string(k)` | 获取字符串值 |
| `del_string(k)` | 删除键 |
| `set_string_ex(k, v, ttl)` | 设置带可选 TTL 的值 |
| `set_string_ex_nx(k, v, ttl)` | 原子条件设置（不存在时才设置） |
| `set_json(k, v)` | 序列化并存储 JSON |
| `get_json(k)` | 获取并反序列化 JSON |
| `ttl(k)` | 获取剩余 TTL（秒） |

## 快速开始

### 1. 添加依赖

```sh
cargo add genies_cache genies_config
```

> 也可以手动在 `Cargo.toml` 中添加依赖，请前往 [crates.io](https://crates.io) 查看最新版本。

### 2. 配置后端

在 `application.yml` 中：

```yaml
# 使用 Redis 后端
cache_type: "redis"
redis_url: "redis://:password@127.0.0.1:6379"
redis_save_url: "redis://:password@127.0.0.1:6379"

# 或使用内存后端（用于测试）
cache_type: "mem"
```

### 3. 创建和使用缓存

```rust
use genies_cache::cache_service::CacheService;
use genies_config::app_config::ApplicationConfig;
use std::time::Duration;

// 加载配置
let config = ApplicationConfig::from_sources("./application.yml")?;

// 创建缓存服务
let cache = CacheService::new(&config);

// 基本操作
cache.set_string("key", "value").await?;
let value = cache.get_string("key").await?;
cache.del_string("key").await?;

// 带 TTL（5 分钟后过期）
cache.set_string_ex("session", "data", Some(Duration::from_secs(300))).await?;

// 检查剩余 TTL
let remaining = cache.ttl("session").await?;
```

## 使用模式

### JSON 存储

```rust
use serde::{Serialize, Deserialize};

#[derive(Serialize, Deserialize)]
struct User {
    id: u64,
    name: String,
}

// 存储 JSON 对象
let user = User { id: 1, name: "张三".into() };
cache.set_json("user:1", &user).await?;

// 获取 JSON 对象
let user: User = cache.get_json("user:1").await?;
```

### 分布式锁 / 幂等性

```rust
use std::time::Duration;

// 获取锁（原子条件设置）
let lock_acquired = cache.set_string_ex_nx(
    "lock:order:123",
    "processing",
    Some(Duration::from_secs(60))
).await?;

if lock_acquired {
    // 获得锁，处理订单
    process_order().await?;
    
    // 标记完成
    cache.set_string_ex(
        "lock:order:123",
        "completed",
        Some(Duration::from_secs(86400))
    ).await?;
} else {
    // 其他实例正在处理或已完成
    println!("订单已在处理中");
}
```

### 消息幂等性模式

```rust
async fn handle_message(msg_id: &str, payload: &str) -> Result<()> {
    let key = format!("msg:{}", msg_id);
    let ttl = Some(Duration::from_secs(300)); // 5 分钟
    
    // 尝试获取处理锁
    let lock = cache.set_string_ex_nx(&key, "CONSUMING", ttl).await?;
    
    if !lock {
        // 检查是否已处理完成
        let status = cache.get_string(&key).await?;
        if status == "CONSUMED" {
            return Ok(()); // 已处理，跳过
        }
        // 其他实例正在处理
        return Err(Error::from("消息正在处理中"));
    }
    
    // 处理消息
    match process_payload(payload).await {
        Ok(_) => {
            // 标记完成
            cache.set_string_ex(&key, "CONSUMED", ttl).await?;
            Ok(())
        }
        Err(e) => {
            // 失败，移除锁以允许重试
            cache.del_string(&key).await?;
            Err(e)
        }
    }
}
```

### 两个缓存实例

```rust
// 业务缓存（易失，使用 redis_url）
let cache = CacheService::new(&config);

// 持久化缓存（持久，使用 redis_save_url）
let saved_cache = CacheService::new_saved(&config);

// 使用业务缓存存储会话
cache.set_string_ex("session:abc", "data", Some(Duration::from_secs(3600))).await?;

// 使用持久化缓存存储重要数据
saved_cache.set_string("config:feature_flags", flags_json).await?;
```

## 后端对比

| 特性 | Redis | 内存 |
|------|-------|------|
| 分布式 | 是 | 否 |
| 持久化 | 可选 | 否 |
| TTL 支持 | 原生 | 模拟 |
| 原子 NX | 原生 | 基于 Mutex |
| 适用场景 | 生产 | 测试 |

## TTL 返回值

`ttl(k)` 方法返回：

| 返回值 | 含义 |
|--------|------|
| `> 0` | 距离过期的剩余秒数 |
| `-1` | 键存在但没有 TTL |
| `-2` | 键不存在 |

## 配置说明

```yaml
# 后端选择
cache_type: "redis"  # 或 "mem"

# Redis URLs
redis_url: "redis://:password@host:port"           # 业务缓存
redis_save_url: "redis://:password@host:port/db1"  # 持久化缓存
```

Redis URL 格式：`redis://[:password@]host:port[/db]`

## 测试

对于集成测试，使用内存后端以避免 Redis 依赖：

```yaml
# test-application.yml
cache_type: "mem"
```

或模拟 trait：

```rust
#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_cache() {
        let config = ApplicationConfig {
            cache_type: "mem".to_string(),
            // ... 其他字段
        };
        let cache = CacheService::new(&config);
        
        cache.set_string("test", "value").await.unwrap();
        assert_eq!(cache.get_string("test").await.unwrap(), "value");
    }
}
```

## 依赖项

- **redis** - Redis 客户端
- **tokio** - 异步运行时
- **async-trait** - 异步 trait 支持
- **serde** / **serde_json** - JSON 序列化
- **genies_core** - 错误类型
- **genies_config** - ApplicationConfig

## 与其他 Crate 集成

- **genies_config**：提供 `cache_type`、`redis_url`、`redis_save_url`
- **genies_context**：`CONTEXT.cache()` 返回业务缓存
- **genies_auth**：使用缓存进行 enforcer 版本同步
- **genies_ddd**：使用缓存实现消息幂等性

## 许可证

请参阅项目根目录的许可证信息。
