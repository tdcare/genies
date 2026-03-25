# Genies 框架详细参考

## ApplicationConfig 完整配置项表

| 配置项 | 类型 | 默认值 | 说明 |
|--------|------|--------|------|
| `debug` | bool | false | 调试模式开关 |
| `server_name` | String | - | 微服务名称 |
| `servlet_path` | String | - | 当前服务路由前缀 |
| `server_url` | String | - | 服务监听地址，如 "0.0.0.0:8080" |
| `gateway` | Option\<String\> | None | 网关地址，http/https 协议使用网关，否则使用 Dapr |
| `redis_url` | String | - | Redis 缓存地址 |
| `redis_save_url` | String | - | 持久化 Redis 地址（用于事件幂等性检查） |
| `database_url` | String | - | MySQL 数据库连接串 |
| `max_connections` | u32 | - | 数据库最大连接数 |
| `min_connections` | u32 | - | 数据库最小连接数 |
| `wait_timeout` | u64 | - | 连接等待超时（秒） |
| `create_timeout` | u64 | - | 创建连接超时（秒） |
| `max_lifetime` | u64 | - | 连接最大生命周期（秒） |
| `log_level` | String | - | 日志级别，如 "debug,sqlx=warn" |
| `white_list_api` | Vec\<String\> | [] | 免认证白名单接口列表 |
| `cache_type` | String | - | 缓存类型："redis" 或 "mem" |
| `keycloak_auth_server_url` | String | - | Keycloak 认证服务地址 |
| `keycloak_realm` | String | - | Keycloak Realm 名称 |
| `keycloak_resource` | String | - | Keycloak Client ID |
| `keycloak_credentials_secret` | String | - | Keycloak Client Secret |
| `dapr_pubsub_name` | String | - | Dapr PubSub 组件名 |
| `dapr_pub_message_limit` | i64 | - | 每次发布消息数量限制 |
| `dapr_cdc_message_period` | i64 | - | CDC 消息轮询周期（毫秒） |
| `processing_expire_seconds` | i64 | - | 消息处理超时时间（秒） |
| `record_reserve_minutes` | i64 | - | 消息记录保留时间（分钟） |

## application.yml 完整示例

```yaml
# 基础配置
debug: true
server_name: "order-service"
servlet_path: "/order"
server_url: "0.0.0.0:8080"

# 网关配置
# HTTP 协议使用网关访问其他微服务，否则使用 Dapr sidecar
gateway: "http://api-gateway:8080"

# 缓存配置
cache_type: "redis"
redis_url: "redis://:password@redis:6379"
redis_save_url: "redis://:password@redis-persistent:6379"

# 数据库配置
database_url: "mysql://root:password@mysql:3306/order_db?serverTimezone=Asia/Shanghai"
max_connections: 20
min_connections: 2
wait_timeout: 60
create_timeout: 120
max_lifetime: 1800

# 日志配置
# 格式：level 或 "level,module=level,module2=level"
log_level: "debug,sqlx=warn,hyper=info"

# Keycloak 认证配置
keycloak_auth_server_url: "http://keycloak:8080/auth/"
keycloak_realm: "myrealm"
keycloak_resource: "order-service"
keycloak_credentials_secret: "your-client-secret-here"

# Dapr 配置
dapr_pubsub_name: "messagebus"
dapr_pub_message_limit: 50
dapr_cdc_message_period: 5000

# 事件消费配置
processing_expire_seconds: 60    # 消息处理中状态的过期时间
record_reserve_minutes: 10080    # 已消费消息记录保留时间（7天）

# 白名单接口（免登录）
white_list_api:
  - "/"
  - "/actuator/*"
  - "/dapr/*"
  - "/daprsub/*"
  - "/swagger-ui/*"
  - "/api-doc/*"
```

## 环境变量覆盖规则

Genies 支持通过环境变量覆盖配置文件中的值，优先级：**默认值 < YAML 文件 < 环境变量**

### 支持的格式

1. **原字段名格式**（小写下划线）
   ```bash
   export database_url="mysql://prod:password@prod-db:3306/db"
   export redis_url="redis://:pwd@prod-redis:6379"
   ```

