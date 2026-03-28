# Auth Module Reference

本文档提供 genies_auth 模块的详细技术参考。

---

## 源代码中文说明总览

> 以下中文内容均来源于 `crates/auth/` 源代码中的注释、日志、变量名等。

### 模块功能概览（来源：lib.rs）

```
Auth 模块 - Casbin 权限管理系统

提供基于 Casbin 的完整权限管理方案，包括：
- API 接口级访问控制
- 字段级权限过滤
- 动态策略管理
- OpenApi Schema 自动同步

核心组件：
- EnforcerManager - Casbin Enforcer 管理器，支持热更新
- casbin_auth - API 权限中间件
- auth_admin_router - Admin API 路由
- extract_and_sync_schemas - Schema 同步函数
```

---

## 1. 完整的 Casbin 默认模型定义

```conf
[request_definition]
r = sub, obj, act

[policy_definition]
p = sub, obj, act, eft

[role_definition]
g = _, _
g2 = _, _

[policy_effect]
e = !some(where (p.eft == deny))

[matchers]
m = (g(r.sub, p.sub) || r.sub == p.sub) && (g2(r.obj, p.obj) || r.obj == p.obj || keyMatch2(r.obj, p.obj)) && r.act == p.act
```

### Casbin 模型中文解读

<!-- 来源：V4__seed_casbin_model_and_policies.sql -->

**模型描述**：`RBAC + 对象分组 + deny 黑名单模型`

| 配置行 | 业务含义 |
|--------|----------|
| `r = sub, obj, act` | 请求三要素：谁(用户/角色) 访问 什么(资源/字段) 做 什么操作(读/写/删) |
| `p = sub, obj, act, eft` | 策略四要素：指定 主体 对 对象 的 操作 是 允许还是拒绝 |
| `g = _, _` | 用户-角色映射：将用户分配到角色 |
| `g2 = _, _` | 对象分组映射：将资源/字段归类到分组 |
| `e = !some(where (p.eft == deny))` | **黑名单模式**：默认允许，只有存在 deny 规则时才拒绝 |
| `m = ...` | 匹配规则：支持直接匹配、角色继承、分组继承、路径通配符 |

### 各部分解释

#### request_definition
```conf
r = sub, obj, act
```
- `sub` - Subject，请求主体（用户名/角色名）
- `obj` - Object，请求对象（API 路径或字段名）
- `act` - Action，操作类型（HTTP 方法或 "read"）

#### policy_definition
```conf
p = sub, obj, act, eft
```
- `sub` - 策略适用的主体
- `obj` - 策略适用的对象
- `act` - 策略适用的操作
- `eft` - 效果，取值为 "allow" 或 "deny"

#### role_definition
```conf
g = _, _     # 用户-角色映射
g2 = _, _    # 对象分组映射
```
- `g` - 将用户映射到角色，如 `g, alice, admin` 表示 alice 拥有 admin 角色
- `g2` - 将资源映射到分组，如 `g2, /api/users, user:manage`

#### policy_effect
```conf
e = !some(where (p.eft == deny))
```
**黑名单模式**：默认允许，只有存在 `deny` 规则时才拒绝。

#### matchers
```conf
m = (g(r.sub, p.sub) || r.sub == p.sub) && (g2(r.obj, p.obj) || r.obj == p.obj || keyMatch2(r.obj, p.obj)) && r.act == p.act
```
匹配逻辑：
1. **主体匹配**: 直接匹配 或 通过角色继承匹配
2. **对象匹配**: 直接匹配 或 通过分组匹配 或 路径模式匹配（keyMatch2）
3. **操作匹配**: 精确匹配

`keyMatch2` 支持路径参数：
- `/api/users/*` 匹配 `/api/users/123`
- `/api/users/:id` 匹配 `/api/users/456`

---

## 2. 中间件完整工作流程

### 中间件功能说明

<!-- 来源：middleware.rs 模块文档 -->

