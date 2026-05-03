# genies_auth_admin

Genies (神灯) 框架的统一认证管理中心 — 提供用户、角色、权限、组织架构和多应用授权的完整管理后台。

## 概述

genies_auth_admin 是神灯权限体系的**管理界面**。`genies_auth` 作为核心 RBAC 权限引擎库（Casbin Enforcer、中间件、字段级过滤），而 `genies_auth_admin` 则提供完整的管理后台：

- 一整套 RESTful API，覆盖用户、角色、权限、部门、应用管理
- 内置 JWT 认证（登录 / 登出 / Token 刷新）
- Vue 3 + Element Plus Web UI，通过 `rust-embed` 直接嵌入到二进制中
- 多应用 API 代理 — 在统一面板中管理远程微服务的 Casbin 策略
- 领域驱动设计（DDD）分层架构

## 核心特性

- **用户管理**：增删改查、状态切换、密码重置、角色分配、权限查询、批量删除
- **角色管理**：增删改查、查看角色下的用户、权限分配/撤销
- **权限管理**：细粒度权限项的增删改查
- **部门管理**：增删改查、树形移动、按部门查看用户
- **应用注册**：注册微服务及其访问地址，远程管理其授权
- **API 代理**：将策略/角色/分组/Schema/重载请求转发到目标微服务的 `/auth/*` 端点
- **本地 JWT 认证**：自包含的登录流程，bcrypt 密码哈希 + JWT 令牌签发
- **Casbin 集成**：所有受保护路由经过 JWT 认证 + Casbin RBAC 权限检查
- **字段级权限过滤**：继承 `genies_auth` 的 `#[casbin]` 宏，实现响应字段过滤
- **内嵌 Web UI**：SPA 前端通过 `/auth-admin/ui/` 提供服务，智能缓存控制
- **OpenAPI 自动同步**：从 OpenAPI 文档提取 Schema 并同步到权限系统
- **Flyway 数据库迁移**：启动时自动创建所有所需数据库表
- **Dapr 事件总线**：CRUD 操作完成后发布 CloudEvent，下游 `genies_auth` 同步 Casbin 规则

## 架构设计

### DDD 分层结构

```
src/
├── main.rs                    # 启动入口 — 初始化、迁移、路由、服务启动
├── lib.rs                     # 库入口
├── interfaces/                # 接口层
│   ├── router.rs              #   路由定义（公开路由 + 受保护路由）
│   ├── admin_ui.rs            #   内嵌 SPA 静态资源服务
│   ├── handler/               #   HTTP 处理器
│   │   ├── auth_handler.rs    #     登录 / 登出 / 刷新 / 当前用户 / 修改密码
│   │   ├── user_handler.rs    #     用户 CRUD + 角色分配
│   │   ├── role_handler.rs    #     角色 CRUD + 权限分配
│   │   ├── permission_handler.rs  # 权限 CRUD
│   │   ├── department_handler.rs  # 部门 CRUD + 移动
│   │   ├── application_handler.rs # 应用注册 CRUD
│   │   └── app_proxy_handler.rs   # 多应用 API 代理
│   └── dto/                   #   请求/响应 DTO
├── application/               # 应用层
│   ├── service.rs             #   AuthService、UserService、RoleService 等
│   ├── app_service.rs         #   ApplicationAppService
│   └── dto.rs                 #   共享 DTO（LoginResponse、PageQuery 等）
├── domain/                    # 领域层
│   ├── entity/                #   AdminUser、AdminRole、AdminPermission、AdminDepartment、ApplicationEntity
│   ├── aggregate/             #   聚合根（User、Role、Permission、Department）
│   ├── service/               #   UserDomainService、RoleDomainService、ApplicationDomainService
│   ├── repository/            #   RBatis 仓储实现
│   └── event/                 #   领域事件（UserEvent、RoleEvent）
└── infrastructure/            # 基础设施层
    └── migration.rs           #   Flyway 迁移运行器
```

### 中间件执行流程

```
请求 → JWT 认证 (local_auth) → Casbin RBAC (casbin_auth) → 业务 Handler → Writer (字段过滤) → 响应
```

公开路由（登录、登出、刷新 Token、Admin UI）跳过认证。

## 技术栈