2. **大写下划线格式**（SCREAMING_SNAKE_CASE）
   ```bash
   export DATABASE_URL="mysql://prod:password@prod-db:3306/db"
   export REDIS_URL="redis://:pwd@prod-redis:6379"
   export LOG_LEVEL="info"
   ```

### 特殊类型处理

- **Vec 类型**：使用逗号分隔
  ```bash
  export WHITE_LIST_API="/health/*,/actuator/*,/public/*"
  ```

- **Option 类型**：空字符串表示 None
  ```bash
  export GATEWAY=""  # 设置为 None
  ```

## Casbin 权限模型详解

### model.conf 配置

```ini
[request_definition]
r = sub, obj, act
# sub: 主体（用户名/角色）
# obj: 对象（资源路径，如 "StructName.field_name"）
# act: 动作（如 "read", "write"）

[policy_definition]
p = sub, obj, act, eft
# eft: 效果，"allow" 或 "deny"

[role_definition]
g = _, _      # 用户-角色映射：g, user, role
g2 = _, _     # 资源-资源组映射：g2, resource, group

[policy_effect]
e = !some(where (p.eft == deny))
# 默认允许，只要有一条 deny 策略匹配则拒绝

[matchers]
m = g(r.sub, p.sub) && g2(r.obj, p.obj) && r.act == p.act
# g(r.sub, p.sub): 请求主体是否匹配策略主体（支持角色继承）
# g2(r.obj, p.obj): 请求对象是否匹配策略对象（支持资源组）
```

### policy.csv 格式详解

```csv
# 策略格式：p, 主体, 对象, 动作, 效果

# 直接授权：用户 alice 不能读取 UserProfile.email 字段
p, alice, genies_auth.vo.UserProfile.email, read, deny

# 角色授权：data_group_admin 角色不能读取 data_group 组的资源
p, data_group_admin, data_group, read, deny

# 用户-角色映射：alice 属于 data_group_admin 角色
g, alice, data_group_admin

# 资源-资源组映射：这些字段属于 data_group 组
g2, genies_auth.vo.UserProfile.credit_card, data_group
g2, genies_auth.vo.UserProfile.name, data_group
g2, genies_auth.vo.User.phone, data_group
```

### 权限检查工作流程

1. `#[casbin]` 宏修改结构体，添加 `enforcer` 和 `subject` 字段
2. 自定义 `Serialize` 实现在序列化每个字段前调用 `check_permission`
3. `check_permission` 构造请求 `(subject, "StructName.field_name", "read")`
4. Casbin Enforcer 根据策略决定是否允许
5. 被拒绝的字段不会出现在 JSON 输出中

### 完整使用示例

```rust
use genies_derive::casbin;
use serde::Deserialize;
use salvo::oapi::ToSchema;
use std::sync::Arc;
use casbin::Enforcer;

#[casbin]
#[derive(Deserialize, ToSchema)]
pub struct UserProfile {
    pub id: u64,
    pub name: String,
    pub email: String,
    pub phone: String,
    pub credit_card: String,
    // 宏自动添加的字段（在源码中不需要手动写）：
    // pub enforcer: Option<Arc<Enforcer>>,
    // pub subject: Option<String>,
}

// Handler 中使用
#[endpoint]
async fn get_user_profile(
    req: &mut Request,
    depot: &mut Depot
) -> Json<UserProfile> {
    // 从 depot 获取 Enforcer（需要在中间件中注入）
    let enforcer = depot.obtain::<Arc<Enforcer>>().unwrap();
    
    // 获取当前用户
    let current_user = req.query::<String>("user").unwrap_or("guest".into());
    
    // 构建响应对象
    let profile = UserProfile {
        id: 1,
        name: "张三".to_string(),
        email: "zhangsan@example.com".to_string(),
        phone: "13800138000".to_string(),
        credit_card: "1234-5678-9012-3456".to_string(),
        enforcer: None,
        subject: None,
    };
    
    // 应用权限策略后返回
    // 被拒绝的字段将不会出现在 JSON 响应中
    Json(profile.with_policy(Arc::clone(&enforcer), current_user))
}
```

## 核心 Trait API 签名

### AggregateType (genies::ddd::aggregate)

```rust
/// 聚合类型标识 Trait
pub trait AggregateType {
    /// 获取聚合类型名称（实例方法）
    fn aggregate_type(&self) -> String;
    
    /// 获取聚合类型名称（静态方法）
    fn atype() -> String;
}
```

