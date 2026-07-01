---
name: genies-hub
description: "Genies framework unified skill hub. Use when you need to find the right skill for any Genies framework task, including authentication, authorization, caching, configuration, database, DDD microservices, Dapr messaging, macros, K8s deployment, testing, API conventions, gateway proxy, or Salvo web framework features. Also use when the user asks about Genies framework capabilities, asks which skill to use, or wants a quick overview of available Genies skills."
---

# Genies 框架统一 Skill 入口

## 概述

本文件是 Genies 框架的 **统一 Skill 入口**，所有 Skill 按功能域分类，按需加载。当你不确定该使用哪个 Skill 时，从此处查找即可。

---

## Genies 框架核心 Skill

### 认证与权限

| Skill | 说明 | 加载场景 |
|-------|------|----------|
| `auth-usage` | genies_auth 权限与认证模块 | Casbin RBAC、JWT中间件、`#[casbin]`宏、字段级权限过滤、事件驱动同步 |
| `auth-admin-usage` | 统一认证管理后台 | 用户/角色/权限/部门/应用CRUD、OAuth 2.0、JWT认证、Dapr事件发布 |

### 数据存储与迁移

| Skill | 说明 | 加载场景 |
|-------|------|----------|
| `rbatis-usage` | RBatis v4 ORM框架 | 动态SQL、py_sql/html_sql宏、事务、拦截器、多数据库驱动工厂 |
| `flyway-usage` | flyway-rs 数据库迁移 | 运行时SQL迁移、多数据库支持、MigrationRunner |
| `context-usage` | 应用上下文管理 | 全局CONTEXT单例、多数据库连接池、缓存服务、JWT中间件集成 |

### 缓存

| Skill | 说明 | 加载场景 |
|-------|------|----------|
| `cache-usage` | 双后端缓存服务 | Redis/Memory双后端、ICacheService trait、TTL、原子操作 |

### 配置管理

| Skill | 说明 | 加载场景 |
|-------|------|----------|
| `config-usage` | 配置管理 | YAML配置加载、`#[derive(Config)]`宏、ApplicationConfig |

### 微服务与消息

| Skill | 说明 | 加载场景 |
|-------|------|----------|
| `dapr-usage` | Dapr集成 | pub/sub消息、CloudEvent解析、`#[topic]`宏、自动topic订阅 |
| `ddd-usage` | DDD基础库 | 聚合根模式、领域事件、Outbox模式、事件发布 |

### 响应与错误处理

| Skill | 说明 | 加载场景 |
|-------|------|----------|
| `core-usage` | 响应模型/错误处理/JWT | RespVO/ResultDTO双响应模型、Salvo Writer集成、Keycloak JWT验证 |

### 宏与代码生成

| Skill | 说明 | 加载场景 |
|-------|------|----------|
| `derive-usage` | 过程宏集合 | Aggregate、DomainEvent、Config、ConfigCore、topic、remote、casbin宏 |

### K8s部署

| Skill | 说明 | 加载场景 |
|-------|------|----------|
| `k8s-usage` | K8s健康探针 | liveness/readiness端点、SERVICE_STATUS全局状态 |

### 测试

| Skill | 说明 | 加载场景 |
|-------|------|----------|
| `test-usage` | 对比测试基础设施 | Java/Rust API对比测试、数据库快照/diff/restore、Deep JSON Diff |

### 框架总览与工程规范

| Skill | 说明 | 加载场景 |
|-------|------|----------|
| `genies-usage` | 框架整体使用指南 | 框架全貌、DDD+Dapr集成、各crate协同、完整配置 |
| `genies-ddd-microservice` | DDD微服务开发指南 | Java DDD四层→Rust映射、聚合根设计、项目结构、迁移指南 |
| `api-conventions` | 前后端接口规范 | 字段命名、日期格式、响应模型、分页、错误处理、ID策略 |
| `gateway-dev-proxy` | Gateway/Proxy开发代理 | nginx反向代理、servlet_path路由分发、本地开发环境配置 |

---

## Salvo Web 框架 Skill

### Web 基础

| Skill | 说明 | 加载场景 |
|-------|------|----------|
| `salvo-basic-app` | 基础应用 | Handler/Router/Server 创建 |
| `salvo-routing` | 路由配置 | 路径参数、嵌套路由、过滤器 |
| `salvo-path-syntax` | 路径参数语法 | `{}` vs 已弃用的 `<>` 语法 |
| `salvo-middleware` | 中间件 | Hoop/FlowCtrl 实现 |
| `salvo-error-handling` | 错误处理 | 自定义错误类型、状态码 |

