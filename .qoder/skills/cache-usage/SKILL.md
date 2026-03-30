---
name: cache-usage
description: Guide for using genies_cache dual-backend caching service. Use when implementing Redis or in-memory caching, managing TTL and atomic operations, or integrating caching into Genies microservices.
---

# Cache Module (genies_cache)

## Overview

genies_cache 是 Genies 框架的双后端缓存库，提供 Redis 和内存两种可互换的后端。纯库 crate，无 binary。

**核心特性：**
- 双后端：Redis（生产）/ Memory（测试）
- 统一接口：`ICacheService` trait
- 两种工厂：`new()`（业务缓存）/ `new_saved()`（持久化缓存）
- 原子操作：`set_string_ex_nx` 用于分布式锁和幂等性
- TTL 管理：过期时间设置和查询

## Quick Reference

### 创建缓存

```rust
use genies_cache::cache_service::CacheService;
use genies_config::app_config::ApplicationConfig;

let config = ApplicationConfig::from_sources("./application.yml")?;

// 业务缓存（使用 redis_url）
let cache = CacheService::new(&config);

// 持久化缓存（使用 redis_save_url）
let saved_cache = CacheService::new_saved(&config);
```

### 基本操作

```rust
use std::time::Duration;

// 设置/获取/删除
cache.set_string("key", "value").await?;
let value = cache.get_string("key").await?;
cache.del_string("key").await?;

// 带 TTL
cache.set_string_ex("key", "value", Some(Duration::from_secs(300))).await?;

// 查询剩余 TTL
let ttl = cache.ttl("key").await?;
// > 0: 剩余秒数, -1: 无过期, -2: 不存在
```

### JSON 操作

```rust
use serde::{Serialize, Deserialize};

#[derive(Serialize, Deserialize)]
struct User {
    id: u64,
    name: String,
}

// 存储 JSON
cache.set_json("user:1", &User { id: 1, name: "张三".into() }).await?;

// 获取 JSON
let user: User = cache.get_json("user:1").await?;
```

### 原子操作 (NX)

```rust
// 仅当 key 不存在时设置，返回是否设置成功
let success = cache.set_string_ex_nx(
    "lock:resource",
    "holder_id",
    Some(Duration::from_secs(60))
).await?;

if success {
    // 获得锁
} else {
    // 锁已被占用
}
```

## ICacheService Trait

```rust
#[async_trait]
pub trait ICacheService: Sync + Send {
    // 字符串操作
    async fn set_string(&self, k: &str, v: &str) -> Result<String>;
    async fn get_string(&self, k: &str) -> Result<String>;
    async fn del_string(&self, k: &str) -> Result<String>;
    
    // 带 TTL
    async fn set_string_ex(&self, k: &str, v: &str, ex: Option<Duration>) -> Result<String>;
    
    // 原子 NX（Set-if-Not-eXists）
    async fn set_string_ex_nx(&self, k: &str, v: &str, ex: Option<Duration>) -> Result<bool>;
    
    // 二进制操作
    async fn set_value(&self, k: &str, v: &[u8]) -> Result<String>;
    async fn get_value(&self, k: &str) -> Result<Vec<u8>>;
    async fn set_value_ex(&self, k: &str, v: &[u8], ex: Option<Duration>) -> Result<String>;
    
    // TTL 查询
    async fn ttl(&self, k: &str) -> Result<i64>;
}
```

## 使用模式

### 分布式锁

```rust
async fn with_lock<F, T>(cache: &CacheService, key: &str, f: F) -> Result<T>
where
    F: Future<Output = Result<T>>,
{
    let lock_key = format!("lock:{}", key);
    let ttl = Some(Duration::from_secs(60));
    
    // 获取锁
    let acquired = cache.set_string_ex_nx(&lock_key, "locked", ttl).await?;
    if !acquired {
        return Err(Error::from("Failed to acquire lock"));
    }
    
    // 执行业务
    let result = f.await;
    
    // 释放锁
    cache.del_string(&lock_key).await?;
    
    result
}
```

### 消息幂等性

```rust
async fn process_message_idempotent(cache: &CacheService, msg_id: &str, payload: &str) -> Result<()> {
    let key = format!("msg:{}", msg_id);
    let ttl = Some(Duration::from_secs(300));
    
    // Step 1: 尝试抢锁
    let lock = cache.set_string_ex_nx(&key, "CONSUMING", ttl).await?;
    
    if !lock {
        // 检查是否已完成
        let status = cache.get_string(&key).await?;
        if status == "CONSUMED" {
            log::info!("消息 {} 已处理，跳过", msg_id);
            return Ok(());
        }
        // 正在被其他实例处理
        return Err(Error::from("消息正在处理中"));
    }
    
    // Step 2: 处理消息
    match do_business(payload).await {
        Ok(_) => {
            // Step 3: 标记完成
            cache.set_string_ex(&key, "CONSUMED", ttl).await?;
            Ok(())
        }
        Err(e) => {
            // 处理失败，删除锁允许重试
            cache.del_string(&key).await?;
            Err(e)
        }
    }
}
```