### WithAggregateId (genies::ddd::aggregate)

```rust
/// 聚合 ID 标识 Trait
pub trait WithAggregateId {
    /// ID 类型，必须实现 Debug, Clone, PartialEq, Serialize, DeserializeOwned
    type Id: Debug + Clone + PartialEq + Serialize + DeserializeOwned;
    
    /// 获取聚合 ID 的引用
    fn aggregate_id(&self) -> &Self::Id;
}

/// 便捷类型别名
pub type AggregateIdOf<A> = <A as WithAggregateId>::Id;
```

### InitializeAggregate (genies::ddd::aggregate)

```rust
/// 聚合初始化 Trait
pub trait InitializeAggregate {
    /// 聚合状态类型
    type State: WithAggregateId;
    
    /// 使用聚合 ID 初始化聚合状态
    fn initialize(aggregate_id: AggregateIdOf<Self::State>) -> Self::State;
}
```

### DomainEvent (genies::ddd::event)

```rust
/// 领域事件 Trait
pub trait DomainEvent: Send {
    /// 获取事件版本（如 "V1", "V2"）
    fn event_type_version(&self) -> String;
    
    /// 获取事件类型标识（用于反序列化路由）
    fn event_type(&self) -> String;
    
    /// 获取事件来源（通常是聚合根全限定名）
    fn event_source(&self) -> String;
    
    /// 序列化为 JSON 字符串
    fn json(&self) -> String;
}
```

### ICacheService (genies::cache::cache_service)

```rust
#[async_trait]
pub trait ICacheService: Sync + Send {
    /// 设置字符串值
    async fn set_string(&self, k: &str, v: &str) -> Result<String>;
    
    /// 获取字符串值
    async fn get_string(&self, k: &str) -> Result<String>;
    
    /// 删除键
    async fn del_string(&self, k: &str) -> Result<String>;
    
    /// 设置字符串值（带过期时间）
    async fn set_string_ex(&self, k: &str, v: &str, ex: Option<Duration>) -> Result<String>;
    
    /// 设置二进制值
    async fn set_value(&self, k: &str, v: &[u8]) -> Result<String>;
    
    /// 获取二进制值
    async fn get_value(&self, k: &str) -> Result<Vec<u8>>;
    
    /// 设置二进制值（带过期时间）
    async fn set_value_ex(&self, k: &str, v: &[u8], ex: Option<Duration>) -> Result<String>;
    
    /// 获取键的剩余生存时间（秒），-1 表示永不过期，-2 表示键不存在
    async fn ttl(&self, k: &str) -> Result<i64>;
}
```

## RespVO / ResultDTO / Error 类型说明

### RespVO\<T\> (genies::core)

标准 HTTP 响应模型，用于返回 JSON 响应：

```rust
#[derive(Debug, Serialize, Deserialize, Clone, Default)]
pub struct RespVO<T> {
    /// 响应码："SUCCESS" 或 "FAIL"
    pub code: Option<String>,
    /// 错误消息
    pub msg: Option<String>,
    /// 响应数据
    pub data: Option<T>,
}

impl<T> RespVO<T> {
    /// 从 Result 创建响应
    pub fn from_result(arg: &Result<T>) -> Self;
    
    /// 从数据创建成功响应
    pub fn from(arg: &T) -> Self;
    
    /// 创建错误响应
    pub fn from_error(code: &str, arg: &Error) -> Self;
    
    /// 从错误信息创建响应
    pub fn from_error_info(code: &str, info: &str) -> Self;
    
    /// 创建 Dapr pubsub 成功响应
    pub fn is_success(&mut self) -> RespVO<serde_json::Value>;
    
    /// 创建 Dapr pubsub 重试响应
    pub fn is_retry(&mut self) -> RespVO<serde_json::Value>;
}

// 常量
pub const CODE_SUCCESS: &str = "SUCCESS";
pub const CODE_FAIL: &str = "FAIL";
```

### ResultDTO\<T\> (genies::core)

兼容 Java 的响应模型：