```
Casbin API 接口访问控制中间件

提供基于 Casbin 的 API 权限检查，支持黑名单模式（默认允许，仅 deny 规则生效）。

功能特性：
- 从 JWT Token 提取用户身份（subject）
- 使用 Casbin Enforcer 进行权限检查
- 将 enforcer 和 subject 注入 Depot，供后续 Writer 字段过滤使用
```

### 请求处理流程

```
┌─────────────────────────────────────────────────────────────────────────┐
│                           HTTP Request                                   │
└─────────────────────────────────────────────────────────────────────────┘
                                    │
                                    ▼
┌─────────────────────────────────────────────────────────────────────────┐
│  salvo_auth 中间件                                                       │
│  ├── 解析 Authorization Header 中的 JWT Token                            │
│  ├── 验证 Token 签名和有效期                                              │
│  └── 将 JWTToken 存入 Depot: depot.insert("jwtToken", token)            │
└─────────────────────────────────────────────────────────────────────────┘
                                    │
                                    ▼
┌─────────────────────────────────────────────────────────────────────────┐
│  affix_state::inject(mgr) 中间件                                         │
│  └── 将 Arc<EnforcerManager> 注入 Depot                                  │
└─────────────────────────────────────────────────────────────────────────┘
                                    │
                                    ▼
┌─────────────────────────────────────────────────────────────────────────┐
│  casbin_auth 中间件                                                  │
│  ├── 1. 从 Depot 获取 JWTToken，提取 preferred_username 作为 subject     │
│  │      (若无 Token 则 subject = "guest")                                │
│  ├── 2. 从 Depot 获取 EnforcerManager，调用 get_enforcer()               │
│  ├── 3. 构造请求参数：(subject, path, method)                            │
│  ├── 4. 调用 enforcer.enforce() 进行权限检查                             │
│  │      ├── Ok(true)  → 允许，继续                                       │
│  │      ├── Ok(false) → 拒绝，返回 403 Forbidden                        │
│  │      └── Err(_)    → 引擎错误，返回 500                               │
│  └── 5. 注入 enforcer 和 subject 到 Depot（供 Writer 使用）              │
│         depot.insert("casbin_enforcer", enforcer);                       │
│         depot.insert("casbin_subject", subject);                         │
└─────────────────────────────────────────────────────────────────────────┘
                                    │
                                    ▼
┌─────────────────────────────────────────────────────────────────────────┐
│  Handler（业务逻辑）                                                      │
│  └── 返回带有 #[casbin] 宏的结构体                                        │
└─────────────────────────────────────────────────────────────────────────┘
                                    │
                                    ▼
┌─────────────────────────────────────────────────────────────────────────┐
│  Writer（字段级权限过滤）                                                 │
│  ├── 1. 从 Depot 提取 casbin_enforcer 和 casbin_subject                  │
│  ├── 2. 注入到结构体的 enforcer 和 subject 字段                          │
│  └── 3. 序列化时自动触发 check_permission() 过滤每个字段                  │
└─────────────────────────────────────────────────────────────────────────┘
                                    │
                                    ▼
┌─────────────────────────────────────────────────────────────────────────┐
│                           HTTP Response                                  │
│  (只包含用户有权限查看的字段)                                             │
└─────────────────────────────────────────────────────────────────────────┘
```

---

## 3. #[casbin] 宏生成的完整代码示例

### 原始结构体

```rust
#[casbin]
#[derive(Deserialize, ToSchema)]
pub struct User {
    pub id: u64,
    pub name: Option<String>,
    pub email: String,
}
```

### 宏展开后的完整代码

