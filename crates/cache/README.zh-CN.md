# Cache 模块

`cache` 模块负责缓存相关的功能，包括内存缓存和 Redis 缓存服务。该模块提供了简单易用的 API，帮助开发者快速集成缓存功能到应用程序中。

## 功能概述
- **内存缓存**: 提供基于内存的缓存服务，适合快速访问但数据量较小的场景。
- **Redis 缓存**: 提供基于 Redis 的缓存服务，适合分布式系统或需要持久化缓存的场景。

## 使用说明

### 内存缓存
```rust
use cache::mem_service::MemService;

let mem_service = MemService::new();
mem_service.set("key", "value").unwrap();
let value = mem_service.get("key").unwrap();
println!("{}", value);
```

### Redis 缓存
```rust
use cache::redis_service::RedisService;

let redis_service = RedisService::new("redis://127.0.0.1/");
redis_service.set("key", "value").unwrap();
let value = redis_service.get("key").unwrap();
println!("{}", value);
```

### 缓存配置
你可以在 `application.yml` 中配置缓存服务的参数，例如 Redis 的连接字符串和内存缓存的最大容量。

```yaml
cache:
  redis:
    url: "redis://127.0.0.1/"
  mem:
    max_capacity: 1000
```

## 贡献
欢迎提交 Pull Request 或 Issue 来改进本项目。

## 许可证
本项目采用 MIT 许可证，详情请参阅 `LICENSE` 文件。 