```rust
#[derive(Debug, Serialize, Deserialize, Clone, Default, ToSchema)]
pub struct ResultDTO<T> {
    /// 状态码：1=成功, 0=失败
    pub status: Option<i32>,
    /// 消息
    pub message: Option<String>,
    /// 数据
    pub data: Option<T>,
}

impl<T> ResultDTO<T> {
    /// 创建成功响应
    pub fn success(message: &str, data: T) -> Self;
    
    /// 创建失败响应
    pub fn error(message: &str) -> Self;
    
    /// 创建无数据的成功响应
    pub fn success_empty(message: &str) -> ResultDTO<()>;
    
    /// 从 Result 创建响应
    pub fn from_result(arg: &Result<T>) -> Self;
    
    /// 从数据创建成功响应
    pub fn from(arg: &T) -> Self;
    
    /// 从消息和数据创建响应
    pub fn from_message(data: &T, message: &str) -> Self;
    
    /// 自定义状态码响应
    pub fn from_code_message(code: i32, message: &str, data: &T) -> Self;
}

// 常量
pub const CODE_SUCCESS_I32: i32 = 1;
pub const CODE_FAIL_I32: i32 = 0;
```

### Error (genies::core::error)

通用错误类型：

```rust
#[derive(Debug)]
#[non_exhaustive]
pub enum Error {
    E(String),
}

// 支持从多种类型转换
impl From<&str> for Error { ... }
impl From<String> for Error { ... }
impl From<io::Error> for Error { ... }
impl From<rbdc::Error> for Error { ... }
```

### ConfigError (genies::core::error)

配置相关错误类型：

```rust
#[derive(Debug, thiserror::Error)]
pub enum ConfigError {
    #[error("Validation error: {0}")]
    ValidationError(String),
    
    #[error("Parse error: {0}")]
    ParseError(String),
    
    #[error("Environment error: {0}")]
    EnvError(String),
    
    #[error("Build error: {0}")]
    BuildError(String),
    
    #[error("Reload error: {0}")]
    ReloadError(String),
    
    #[error("File error: {0}")]
    FileError(String),
    
    #[error("Conversion error: {0}")]
    ConversionError(String),
}
```

## JWTToken 结构与用法

### JWTToken 结构 (genies::core::jwt)

```rust
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct JWTToken {
    pub id: Option<String>,              // 账号 ID
    pub exp: Option<usize>,              // 过期时间
    pub iat: Option<usize>,              // 签发时间
    pub jti: Option<String>,             // JWT ID
    pub iss: Option<String>,             // 签发者
    pub sub: Option<String>,             // 主题
    pub typ: Option<String>,             // 类型
    pub azp: Option<String>,             // 授权方
    pub session_state: Option<String>,   // 会话状态
    pub acr: Option<String>,             // 认证上下文类引用
    pub realm_access: Option<Value>,     // Realm 访问权限
    pub resource_access: Option<Value>,  // 资源访问权限
    pub scope: Option<String>,           // 作用域
    pub department_name: Option<String>, // 部门名称
    pub department_code: Option<String>, // 部门编码
    pub department_id: Option<String>,   // 部门 ID
    pub roles: Option<Vec<String>>,      // 角色列表
    pub groups: Option<Vec<String>>,     // 组列表
    pub dept: Option<Vec<String>>,       // 部门列表
    pub preferred_username: Option<String>, // 首选用户名
    pub given_name: Option<String>,      // 名字
    pub user_id: Option<String>,         // 用户 ID
    pub name: Option<String>,            // 姓名
    pub department_abstract: Option<String>, // 部门摘要
}

impl JWTToken {
    /// 创建 JWT Token
    pub fn create_token(&self, secret: &str) -> Result<String, Error>;
    
    /// 使用密钥验证 Token
    pub fn verify(secret: &str, token: &str) -> Result<JWTToken, Error>;
    
    /// 使用 Keycloak 公钥验证 Token
    pub fn verify_with_keycloak(keycloak: &Keys, token: &str) -> Result<JWTToken, Error>;
}
```

### Keycloak 相关函数

```rust
/// 获取 Keycloak 公钥
pub async fn get_keycloak_keys(
    keycloak_auth_server_url: &str,
    keycloak_realm: &str
) -> Keys;

/// 获取临时访问令牌（Client Credentials 模式）
pub async fn get_temp_access_token(
    keycloak_auth_server_url: &str,
    keycloak_realm: &str,
    keycloak_resource: &str,
    keycloak_credentials_secret: &str
) -> String;
```

