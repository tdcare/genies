# Genies Derive

用于Rust配置管理的强大派生宏，提供了一种灵活且类型安全的应用程序配置处理方式。

## 功能特性

- 🔧 通过属性设置默认值
- 🌍 环境变量支持
- ✅ 配置验证
- 📁 YAML文件配置
- 🔄 热重载支持
- 🏗️ 构建器模式
- 🔄 类型转换
- 📦 数组和Option类型支持

## 安装

将以下内容添加到你的`Cargo.toml`中：

```toml
[dependencies]
genies_derive = "0.1.0"
```

## 快速开始

```rust
use genies_derive::Config;
use serde::{Deserialize, Serialize};

#[derive(Config, Debug, Deserialize, Serialize)]
struct ServerConfig {
    #[config(default = "localhost")]
    host: String,
    
    #[config(default = 8080)]
    #[config(validate(range(min = 1, max = 65535)))]
    port: u16,
    
    #[config(default = "topic1,topic2,topic3")]
    topics: Vec<String>,
    
    // 可选字段
    username: Option<String>,
    
    #[config(default = "60")]
    timeout_seconds: Option<u64>,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 从文件和环境变量加载
    let config = ServerConfig::from_sources("config.yml")?;
    println!("Config: {:?}", config);

    Ok(())
}
```

## 配置来源

配置按以下优先级顺序加载（最高优先级在前）：

1. 环境变量
2. 配置文件
3. 默认值
4. None（对于可选字段）

### 环境变量

字段名会自动转换为大写下划线格式（SCREAMING_SNAKE_CASE）：

```bash
export HOST="production.example.com"
export PORT="443"
export TOPICS="prod/events,prod/logs,prod/metrics"
export USERNAME="admin"
export TIMEOUT_SECONDS="120"
```

### YAML配置

```yaml
host: "example.com"
port: 8080
topics:
  - "topic1"
  - "topic2"
username: "user"
timeout_seconds: 60
```

## 功能特性

### 默认值

```rust
#[config(default = "localhost")]
host: String
```

### 验证

```rust
#[config(validate(range(min = 1, max = 65535)))]
port: u16
```

### 数组

```rust
#[config(default = "topic1,topic2,topic3")]
topics: Vec<String>
```

### 可选字段

```rust
username: Option<String>
```

### 热重载

```rust
let mut config = ServerConfig::from_sources("config.yml")?;
config.reload().await?;
```

## 示例

查看[examples](examples/)目录获取更详细的示例。

## 贡献

我们欢迎贡献！请随时提交Pull Request。

## 许可证

本项目采用MIT许可证 - 查看[LICENSE](LICENSE)文件了解详情。
