use genies_derive::ConfigCore;
use serde::{Deserialize, Serialize};


/// 基础配置示例
#[derive(ConfigCore, Debug, Deserialize, Serialize)]
pub struct BasicConfig {
    // 基本类型，带默认值
    #[config(default = "localhost")]
    pub host: String,
    
    #[config(default = 8080)]
    #[config(validate(range(min = 1, max = 65535)))]
    pub port: u16,
    
    // 基本类型，无默认值
    pub app_name: String,
}

/// 数组配置示例
#[derive(ConfigCore, Debug, Deserialize, Serialize)]
pub struct ArrayConfig {
    // 字符串数组，带默认值
    #[config(default = "topic1,topic2,topic3")]
    pub topics: Vec<String>,
    
    // 数字数组，带默认值
    #[config(default = "1,2,3")]
    pub numbers: Vec<i32>,
    
    // 空数组默认值
    #[config(default = "")]
    pub empty_array: Vec<String>,
}

/// 可选值配置示例
#[derive(ConfigCore, Debug, Deserialize, Serialize)]
pub struct OptionalConfig {
    // 可选字符串，带默认值
    #[config(default = "guest")]
    pub username: Option<String>,
    
    // 可选字符串，无默认值
    pub password: Option<String>,
    
    // 可选数字，带默认值和验证
    #[config(default = "3600")]
    #[config(validate(range(min = 60, max = 86400)))]
    pub timeout_seconds: Option<u64>,
}

/// 复杂配置示例
#[derive(ConfigCore, Debug, Deserialize, Serialize)]
pub struct ComplexConfig {
    // 必填基本配置
    #[config(default = "my-app")]
    pub app_name: String,
    
    #[config(default = "development")]  
    pub environment: String,
    
    // 可选数组
    pub allowed_hosts: Option<Vec<String>>,
    
    // 带验证的端口范围
    #[config(default = "8080,8081,8082")]
    #[config(validate(range(min = 1, max = 65535)))]
    pub ports: Vec<u16>,
    
    // 可选超时配置
    #[config(default = "30")]
    pub connection_timeout: Option<u64>,
    
    #[config(default = "300")]
    pub read_timeout: Option<u64>,
}

/// 环境变量数组功能示例
#[derive(ConfigCore, Debug, Deserialize, Serialize)]
pub struct EnvArrayConfig {
    #[config(default = "default1,default2")]
    pub topics: Vec<String>,

    #[config(default = "1,2,3")]
    pub numbers: Vec<i32>,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("cwd: {:?}", std::env::current_dir()?);

    // 1. 基础配置示例
    let basic_config_path = "crates/config/examples/config/basic.yml";
    println!("Attempting to read basic config from path: {}", basic_config_path);
    let basic_config = BasicConfig::from_sources(basic_config_path)?;
    println!("Basic Config: {:?}", basic_config);

    // 2. 数组配置示例
    let array_config_path = "crates/config/examples/config/array.yml";
    println!("Attempting to read array config from path: {}", array_config_path);
    let array_config = ArrayConfig::from_sources(array_config_path)?;
    println!("Array Config: {:?}", array_config);

    // 3. 可选值配置示例
    let optional_config_path = "crates/config/examples/config/optional.yml";
    println!("Attempting to read optional config from path: {}", optional_config_path);
    let optional_config = OptionalConfig::from_sources(optional_config_path)?;
    println!("Optional Config: {:?}", optional_config);

    // 4. 复杂配置示例
    let complex_config_path = "crates/config/examples/config/complex.yml";
    println!("Attempting to read complex config from path: {}", complex_config_path);
    let complex_config = ComplexConfig::from_sources(complex_config_path)?;
    println!("Complex Config: {:?}", complex_config);

    // 环境变量数组功能示例
    let env_array_config_path = "crates/config/examples/config/array.yml";
    println!("Attempting to read env array config from path: {}", env_array_config_path);
    let env_array_config = EnvArrayConfig::from_sources(env_array_config_path)?;
    println!("EnvArrayConfig (from env): {:?}", env_array_config);

    Ok(())
}

/*
配置文件示例:

1. basic.yml:
```yaml
host: "example.com"
port: 9090
app_name: "my-service"
```

2. array.yml:
```yaml
topics:
  - "news"
  - "updates"
  - "alerts"
numbers: [10, 20, 30]
empty_array: []
```

3. optional.yml:
```yaml
username: "admin"
password: "secret"
timeout_seconds: 7200
```

4. complex.yml:
```yaml
app_name: "production-app"
environment: "production"
allowed_hosts:
  - "api.example.com"
  - "admin.example.com"
ports: [8080, 8081, 8082]
connection_timeout: 60
read_timeout: 600
```

环境变量示例:

```bash
# 基础配置
export HOST="production.example.com"
export PORT="443"
export APP_NAME="prod-service"

# 数组配置
export TOPICS="prod/events,prod/logs,prod/metrics"
export NUMBERS="100,200,300"
export EMPTY_ARRAY=""

# 可选值配置
export USERNAME="prod-admin"
export PASSWORD="prod-secret"
export TIMEOUT_SECONDS="3600"

# 复杂配置
export APP_NAME="prod-app"
export ENVIRONMENT="production"
export ALLOWED_HOSTS="api.prod.com,admin.prod.com"
export PORTS="80,443,8443"
export CONNECTION_TIMEOUT="120"
export READ_TIMEOUT="900"
```
*/