```rust
// 原始结构体（添加了两个字段）
#[derive(Deserialize, ToSchema)]
pub struct User {
    pub id: u64,
    pub name: Option<String>,
    pub email: String,
    #[serde(skip)]
    pub enforcer: Option<std::sync::Arc<casbin::Enforcer>>,
    #[serde(skip)]
    pub subject: Option<String>,
}

// with_policy 和 check_permission 方法
impl User {
    /// 设置权限策略上下文
    pub fn with_policy(mut self, enforcer: std::sync::Arc<casbin::Enforcer>, subject: String) -> Self {
        self.enforcer = Some(enforcer);
        self.subject = Some(subject);
        self
    }

    /// 检查字段权限
    /// 参数 field 是字段名（如 "email"）
    /// 返回 true 表示允许访问
    pub fn check_permission(&self, field: &str) -> bool {
        use casbin::CoreApi;
        // 获取 Schema 名称（如 "User"）
        let name = salvo::oapi::naming::assign_name::<User>(salvo::oapi::naming::NameRule::Auto);
        // 构造完整字段标识（如 "User.email"）
        let field_str = format!("{}.{}", name, field);
        
        match (&self.enforcer, &self.subject) {
            (Some(e), Some(s)) => {
                // 调用 Casbin 检查权限
                match e.enforce((s, &field_str, "read")) {
                    Ok(b) => {
                        log::debug!("权限检查:sub={},obj={},permission:{}", s, field_str, b);
                        return b;
                    }
                    Err(e) => {
                        log::warn!("权限检查--策略文件定义出错:sub={},obj={},permission:{},Err:{}", s, field_str, false, e);
                        return false;
                    }
                }
            }
            _ => {
                log::warn!("权限检查--enforcer或subject未设置:obj={},permission:{}", field_str, false);
                false
            }
        }
    }
}

// 自定义序列化实现（带权限检查）
impl ::serde::Serialize for User {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: ::serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut state = serializer.serialize_struct("User", 5)?;
        
        // 每个字段序列化前检查权限
        if self.check_permission("id") {
            state.serialize_field("id", &self.id)?;
        }
        if self.check_permission("name") {
            state.serialize_field("name", &self.name)?;
        }
        if self.check_permission("email") {
            state.serialize_field("email", &self.email)?;
        }
        // enforcer 和 subject 字段被过滤，不序列化
        
        state.end()
    }
}

// Salvo Writer trait 实现
#[async_trait::async_trait]
impl salvo::writing::Writer for User {
    async fn write(mut self, _req: &mut salvo::Request, depot: &mut salvo::Depot, res: &mut salvo::Response) {
        // 自动从 Depot 提取 enforcer（由 casbin_auth 中间件注入）
        if let Ok(enforcer) = depot.get::<std::sync::Arc<casbin::Enforcer>>("casbin_enforcer") {
            self.enforcer = Some(enforcer.clone());
        }
        // 自动从 Depot 提取 subject（由 casbin_auth 中间件注入）
        if let Ok(subject) = depot.get::<String>("casbin_subject") {
            self.subject = Some(subject.clone());
        }
        // 序列化时自动触发 check_permission 过滤字段
        res.render(salvo::prelude::Json(&self));
    }
}
```

---

## 4. EnforcerManager 并发安全详解

### 结构定义

```rust
pub struct EnforcerManager {
    /// Enforcer 实例，使用 Arc 支持并发读取快照
    enforcer: RwLock<Arc<Enforcer>>,
}
```

### RwLock 策略

- **读操作 (get_enforcer)**：获取读锁，clone Arc 引用后立即释放
- **写操作 (reload)**：先在锁外构建新 Enforcer，再获取写锁原子替换

### 关键方法

```rust
/// 获取 Enforcer 快照（高并发安全）
pub async fn get_enforcer(&self) -> Arc<Enforcer> {
    // 获取读锁，clone Arc 后立即释放
    // 即使后续发生 reload，持有的快照仍然有效
    self.enforcer.read().await.clone()
}

/// 热更新：从数据库重新加载
pub async fn reload(&self) -> anyhow::Result<()> {
    // 1. 先构建新的 Enforcer（不持有锁）
    let new_enforcer = Self::build_enforcer().await?;

    // 2. 获取写锁并原子替换
    let mut guard = self.enforcer.write().await;
    *guard = Arc::new(new_enforcer);

    // 3. 清除 Redis 缓存并更新版本号
    if let Err(e) = cache::invalidate_and_reload().await {
        log::warn!("清除 Redis 缓存失败: {}", e);
    }

    Ok(())
}
```

### 并发安全性保证