### 数据与文件

| Skill | 说明 | 加载场景 |
|-------|------|----------|
| `salvo-data-extraction` | 数据提取 | JSON/Form/Query/Path 参数提取 |
| `salvo-database` | 数据库集成 | SQLx/Diesel/SeaORM |
| `salvo-static-files` | 静态文件 | CSS/JS/图片/下载文件服务 |
| `salvo-file-handling` | 文件处理 | 上传/下载/Multipart |

### 安全

| Skill | 说明 | 加载场景 |
|-------|------|----------|
| `salvo-auth` | 认证授权 | JWT/Basic Auth |
| `salvo-cors` | CORS | 跨域配置 |
| `salvo-csrf` | CSRF防护 | Cookie/Session CSRF |
| `salvo-rate-limiter` | 限流 | API限流/DDoS防护 |
| `salvo-session` | 会话管理 | Session 状态持久化 |
| `salvo-tls-acme` | TLS/HTTPS | ACME自动证书管理 |

### 性能

| Skill | 说明 | 加载场景 |
|-------|------|----------|
| `salvo-compression` | 响应压缩 | gzip/brotli/zstd/deflate |
| `salvo-caching` | 缓存策略 | Cache-Control/ETag |
| `salvo-concurrency-limiter` | 并发限制 | 并发请求限制 |
| `salvo-timeout` | 请求超时 | 超时配置 |

### 运维

| Skill | 说明 | 加载场景 |
|-------|------|----------|
| `salvo-logging` | 日志/Tracing | 请求日志、可观测性 |
| `salvo-graceful-shutdown` | 优雅关闭 | 零停机部署 |

### 高级功能

| Skill | 说明 | 加载场景 |
|-------|------|----------|
| `salvo-proxy` | 反向代理 | 负载均衡/API网关 |
| `salvo-openapi` | OpenAPI文档 | `#[endpoint]`宏自动生成OpenAPI 3.0 |
| `salvo-testing` | 测试工具 | TestClient 测试 |
| `salvo-flash` | Flash消息 | 一次性通知 |

### 实时通信

| Skill | 说明 | 加载场景 |
|-------|------|----------|
| `salvo-realtime` | 实时通信概览 | WebSocket vs SSE 选型 |
| `salvo-websocket` | WebSocket | 双向通信 |
| `salvo-sse` | SSE | 服务端推送 |

---

## 按需加载指南

快速决策表——根据开发场景查找推荐 Skill：

| 开发场景 | 推荐加载的 Skill |
|----------|-----------------|
| 新建微服务 | `genies-ddd-microservice`, `genies-usage`, `config-usage`, `context-usage` |
| 添加认证/权限 | `auth-usage`, `auth-admin-usage`, `derive-usage` (casbin宏) |
| 数据库操作 | `rbatis-usage`, `flyway-usage`, `context-usage` |
| 添加缓存 | `cache-usage` |
| Dapr消息通信 | `dapr-usage`, `derive-usage` (topic宏) |
| API开发 | `core-usage`, `api-conventions`, `salvo-routing`, `salvo-data-extraction` |
| 前后端联调 | `api-conventions`, `gateway-dev-proxy` |
| 测试 | `test-usage`, `salvo-testing` |
| K8s部署 | `k8s-usage`, `config-usage` |
| 性能优化 | `cache-usage`, `salvo-compression`, `salvo-caching` |
| 实时通信 | `salvo-realtime`, `salvo-websocket`, `salvo-sse` |

---

## 辅助参考文件

以下 Skill 附带了额外的深度参考文档：

| Skill | 参考文件 | 说明 |
|-------|---------|------|
| `auth-usage` | `reference.md` (56KB) | 详细技术参考 |
| `genies-usage` | `reference.md` (30KB) | 完整配置项表 |
| `genies-ddd-microservice` | `migration-guide.md` (96KB) | Java→Rust 迁移指南 |
| `genies-ddd-microservice` | `reference.md` (16KB) | 快速 API 参考 |
| `rbatis-usage` | `genies-patterns.md` (5KB) | Genies 多数据库驱动工厂模式 |