### 缓存穿透保护

```rust
async fn get_user_cached(cache: &CacheService, db: &Database, user_id: &str) -> Result<Option<User>> {
    let key = format!("user:{}", user_id);
    
    // 尝试从缓存获取
    let cached = cache.get_string(&key).await?;
    if !cached.is_empty() {
        if cached == "NULL" {
            return Ok(None); // 缓存的空值
        }
        return Ok(serde_json::from_str(&cached)?);
    }
    
    // 缓存未命中，查询数据库
    match db.find_user(user_id).await? {
        Some(user) => {
            // 缓存用户数据
            cache.set_string_ex(&key, &serde_json::to_string(&user)?, 
                Some(Duration::from_secs(3600))).await?;
            Ok(Some(user))
        }
        None => {
            // 缓存空值，防止缓存穿透
            cache.set_string_ex(&key, "NULL", Some(Duration::from_secs(60))).await?;
            Ok(None)
        }
    }
}
```

### 会话管理

```rust
async fn create_session(cache: &CacheService, user_id: &str) -> Result<String> {
    let session_id = generate_uuid();
    let key = format!("session:{}", session_id);
    let session_data = serde_json::to_string(&SessionData { user_id: user_id.into() })?;
    
    // 24 小时过期
    cache.set_string_ex(&key, &session_data, Some(Duration::from_secs(86400))).await?;
    
    Ok(session_id)
}

async fn get_session(cache: &CacheService, session_id: &str) -> Result<Option<SessionData>> {
    let key = format!("session:{}", session_id);
    let data = cache.get_string(&key).await?;
    
    if data.is_empty() {
        return Ok(None);
    }
    
    Ok(Some(serde_json::from_str(&data)?))
}

async fn refresh_session(cache: &CacheService, session_id: &str) -> Result<()> {
    let key = format!("session:{}", session_id);
    let data = cache.get_string(&key).await?;
    
    if !data.is_empty() {
        // 刷新 TTL
        cache.set_string_ex(&key, &data, Some(Duration::from_secs(86400))).await?;
    }
    
    Ok(())
}
```

## 配置

```yaml
# application.yml

# 后端选择
cache_type: "redis"  # 或 "mem"

# Redis 连接
redis_url: "redis://:password@127.0.0.1:6379"
redis_save_url: "redis://:password@127.0.0.1:6379/1"
```

### 后端选择策略

| 场景 | cache_type | 说明 |
|------|------------|------|
| 生产环境 | `redis` | 分布式、持久化 |
| 单元测试 | `mem` | 无外部依赖 |
| 集成测试 | `mem` | 避免 Redis 依赖 |
| 本地开发 | 均可 | 根据需要选择 |

## 两个缓存实例

```rust
// new() - 业务缓存
// 使用 redis_url，用于临时数据
let cache = CacheService::new(&config);

// new_saved() - 持久化缓存  
// 使用 redis_save_url，用于重要数据
let saved_cache = CacheService::new_saved(&config);
```

典型用途：
- `cache`：会话、临时锁、缓存数据
- `saved_cache`：配置、持久化状态、重要记录

## 与其他 Crate 集成

| Crate | 集成方式 |
|-------|----------|
| genies_config | 提供 `cache_type`, `redis_url`, `redis_save_url` |
| genies_context | `CONTEXT.cache()` 返回业务缓存 |
| genies_auth | 版本同步使用缓存 |
| genies_ddd | 消息幂等性使用缓存 |

## 测试技巧

```rust
#[cfg(test)]
mod tests {
    use genies_cache::cache_service::CacheService;
    
    fn make_test_cache() -> CacheService {
        let config = ApplicationConfig {
            cache_type: "mem".to_string(),
            redis_url: String::new(),
            redis_save_url: String::new(),
            // ... other fields
        };
        CacheService::new(&config)
    }
    
    #[tokio::test]
    async fn test_set_get() {
        let cache = make_test_cache();
        cache.set_string("key", "value").await.unwrap();
        assert_eq!(cache.get_string("key").await.unwrap(), "value");
    }
}
```

## Key Files

- [crates/cache/src/lib.rs](file:///d:/tdcare/genies/crates/cache/src/lib.rs) - 模块入口
- [crates/cache/src/cache_service.rs](file:///d:/tdcare/genies/crates/cache/src/cache_service.rs) - ICacheService trait 和 CacheService
- [crates/cache/src/redis_service.rs](file:///d:/tdcare/genies/crates/cache/src/redis_service.rs) - Redis 实现
- [crates/cache/src/mem_service.rs](file:///d:/tdcare/genies/crates/cache/src/mem_service.rs) - 内存实现