1. **读-读并发**: 多个 get_enforcer() 可以同时执行
2. **读-写互斥**: reload() 执行时会阻塞所有 get_enforcer()
3. **快照一致性**: get_enforcer() 返回的 Arc<Enforcer> 是不可变的快照
4. **失败安全**: build_enforcer() 失败不影响现有 Enforcer

---

## 5. Redis 缓存层详细 API

### 常量定义

```rust
const KEY_POLICIES: &str = "auth:policies";        // 策略规则缓存
const KEY_SCHEMAS: &str = "auth:schemas";          // Schema 信息缓存
const KEY_ENFORCER_VERSION: &str = "auth:enforcer_ver";  // 版本号
```

### 策略规则缓存

```rust
/// 缓存策略规则到 Redis
pub async fn cache_policies(rules: &[CasbinRule]) -> anyhow::Result<()>

/// 从 Redis 加载缓存的策略规则
/// 返回 None 表示缓存未命中，应回退到数据库
pub async fn load_cached_policies() -> anyhow::Result<Option<Vec<CasbinRule>>>
```

### Schema 缓存

```rust
/// 缓存 Schema 信息（JSON 字符串）
pub async fn cache_schemas(schemas_json: &str) -> anyhow::Result<()>

/// 加载缓存的 Schema 信息
pub async fn load_cached_schemas() -> anyhow::Result<Option<String>>
```

### 缓存失效与版本控制

```rust
/// 清除缓存并更新版本号（供 reload 调用）
pub async fn invalidate_and_reload() -> anyhow::Result<()>

/// 获取当前 Enforcer 版本号（用于多实例同步）
pub async fn get_enforcer_version() -> anyhow::Result<Option<String>>
```

### 多实例同步机制

1. 实例 A 调用 `reload()` → 调用 `invalidate_and_reload()`
2. `invalidate_and_reload()` 删除缓存 + 更新版本号（时间戳）
3. 其他实例定期检查 `get_enforcer_version()` 与本地版本对比
4. 版本不一致时触发本地 `reload()`

---

## 6. Schema 提取完整流程

### 函数签名

```rust
pub async fn extract_and_sync_schemas(doc: &OpenApi) -> Result<()>
```

### 处理流程

```
┌──────────────────────────────────────────────────────────────────┐
│  1. 序列化 OpenApi 文档为 JSON                                    │
│     serde_json::to_value(doc)                                    │
└──────────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌──────────────────────────────────────────────────────────────────┐
│  2. extract_schemas() - 提取 Schema 定义                          │
│     遍历 components.schemas 节点                                  │
│     ├── 解析每个 Schema 的 properties                             │
│     └── 提取字段名和字段类型                                       │
│                                                                  │
│  返回: Vec<SchemaInfo> { name, fields: Vec<(field_name, type)> } │
└──────────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌──────────────────────────────────────────────────────────────────┐
│  3. extract_endpoints() - 提取 API 端点                           │
│     遍历 paths 节点                                               │
│     ├── 解析每个路径的 HTTP 方法 (get/post/put/delete...)          │
│     ├── 提取 summary 描述                                         │
│     └── 从 responses/requestBody 提取关联的 Schema $ref           │
│                                                                  │
│  返回: Vec<EndpointInfo> { path, method, summary, schema_refs }  │
└──────────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌──────────────────────────────────────────────────────────────────┐
│  4. merge_schema_endpoints() - 合并信息                           │
│     构建 Schema → Endpoint 映射表                                 │
│     为每个 Schema 的每个字段生成 SchemaFieldRecord                 │
│                                                                  │
│  返回: Vec<SchemaFieldRecord>                                    │
└──────────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌──────────────────────────────────────────────────────────────────┐
│  5. sync_to_db() - 写入数据库                                     │
│     INSERT ... ON DUPLICATE KEY UPDATE                           │
│     ├── 新记录：插入所有字段                                       │
│     └── 已存在：更新 field_type, endpoint_path, http_method       │
│         保留 schema_label, field_label 的手动设置值               │
└──────────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌──────────────────────────────────────────────────────────────────┐
│  6. cache_schemas() - 写入 Redis 缓存                             │
│     序列化为 JSON 存储到 auth:api_schemas                         │
└──────────────────────────────────────────────────────────────────┘
```