## ConditionTree 条件查询用法

`ConditionTree` 用于构建动态查询条件：

```rust
#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct ConditionTree {
    #[serde(alias = "o")]
    pub operator: Option<String>,        // 操作符
    #[serde(alias = "p")]
    pub propertyName: Option<String>,    // 属性名
    #[serde(alias = "v")]
    pub value: Option<String>,           // 比较值
    #[serde(alias = "c")]
    pub conditionTrees: Option<Vec<ConditionTree>>,  // 子条件
}

/// 测试对象是否满足条件
pub fn obj_test(obj_value: &Value, tree: &ConditionTree) -> bool;
```

### 支持的操作符

| 操作符 | 说明 | 适用类型 |
|--------|------|----------|
| `and` | 逻辑与 | 组合条件 |
| `or` | 逻辑或 | 组合条件 |
| `=` | 等于 | String, 数值 |
| `<>`, `!=` | 不等于 | String, 数值 |
| `<`, `<=`, `>`, `>=` | 比较 | 数值 |
| `contain` | 包含 | String |
| `!contain` | 不包含 | String |
| `arr_size_*` | 数组大小比较 | 数组 |
| `arr_exist_*` | 存在满足条件的元素 | 数组 |
| `arr_each_*` | 所有元素满足条件 | 数组 |

### 使用示例

```rust
use genies::core::condition::{ConditionTree, obj_test};
use serde_json::json;

let obj = json!({
    "name": "test",
    "age": 25,
    "tags": ["rust", "web"]
});

// 简单条件
let tree = ConditionTree {
    operator: Some("=".to_string()),
    propertyName: Some("name".to_string()),
    value: Some("test".to_string()),
    conditionTrees: None,
};
assert!(obj_test(&obj, &tree));

// 复合条件 (age > 20 AND name = "test")
let tree = ConditionTree {
    operator: Some("and".to_string()),
    propertyName: None,
    value: None,
    conditionTrees: Some(vec![
        ConditionTree {
            operator: Some(">".to_string()),
            propertyName: Some("age".to_string()),
            value: Some("20".to_string()),
            conditionTrees: None,
        },
        ConditionTree {
            operator: Some("=".to_string()),
            propertyName: Some("name".to_string()),
            value: Some("test".to_string()),
            conditionTrees: None,
        },
    ]),
};
assert!(obj_test(&obj, &tree));
```

## 项目 Crate 依赖关系图

```
                    ┌─────────────────┐
                    │     genies      │  (v1.4.5 - 主入口，重导出所有子 crate)
                    └────────┬────────┘
                             │
        ┌────────────────────┼────────────────────┐
        │                    │                    │
        ▼                    ▼                    ▼
┌───────────────┐   ┌───────────────┐   ┌───────────────┐
│ genies_config │   │genies_context │   │  genies_ddd   │
│   (v1.4.2)    │   │   (v1.4.3)    │   │   (v1.4.2)    │
│   配置管理    │   │  应用上下文   │   │   DDD 核心    │
└───────┬───────┘   └───────┬───────┘   └───────┬───────┘
        │                   │                   │
        │           ┌───────┴───────┐           │
        │           ▼               ▼           │
        │   ┌───────────────┐ ┌───────────────┐ │
        │   │ genies_cache  │ │  genies_dapr  │◄┘
        │   │   (v1.4.2)    │ │   (v1.4.2)    │
        │   │   缓存服务    │ │  Dapr 集成    │
        │   └───────────────┘ └───────────────┘
        │
        ▼
┌───────────────┐   ┌───────────────┐   ┌───────────────┐
│  genies_core  │   │genies_derive  │   │  genies_k8s   │
│   (v1.4.4)    │   │   (v1.4.5)    │   │   (v1.4.2)    │
│   核心基础    │   │   过程宏库    │   │  K8s 探针     │
└───────────────┘   └───────────────┘   └───────────────┘
```

### 各 Crate 职责说明