| 类别 | 技术 |
|------|------|
| Web 框架 | [Salvo](https://salvo.rs) |
| ORM | [RBatis](https://rbatis.github.io/rbatis.io/) |
| 授权引擎 | [Casbin](https://casbin.org/) 2.10 |
| 密码哈希 | bcrypt |
| 令牌 | jsonwebtoken (JWT) |
| 数据库迁移 | Flyway (flyway + flyway-rbatis) |
| 数据库 | MySQL |
| 事件总线 | Dapr pub/sub (CloudEvents) |
| 缓存 | Redis |
| 静态嵌入 | rust-embed |
| 前端框架 | Vue 3 + Element Plus + Vue Router + Axios |
| 构建工具 | Vite 5 + TypeScript |

## API 接口参考

### 公开路由（无需认证）

| 端点 | 方法 | 功能 |
|------|------|------|
| `/auth-admin/login` | POST | 用户名密码登录 |
| `/auth-admin/logout` | POST | 登出 |
| `/auth-admin/refresh` | POST | 刷新 JWT Token |
| `/auth-admin/ui/` | GET | 管理后台 Web UI |

### 受保护路由（JWT + Casbin）

#### 认证

| 端点 | 方法 | 功能 |
|------|------|------|
| `/auth-admin/me` | GET | 获取当前用户信息 |
| `/auth-admin/me/password` | PUT | 修改个人密码 |

#### 用户管理

| 端点 | 方法 | 功能 |
|------|------|------|
| `/auth-admin/users` | GET | 用户列表（分页） |
| `/auth-admin/users` | POST | 创建用户 |
| `/auth-admin/users/{id}` | GET | 用户详情 |
| `/auth-admin/users/{id}` | PUT | 更新用户 |
| `/auth-admin/users/{id}` | DELETE | 删除用户 |
| `/auth-admin/users/{id}/status` | PUT | 切换用户状态 |
| `/auth-admin/users/{id}/reset-password` | PUT | 重置用户密码 |
| `/auth-admin/users/{id}/roles` | GET | 查看用户角色 |
| `/auth-admin/users/{id}/roles` | POST | 分配角色 |
| `/auth-admin/users/{id}/roles/{role_id}` | DELETE | 撤销角色 |
| `/auth-admin/users/{id}/permissions` | GET | 查看用户权限 |
| `/auth-admin/users/batch-delete` | POST | 批量删除用户 |

#### 角色管理

| 端点 | 方法 | 功能 |
|------|------|------|
| `/auth-admin/roles` | GET | 角色列表 |
| `/auth-admin/roles` | POST | 创建角色 |
| `/auth-admin/roles/{id}` | GET | 角色详情 |
| `/auth-admin/roles/{id}` | PUT | 更新角色 |
| `/auth-admin/roles/{id}` | DELETE | 删除角色 |
| `/auth-admin/roles/{id}/users` | GET | 查看角色下的用户 |
| `/auth-admin/roles/{id}/permissions` | GET | 查看角色权限 |
| `/auth-admin/roles/{id}/permissions` | POST | 分配权限 |
| `/auth-admin/roles/{id}/permissions/{perm_id}` | DELETE | 撤销权限 |

#### 权限管理

| 端点 | 方法 | 功能 |
|------|------|------|
| `/auth-admin/permissions` | GET | 权限列表 |
| `/auth-admin/permissions` | POST | 创建权限 |
| `/auth-admin/permissions/{id}` | GET | 权限详情 |
| `/auth-admin/permissions/{id}` | PUT | 更新权限 |
| `/auth-admin/permissions/{id}` | DELETE | 删除权限 |

#### 部门管理

| 端点 | 方法 | 功能 |
|------|------|------|
| `/auth-admin/departments` | GET | 部门列表 |
| `/auth-admin/departments` | POST | 创建部门 |
| `/auth-admin/departments/{id}` | GET | 部门详情 |
| `/auth-admin/departments/{id}` | PUT | 更新部门 |
| `/auth-admin/departments/{id}` | DELETE | 删除部门 |
| `/auth-admin/departments/{id}/move/{parent_id}` | PUT | 移动部门 |
| `/auth-admin/departments/{id}/users` | GET | 查看部门下的用户 |

#### 应用管理

| 端点 | 方法 | 功能 |
|------|------|------|
| `/auth-admin/apps` | GET | 应用列表 |
| `/auth-admin/apps` | POST | 注册应用 |
| `/auth-admin/apps/{id}` | GET | 应用详情 |
| `/auth-admin/apps/{id}` | PUT | 更新应用 |
| `/auth-admin/apps/{id}` | DELETE | 删除应用 |

#### 应用代理（转发到目标微服务）

| 端点 | 方法 | 功能 |
|------|------|------|
| `/auth-admin/apps/{id}/schemas` | GET | 代理：查询目标应用的 API Schema |
| `/auth-admin/apps/{id}/policies` | GET | 代理：查询目标应用的 Casbin 策略 |
| `/auth-admin/apps/{id}/policies` | POST | 代理：添加策略到目标应用 |
| `/auth-admin/apps/{id}/policies/{policy_id}` | DELETE | 代理：删除目标应用的策略 |
| `/auth-admin/apps/{id}/roles` | GET | 代理：查询目标应用的角色映射 |
| `/auth-admin/apps/{id}/roles` | POST | 代理：添加角色映射到目标应用 |
| `/auth-admin/apps/{id}/roles/{role_id}` | DELETE | 代理：删除目标应用的角色映射 |
| `/auth-admin/apps/{id}/groups` | GET | 代理：查询目标应用的分组 |
| `/auth-admin/apps/{id}/groups` | POST | 代理：添加分组到目标应用 |
| `/auth-admin/apps/{id}/groups/{group_id}` | DELETE | 代理：删除目标应用的分组 |
| `/auth-admin/apps/{id}/reload` | POST | 代理：重载目标应用的 Enforcer |

## 数据库表

通过 Flyway 迁移自动创建：

| 表名 | 说明 |
|------|------|
| `auth_admin_users` | 管理员用户账户 |
| `auth_admin_roles` | 角色定义 |
| `auth_admin_permissions` | 权限项 |
| `auth_admin_departments` | 部门/组织架构树 |
| `auth_admin_user_roles` | 用户-角色关联 |
| `auth_admin_role_permissions` | 角色-权限关联 |
| `auth_admin_applications` | 注册的微服务应用 |
| `message` | Dapr 消息发件箱 |

来自 `genies_auth` 迁移的表（优先创建）：

| 表名 | 说明 |
|------|------|
| `casbin_rules` | Casbin 策略规则 |
| `casbin_model` | Casbin 模型定义 |
| `auth_api_schemas` | API Schema 元信息 |

## 配置说明

`application.yml` 关键配置项：

```yaml
server_name: "auth-admin"
servlet_path: "/auth-admin"
server_url: "0.0.0.0:9099"

database_url: "mysql://root:password@127.0.0.1:3306/auth_admin_service"

# JWT
jwt_secret: "auth_admin_jwt_secret_change_in_production"
auth_mode: "local"

# Redis
cache_type: "redis"
redis_url: "redis://127.0.0.1:6379"

# Dapr
dapr_pubsub_name: messagebus
```

## 快速开始

### 环境要求

- Rust 1.75+
- MySQL 5.7+ / 8.0
- Redis
- （可选）Dapr 运行时，用于事件驱动同步

### 启动服务

```bash
cargo run -p genies_auth_admin
```

服务启动地址：`http://127.0.0.1:9099`

### 访问 Web UI

在浏览器中打开：

```
http://127.0.0.1:9099/auth-admin/ui/
```

> **注意**：URL 末尾的斜杠 `/` 是必须的，否则 SPA 无法正确加载。

### 前端开发

```bash
cd crates/auth-admin/web
npm install
npm run dev      # 开发服务器（热更新）
npm run build    # 生产构建 → ../static/
```

## 与 genies_auth 的关系

| Crate | 定位 |
|-------|------|
| `genies_auth` | **权限引擎库** — Casbin Enforcer、中间件（`casbin_auth`）、字段级过滤（`#[casbin]` 宏）、Admin API（策略 CRUD）、OpenAPI Schema 同步 |
| `genies_auth_admin` | **管理后台** — 用户/角色/权限/部门/应用 CRUD、本地 JWT 登录、Web UI、多应用代理；依赖 `genies_auth` 提供认证和授权能力 |

`genies_auth_admin` 以 `genies_auth` 作为认证和授权骨架：所有受保护路由都经过 `genies_auth` 的 `local_auth`（JWT 验证）和 `casbin_auth`（RBAC 检查）中间件。

## 许可证

请参阅项目根目录的许可证信息。