### SchemaFieldRecord 结构

```rust
pub struct SchemaFieldRecord {
    pub schema_name: String,        // Schema 名称，如 "genies_auth.vo.User"
    pub schema_label: Option<String>,  // 中文标签（管理员设置）
    pub field_name: String,         // 字段名
    pub field_label: Option<String>,   // 字段中文标签
    pub field_type: Option<String>,    // 字段类型，如 "string", "array<User>"
    pub endpoint_path: Option<String>, // 关联的 API 路径
    pub endpoint_label: Option<String>,// 端点描述
    pub http_method: Option<String>,   // HTTP 方法
}
```

---

## 7. Admin API 请求/响应格式

### Admin API 端点中文说明

<!-- 来源：admin_api.rs 模块文档和函数注释 -->

```
Auth 模块的 Admin API 端点

提供权限管理的 REST API，包括：
- API Schema 查询
- Casbin 模型定义管理
- 策略规则管理（policy/role/group）
- Enforcer 热重载
```

| 端点 | 方法 | 功能说明 | 中文标签建议 |
|------|------|----------|-------------|
| `/auth/schemas` | GET | 列出所有 API Schema 和字段 | 获取Schema列表 |
| `/auth/model` | GET | 获取当前 Casbin 模型定义 | 获取权限模型 |
| `/auth/model` | PUT | 修改 Casbin 模型定义（更新后自动重载 Enforcer） | 更新权限模型 |
| `/auth/policies` | GET | 列出所有策略规则 | 获取策略列表 |
| `/auth/policies` | POST | 添加策略规则（添加后自动重载 Enforcer） | 添加策略规则 |
| `/auth/policies/<id>` | DELETE | 删除策略规则（删除后自动重载 Enforcer） | 删除策略规则 |
| `/auth/roles` | GET | 列出角色分配 (g 类型) | 获取角色分配 |
| `/auth/roles` | POST | 添加用户到角色的映射（ptype='g'） | 添加角色分配 |
| `/auth/roles/<id>` | DELETE | 移除角色分配 | 移除角色分配 |
| `/auth/groups` | GET | 列出对象分组 (g2 类型) | 获取对象分组 |
| `/auth/groups` | POST | 添加资源到分组的映射（ptype='g2'） | 添加对象分组 |
| `/auth/groups/<id>` | DELETE | 移除对象分组 | 移除对象分组 |
| `/auth/reload` | POST | 手动触发 Enforcer 重载（从数据库重新加载模型和策略） | 重载权限引擎 |

### 通用响应格式

<!-- 来源：admin_api.rs ApiResponse 结构体注释 -->

```rust
/// 通用 API 响应
pub struct ApiResponse<T: Serialize> {
    /// 响应码，"0" 表示成功，其他表示错误
    pub code: String,
    /// 响应消息
    pub msg: String,
    /// 响应数据
    pub data: Option<T>,
}
```

### PolicyDto（添加策略/角色/分组）

<!-- 来源：admin_api.rs PolicyDto 结构体注释 -->

```rust
/// 策略规则 DTO（用于添加策略）
pub struct PolicyDto {
    /// 策略类型: "p" (策略), "g" (角色), "g2" (分组)
    pub ptype: String,
    /// 第一个参数（通常是 subject/用户/角色）
    pub v0: String,
    /// 第二个参数（通常是 object/资源/角色）
    pub v1: String,
    /// 第三个参数（通常是 action/操作）
    pub v2: String,
    /// 第四个参数（可选扩展字段）
    pub v3: String,
    /// 第五个参数（可选扩展字段）
    pub v4: String,
    /// 第六个参数（可选扩展字段）
    pub v5: String,
}
```

### PolicyRecord（策略查询结果）

<!-- 来源：admin_api.rs PolicyRecord 结构体注释 -->