| Crate | 版本 | 职责 |
|-------|------|------|
| **genies** | 1.4.5 | 主框架入口，重导出所有子 crate，提供便捷宏 `pool!`、`tx_defer!`、`copy!`、`config_gateway!` |
| **genies_core** | 1.4.4 | 核心基础设施：错误处理、JWT 验证、HTTP 响应模型、条件查询 |
| **genies_derive** | 1.4.5 | 过程宏库：`DomainEvent`、`Aggregate`、`Config`、`ConfigCore`、`topic`、`wrapper`、`casbin` |
| **genies_config** | 1.4.2 | 配置管理：`ApplicationConfig` 定义、日志配置初始化 |
| **genies_context** | 1.4.3 | 全局上下文：`CONTEXT`、`REMOTE_TOKEN`、`SERVICE_STATUS`、JWT 认证中间件 |
| **genies_cache** | 1.4.2 | 缓存抽象层：`CacheService`、`ICacheService`、Redis/内存双后端 |
| **genies_dapr** | 1.4.2 | Dapr 集成：CloudEvent 解析、PubSub 订阅配置、Topic 注册 |
| **genies_ddd** | 1.4.2 | DDD 核心：聚合根 Trait、领域事件 Trait、消息发布器、Message 表模型 |
| **genies_k8s** | 1.4.2 | Kubernetes 探针：存活/就绪检查端点 |

### 第三方依赖

| 依赖 | 版本 | 用途 |
|------|------|------|
| rbatis | 4.5 | ORM 框架 |
| salvo | 0.79 | Web 框架 |
| tokio | 1.22 | 异步运行时 |
| serde | 1.0 | 序列化框架 |
| serde_json | 1.0 | JSON 处理 |
| serde_yaml | 0.9 | YAML 处理 |
| tracing | 0.1 | 日志框架 |
| async-trait | 0.1 | 异步 trait |
| inventory | 0.3 | 静态注册 |
| thiserror | 1.0 | 错误派生 |

## Message 表 DDL

用于事件存储的数据库表：

```sql
CREATE TABLE `message` (
  `id` varchar(36) NOT NULL COMMENT '消息ID',
  `destination` varchar(255) DEFAULT NULL COMMENT '目标Topic',
  `headers` text COMMENT '消息头JSON',
  `payload` text NOT NULL COMMENT '消息体JSON',
  `published` int(11) DEFAULT '0' COMMENT '发布状态: 0-未发布, 1-已发布',
  `creation_time` bigint(20) DEFAULT NULL COMMENT '创建时间戳(毫秒)',
  PRIMARY KEY (`id`),
  KEY `idx_published` (`published`),
  KEY `idx_destination` (`destination`)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COMMENT='领域事件消息表';
```

## 全局上下文 (CONTEXT)

```rust
// 访问全局上下文
use genies::context::CONTEXT;

// 初始化数据库连接
CONTEXT.init_mysql().await;

// 访问配置
let server_name = &CONTEXT.config.server_name;

// 访问数据库连接池
let rb = &CONTEXT.rbatis;

// 访问缓存服务
let cache = &CONTEXT.cache_service;

// 访问持久化缓存服务（用于事件幂等性）
let save_cache = &CONTEXT.redis_save_service;

// 访问 Keycloak 公钥
let keys = &CONTEXT.keycloak_keys;
```

## 服务状态管理

```rust
use genies::context::SERVICE_STATUS;
use std::ops::DerefMut;

// 设置服务不就绪
fn set_not_ready() {
    let mut status = SERVICE_STATUS.lock().unwrap();
    let map = status.deref_mut();
    map.insert("readinessProbe".to_string(), false);
}

// 设置服务不存活
fn set_not_alive() {
    let mut status = SERVICE_STATUS.lock().unwrap();
    let map = status.deref_mut();
    map.insert("livenessProbe".to_string(), false);
}

// 恢复服务状态
fn restore_ready() {
    let mut status = SERVICE_STATUS.lock().unwrap();
    let map = status.deref_mut();
    map.insert("readinessProbe".to_string(), true);
    map.insert("livenessProbe".to_string(), true);
}
```

## 跨微服务调用 Token 管理

```rust
use genies::context::REMOTE_TOKEN;

// 获取当前 Token
let token = REMOTE_TOKEN.lock().unwrap().access_token.clone();

// 手动更新 Token
REMOTE_TOKEN.lock().unwrap().access_token = "new_token".to_string();
```

注意：使用 `#[wrapper]` 宏时会自动管理 Token 刷新，一般不需要手动操作。