```rust
/// 策略规则查询结果（含 id）
pub struct PolicyRecord {
    /// 数据库记录 ID
    pub id: i64,
    /// 策略类型
    pub ptype: String,
    /// 第一个参数
    pub v0: String,
    /// 第二个参数
    pub v1: String,
    /// 第三个参数
    pub v2: String,
    /// 第四个参数
    pub v3: String,
    /// 第五个参数
    pub v4: String,
    /// 第六个参数
    pub v5: String,
}
```

### ModelDto（更新模型）

<!-- 来源：admin_api.rs ModelDto 结构体注释 -->

```rust
/// Casbin 模型 DTO
pub struct ModelDto {
    /// 模型名称
    pub model_name: String,
    /// 模型定义文本（Casbin 模型格式）
    pub model_text: String,
    /// 模型描述
    pub description: Option<String>,
}
```

### SchemaRecord（Schema 查询结果）

<!-- 来源：admin_api.rs SchemaRecord 结构体注释 -->

```rust
/// API Schema 查询结果
pub struct SchemaRecord {
    /// 数据库记录 ID
    pub id: i64,
    /// Schema 名称
    pub schema_name: String,
    /// Schema 标签（中文名称，管理员设置）
    pub schema_label: Option<String>,
    /// 字段名称
    pub field_name: String,
    /// 字段标签（中文名称，管理员设置）
    pub field_label: Option<String>,
    /// 字段类型
    pub field_type: Option<String>,
    /// 端点路径
    pub endpoint_path: Option<String>,
    /// 端点标签（中文名称，管理员设置）
    pub endpoint_label: Option<String>,
    /// HTTP 方法
    pub http_method: Option<String>,
}
```

### API 调用示例

```bash
# 添加策略
curl -X POST http://localhost:8080/auth/policies \
  -H "Content-Type: application/json" \
  -d '{"ptype":"p","v0":"bob","v1":"User.email","v2":"read","v3":"deny","v4":"","v5":""}'

# 添加角色映射
curl -X POST http://localhost:8080/auth/roles \
  -H "Content-Type: application/json" \
  -d '{"v0":"alice","v1":"admin","v2":"","v3":"","v4":"","v5":""}'

# 删除策略
curl -X DELETE http://localhost:8080/auth/policies/1

# 重载 Enforcer
curl -X POST http://localhost:8080/auth/reload
```

---

## 8. 数据库表完整 DDL

### 字段中文标签参考

<!-- 来源：admin_api.rs 和 schema_extractor.rs 中的结构体注释 -->

#### casbin_rules 表字段说明

| 字段名 | 中文说明 | 配置提示 |
|--------|----------|----------|
| `id` | 数据库记录 ID | 自动生成，用于删除操作 |
| `ptype` | 策略类型 | `p`=策略规则, `g`=角色分配, `g2`=对象分组 |
| `v0` | 第一个参数 | 通常是 subject/用户/角色 |
| `v1` | 第二个参数 | 通常是 object/资源/角色 |
| `v2` | 第三个参数 | 通常是 action/操作 |
| `v3` | 第四个参数 | 可选扩展字段，对于 p 类型通常是 eft(allow/deny) |
| `v4` | 第五个参数 | 可选扩展字段 |
| `v5` | 第六个参数 | 可选扩展字段 |

#### auth_api_schemas 表字段说明

| 字段名 | 中文说明 | 配置提示 |
|--------|----------|----------|
| `id` | 数据库记录 ID | 自动生成 |
| `schema_name` | Schema 名称 | 如 `genies_auth.vo.User`，自动提取 |
| `schema_label` | Schema 中文标签 | 管理员设置，如 "用户信息" |
| `field_name` | 字段名称 | 如 `email`, `phone`，自动提取 |
| `field_label` | 字段中文标签 | 管理员设置，如 "邮箱"、"手机号" |
| `field_type` | 字段类型 | 如 `string`, `integer`, `array<User>` |
| `endpoint_path` | 关联的 API 路径 | 如 `/api/users`，自动提取 |
| `endpoint_label` | 端点中文标签 | 管理员设置，如 "获取用户列表" |
| `http_method` | HTTP 方法 | 如 `GET`, `POST`, `PUT`, `DELETE` |

### casbin_rules（策略规则表）

```sql
CREATE TABLE IF NOT EXISTS casbin_rules (
    id BIGINT PRIMARY KEY AUTO_INCREMENT,    -- 数据库记录 ID
    ptype VARCHAR(16) NOT NULL,              -- 策略类型: p(策略)/g(角色)/g2(分组)
    v0 VARCHAR(256) NOT NULL DEFAULT '',     -- 第一个参数（通常是 subject/用户/角色）
    v1 VARCHAR(256) NOT NULL DEFAULT '',     -- 第二个参数（通常是 object/资源/角色）
    v2 VARCHAR(256) NOT NULL DEFAULT '',     -- 第三个参数（通常是 action/操作）
    v3 VARCHAR(256) NOT NULL DEFAULT '',     -- 第四个参数（可选，通常是 eft:allow/deny）
    v4 VARCHAR(256) NOT NULL DEFAULT '',     -- 第五个参数（可选扩展字段）
    v5 VARCHAR(256) NOT NULL DEFAULT '',     -- 第六个参数（可选扩展字段）
    created_at DATETIME DEFAULT CURRENT_TIMESTAMP
);
```

### casbin_model（模型定义表）

```sql
CREATE TABLE IF NOT EXISTS casbin_model (
    id BIGINT PRIMARY KEY AUTO_INCREMENT,    -- 数据库记录 ID
    model_name VARCHAR(128) NOT NULL DEFAULT 'default',  -- 模型名称
    model_text TEXT NOT NULL,                -- 模型定义文本（Casbin 模型格式）
    description VARCHAR(512),                -- 模型描述
    created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
    updated_at DATETIME DEFAULT CURRENT_TIMESTAMP ON UPDATE CURRENT_TIMESTAMP,
    UNIQUE KEY uk_model_name (model_name)
);
```

### auth_api_schemas（API Schema 元数据表）

```sql
CREATE TABLE IF NOT EXISTS auth_api_schemas (
    id BIGINT PRIMARY KEY AUTO_INCREMENT,    -- 数据库记录 ID
    schema_name VARCHAR(256) NOT NULL,       -- Schema 完整名称（如 genies_auth.vo.User）
    schema_label VARCHAR(256),               -- Schema 中文标签（管理员设置）
    field_name VARCHAR(128) NOT NULL,        -- 字段名称
    field_label VARCHAR(256),                -- 字段中文标签（管理员设置）
    field_type VARCHAR(64),                  -- 字段类型（如 string, integer, array<User>）
    endpoint_path VARCHAR(256),              -- 关联的 API 端点路径
    endpoint_label VARCHAR(256),             -- 端点中文标签（管理员设置）
    http_method VARCHAR(16),                 -- HTTP 方法（GET/POST/PUT/DELETE）
    created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
    updated_at DATETIME DEFAULT CURRENT_TIMESTAMP ON UPDATE CURRENT_TIMESTAMP,
    UNIQUE KEY uk_schema_field (schema_name, field_name)
);
```

---

## 9. 字段级权限工作流程

### 完整数据流

```
┌─────────────────────────────────────────────────────────────────────────┐
│  1. 定义阶段                                                            │
│                                                                         │
│  #[casbin]                                                              │
│  struct User {                                                          │
│      id: u64,                                                           │
│      email: String,   // ← 敏感字段                                     │
│  }                                                                      │
│                                                                         │
│  宏生成: enforcer, subject 字段 + check_permission() + Writer impl      │
└─────────────────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────────────────┐
│  2. 配置阶段                                                            │
│                                                                         │
│  -- 在数据库中配置策略                                                   │
│  INSERT INTO casbin_rules (ptype,v0,v1,v2,v3)                           │
│  VALUES ('p', 'bob', 'User.email', 'read', 'deny');                     │
│                                                                         │
│  -- bob 无法查看 User.email 字段                                        │
└─────────────────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────────────────┐
│  3. 请求阶段                                                            │
│                                                                         │
│  GET /api/users/1                                                       │
│  Authorization: Bearer <bob's token>                                    │
└─────────────────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────────────────┐
│  4. 中间件处理                                                          │
│                                                                         │
│  casbin_auth:                                                       │
│  ├── 提取 subject = "bob"                                               │
│  ├── API 权限检查通过（未配置 deny）                                     │
│  └── 注入 depot["casbin_enforcer"], depot["casbin_subject"]             │
└─────────────────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────────────────┐
│  5. Handler 执行                                                        │
│                                                                         │
│  async fn get_user() -> Json<User> {                                    │
│      Json(User {                                                        │
│          id: 1,                                                         │
│          email: "bob@example.com".into(),                               │
│          enforcer: None,  // 暂时为空                                   │
│          subject: None,                                                 │
│      })                                                                 │
│  }                                                                      │
└─────────────────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────────────────┐
│  6. Writer 处理                                                         │
│                                                                         │
│  impl Writer for User {                                                 │
│      async fn write(...) {                                              │
│          // 从 Depot 提取并注入                                         │
│          self.enforcer = depot.get("casbin_enforcer");                  │
│          self.subject = depot.get("casbin_subject");  // "bob"          │
│          res.render(Json(&self));                                       │
│      }                                                                  │
│  }                                                                      │
└─────────────────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────────────────┐
│  7. 序列化（字段过滤）                                                   │
│                                                                         │
│  impl Serialize for User {                                              │
│      fn serialize(...) {                                                │
│          // check_permission("id") → true → 序列化                      │
│          if self.check_permission("id") {                               │
│              state.serialize_field("id", &self.id)?;                    │
│          }                                                              │
│                                                                         │
│          // check_permission("email") → 调用 enforcer.enforce()         │
│          //   → enforce(("bob", "User.email", "read"))                  │
│          //   → 匹配策略 ('p','bob','User.email','read','deny')         │
│          //   → 返回 false（被 deny）                                   │
│          if self.check_permission("email") {                            │
│              state.serialize_field("email", &self.email)?; // 跳过      │
│          }                                                              │
│      }                                                                  │
│  }                                                                      │
└─────────────────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────────────────┐
│  8. 最终响应                                                            │
│                                                                         │
│  {                                                                      │
│      "id": 1                                                            │
│      // email 字段被过滤，不返回                                        │
│  }                                                                      │
└─────────────────────────────────────────────────────────────────────────┘
```

### 关键检查点

| 检查点 | 权限类型 | 检查对象 | 检查动作 |
|--------|----------|----------|----------|
| casbin_auth | API 级 | `/api/users/1` | `get` |
| check_permission("id") | 字段级 | `User.id` | `read` |
| check_permission("email") | 字段级 | `User.email` | `read` |

### 策略示例汇总

```sql
-- API 级权限
INSERT INTO casbin_rules (ptype,v0,v1,v2,v3) VALUES
    ('p', 'guest', '/api/admin', 'get', 'deny'),        -- guest 禁止访问 /api/admin
    ('p', 'nurse', 'user:manage', 'delete', 'deny');    -- nurse 禁止删除用户

-- 字段级权限
INSERT INTO casbin_rules (ptype,v0,v1,v2,v3) VALUES
    ('p', 'bob', 'User.email', 'read', 'deny'),         -- bob 禁止看 email
    ('p', 'alice', 'User.phone', 'read', 'deny');       -- alice 禁止看 phone

-- 角色分配
INSERT INTO casbin_rules (ptype,v0,v1) VALUES
    ('g', 'alice', 'admin'),                            -- alice 是 admin
    ('g', 'bob', 'viewer');                             -- bob 是 viewer

-- 资源分组
INSERT INTO casbin_rules (ptype,v0,v1) VALUES
    ('g2', '/api/users', 'user:manage'),                -- /api/users 属于 user:manage
    ('g2', 'User.phone', 'sensitive_fields');           -- User.phone 属于敏感字段组
```
