---
name: gateway-dev-proxy
description: Guide for understanding and configuring the gateway/proxy convention in Genies projects. Use when setting up local development environment with nginx reverse proxy, configuring debug/servlet_path/gateway for a microservice, implementing route prefix mounting in main.rs, debugging cross-service HTTP call routing issues, adding a new service to the nginx configuration, or understanding how production gateway forwarding maps to local development.
---

# Gateway 开发代理约定

## 1. 概述

生产环境中，所有 HTTP 流量经过统一的 nginx gateway，按 URL 一级目录（`servlet_path`）分发到不同微服务——**包括后端 API 和前端静态资源**。nginx 做"去前缀转发"——`/auth-admin/api/foo` → `http://backend:9099/api/foo`。

本地开发时，用同样的方式处理：

1. **本地启动 nginx**（`D:\tools\nginx`）模拟生产 gateway，转发到本地调试端口
2. **后端应用**设置 `debug: false`，路由不加前缀——nginx 统一负责前缀剥离，与生产行为完全一致
3. **前端应用**通过相对路径构建（`base: './'`）+ 动态探测 `servlet_path` 来适配二级目录

> **备选模式**：如果不想启动 nginx（如快速调试单个服务），可设 `debug: true`，此时应用在 Router 上自己挂载 `servlet_path` 前缀，通过 `http://localhost:<port>/<servlet_path>/...` 直接访问。但推荐始终使用 nginx，保证开发与生产一致。

```mermaid
flowchart LR
    subgraph 生产环境
        U1[用户] -->|http://gateway/auth-admin/api/foo| N1[nginx gateway]
        N1 -->|去前缀转发 /api/foo| B1[backend:9099<br/>debug=false]
    end

    subgraph 开发环境（推荐：有本地 nginx）
        U2[用户] -->|http://192.168.1.182/auth-admin/api/foo| N2[本地 nginx<br/>D:\tools\nginx]
        N2 -->|去前缀转发 /api/foo| B2[localhost:9099<br/>debug=false]
    end

    subgraph 开发环境（备选：无 nginx）
        U3[用户] -->|http://localhost:9099/auth-admin/api/foo| B3[localhost:9099<br/>debug=true<br/>Router 挂载前缀]
    end
```

**核心原则**：nginx 负责前缀剥离，后端只关心去掉前缀后的路由。如果本地有 nginx，`debug=false` 就是正确配置——开发与生产对后端而言完全相同。

---

## 2. 核心约定

以下是不可违背的规则：

### 规则 1：nginx location 路径 = servlet_path 值

两者必须一一对应。例如 nginx 中 `location /sickbed` 对应 `application.yml` 中 `servlet_path: "/sickbed"`。

### 规则 2：后端路由不加前缀，nginx 统一负责

后端始终将路由挂载在 `/` 下，不区分环境。nginx 负责 `servlet_path` 前缀的匹配与剥离，与生产行为完全一致。

```rust
// 后端：所有路由直接挂 / 下，无需环境判断
let app_router = Router::new()
    .push(k8s_health_check())
    .push(auth_full_router())
    .push(auth_public_router())
    .push(Router::with_path("/").push(business_router));
```

> 历史上存在 `debug` 字段控制 Router 是否自挂前缀的备选模式（不启动 nginx 时用）。当本地始终通过 nginx 访问时，`debug=false` 是唯一需要的配置，该分支逻辑可以逐步移除。

### 规则 3：nginx proxy_pass 末尾 `/` 是去前缀的关键

```nginx
# 正确：末尾带 /，做去前缀转发
# /auth-admin/api/foo → http://localhost:9099/api/foo
location /auth-admin {
    proxy_pass http://localhost:9099/;
}

# 错误：末尾无 /，保留完整路径
# /auth-admin/api/foo → http://localhost:9099/auth-admin/api/foo （会导致 404）
location /auth-admin {
    proxy_pass http://localhost:9099;
}
```

### 规则 4：跨微服务调用 base URL = gateway + servlet_path

通过 `config_gateway!` 宏展开，例如 `gateway: "http://127.0.0.1:6015"` + `servlet_path: "/auth-admin"` → `http://127.0.0.1:6015/auth-admin`。

### 规则 5：实例注册 base_url = gateway + servlet_path

服务向 auth-admin 注册时，`base_url` 字段由 `gateway + servlet_path` 拼接而成，与跨服务调用规则保持一致。

### 规则 6：前端通过相对路径 + 动态探测适配二级目录

前端应用采用以下两个机制，使其在直连后端和通过 nginx 代理两种方式下都能正确加载：

1. **构建时相对路径**（`vite.config.ts` 中 `base: './'`）：所有 JS/CSS 资源引用使用相对路径，无论 HTML 部署在 `/ui/` 还是 `/auth-admin/ui/` 都能正确加载
2. **运行时动态探测**（`utils/path.ts` 中 `getApiBaseUrl()`）：解析 `window.location.pathname`，正则匹配 `/xxx/ui/` 格式提取 `servlet_path` 作为 axios 的 `baseURL`

```
直连后端: http://localhost:9099/ui/  → getApiBaseUrl() → ""  (API: /login)
通过 nginx: http://gateway/auth-admin/ui/ → getApiBaseUrl() → "/auth-admin" (API: /auth-admin/login)
```

---

## 3. 关键配置字段

`ApplicationConfig` 中与 gateway/proxy 相关的字段（定义见 [crates/config/src/app_config.rs](crates/config/src/app_config.rs)）：

| 字段 | 类型 | 作用 | 示例值 |
|------|------|------|--------|
| `debug` | `bool` | ~~已废弃。~~ 始终设为 `false`，nginx 负责前缀，后端路由不区分环境 | `false` |
| `servlet_path` | `String` | 服务路由前缀，= nginx location 路径 | `"/auth-admin"`, `"/sickbed"` |
| `server_url` | `String` | 服务监听地址:端口 | `"0.0.0.0:9099"` |
| `gateway` | `Option<String>` | HTTP 网关地址。合法 URL →网关模式；否则→Dapr sidecar 模式 | `"http://127.0.0.1:6015"` |
| `auth_admin_url` | `String` | auth-admin 地址，用于实例注册和心跳 | `"http://localhost:9099"` |
| `heartbeat_interval` | `u64` | 心跳间隔（秒），默认 30 | `30` |

---

## 4. 本地 Nginx 配置

### 4.1 基本信息

| 项目 | 值 |
|------|-----|
| 配置文件 | `D:\tools\nginx\conf\nginx.conf` |
| 监听端口 | 80 |
| 访问地址 | `http://192.168.1.182/<servlet_path>/...` |

### 4.2 常用命令

在 `D:\tools\nginx` 目录下执行：

```powershell
# 启动
.\nginx.exe

# 重载配置（修改 nginx.conf 后必须执行）
.\nginx.exe -s reload

# 停止
.\nginx.exe -s stop

# 测试配置语法
.\nginx.exe -t
```

**重要**：修改 `nginx.conf` 后必须手动执行 `nginx.exe -s reload`，配置不会自动生效。

### 4.3 完整配置

当前 `D:\tools\nginx\conf\nginx.conf` 完整内容：

```nginx
#user  nobody;
worker_processes  1;

#error_log  logs/error.log;
#error_log  logs/error.log  notice;
#error_log  logs/error.log  info;

#pid        logs/nginx.pid;

events {
    worker_connections  1024;
}

http {
    include       mime.types;
    default_type  application/octet-stream;

    sendfile        on;
    keepalive_timeout  65;

    server {
        listen       80;
        server_name  localhost;

        # ============================================================
        # 本地开发微服务代理规则 (全部去前缀转发)
        # 启动: D:\tools\nginx\nginx.exe
        # 重载: D:\tools\nginx\nginx.exe -s reload
        # 停止: D:\tools\nginx\nginx.exe -s stop
        # 访问: http://192.168.1.182/<servlet_path>/...
        # ============================================================

        # auth-admin: 统一认证管理后台 (port=9099)
        # 精确匹配 /auth-admin/ui 重定向到 /auth-admin/ui/，保证前端相对路径资源加载正确
        location = /auth-admin/ui {
            return 301 /auth-admin/ui/;
        }
        location /auth-admin {
            proxy_pass http://localhost:9099/;
            proxy_set_header Host $host;
            proxy_set_header X-Real-IP $remote_addr;
            proxy_set_header X-Forwarded-For $proxy_add_x_forwarded_for;
            proxy_set_header X-Forwarded-Proto $scheme;
        }

        # tdssr: 提灯快反后台服务 tdssr-service (port=5800)
        location /tdssr {
            proxy_pass http://localhost:5800/;
            proxy_set_header Host $host;
            proxy_set_header X-Real-IP $remote_addr;
            proxy_set_header X-Forwarded-For $proxy_add_x_forwarded_for;
            proxy_set_header X-Forwarded-Proto $scheme;
        }

        # sickbed: 病房服务 (port=8083)
        location /sickbed {
            proxy_pass http://localhost:8083/;
            proxy_set_header Host $host;
            proxy_set_header X-Real-IP $remote_addr;
            proxy_set_header X-Forwarded-For $proxy_add_x_forwarded_for;
            proxy_set_header X-Forwarded-Proto $scheme;
        }

        # topic: Dapr Topic 测试 / 集成测试服务 (port=8050)
        location /topic {
            proxy_pass http://localhost:8050/;
            proxy_set_header Host $host;
            proxy_set_header X-Real-IP $remote_addr;
            proxy_set_header X-Forwarded-For $proxy_add_x_forwarded_for;
            proxy_set_header X-Forwarded-Proto $scheme;
        }

        # baseinfo: 基础信息服务 (port=8011)
        location /baseinfo {
            proxy_pass http://localhost:8011/;
            proxy_set_header Host $host;
            proxy_set_header X-Real-IP $remote_addr;
            proxy_set_header X-Forwarded-For $proxy_add_x_forwarded_for;
            proxy_set_header X-Forwarded-Proto $scheme;
        }

        # hub: 提灯中枢服务 hub-service (port=6800)
        location /hub {
            proxy_pass http://localhost:6800/;
            proxy_set_header Host $host;
            proxy_set_header X-Real-IP $remote_addr;
            proxy_set_header X-Forwarded-For $proxy_add_x_forwarded_for;
            proxy_set_header X-Forwarded-Proto $scheme;
        }

        # ----- 新增代理模板 (去前缀) -----
        # location /<servlet_path> {
        #     proxy_pass http://localhost:<port>/;
        #     proxy_set_header Host $host;
        #     proxy_set_header X-Real-IP $remote_addr;
        #     proxy_set_header X-Forwarded-For $proxy_add_x_forwarded_for;
        #     proxy_set_header X-Forwarded-Proto $scheme;
        # }

        # 默认: 静态文件
        location / {
            root   html;
            index  index.html index.htm;
        }

        error_page   500 502 503 504  /50x.html;
        location = /50x.html {
            root   html;
        }
    }
}
```

### 4.4 服务端口映射

| servlet_path | 本地端口 | 服务说明 |
|---|---|---|
| `/auth-admin` | 9099 | 统一认证管理后台 |
| `/tdssr` | 5800 | 提灯快反后台服务 |
| `/sickbed` | 8083 | 病房服务 |
| `/topic` | 8050 | Dapr Topic 测试 / 集成测试 |
| `/baseinfo` | 8011 | 基础信息服务 |
| `/hub` | 6800 | 提灯中枢服务 |

### 4.5 新增 location 步骤

1. 找到对应服务 `application.yml` 中的 `servlet_path` 和 `server_url` 端口
2. 在 nginx.conf 的 server 块中添加 location（参考模板注释）
3. 运行 `nginx.exe -t` 测试配置语法
4. 运行 `nginx.exe -s reload` 重载配置
5. 验证：`http://192.168.1.182/<servlet_path>/actuator/health/liveness`

### 4.6 代理到远程服务

有些服务不在本地运行（如遗留系统、外部依赖），可以配置 nginx 将其路径代理到远程生产网关。这是本地开发中常见的混合模式：部分服务跑本地，部分走远程。

**模板**：

```nginx
# /<path>: 远程服务 (本地未部署，代理到生产网关)
location /<path> {
    proxy_pass http://58.20.184.66:6015/;
    proxy_set_header Host $host;
    proxy_set_header X-Real-IP $remote_addr;
    proxy_set_header X-Forwarded-For $proxy_add_x_forwarded_for;
    proxy_set_header X-Forwarded-Proto $scheme;
}
```

**当前远程代理配置**：

| 路径 | 用途 | 来源 |
|------|------|------|
| `/user` | 用户服务 | rrt-web 调用 |
| `/nursingdoc` | 护理文档服务 | rrt-web 调用 |
| `/websocket` | WebSocket 连接 | rrt-web 调用 |
| `/sso` | 单点登录 | smartward-web 调用 |
| `/nisweb` | NIS Web 服务 | smartward-web 调用 |
| `/uaa` | 统一认证 (UAA) | smartward-web 调用 |
| `/salvo-openapi` | OpenAPI 文档 | smartward-web 调用 |

**注意事项**：
- 远程代理目标 `proxy_pass http://58.20.184.66:6015/` 末尾带 `/` 表示去前缀转发（与原路径保持一致）
- WebSocket 路径需要额外配置 `proxy_http_version 1.1` 和 Upgrade 头
- 如果该路径在远程也被作为 `servlet_path` 路由，转发行为与生产一致

---

## 5. application.yml 配置指南

### 5.1 各服务配置对照表

| 服务 | servlet_path | server_url 端口 | gateway | 说明 |
|------|-------------|----------------|---------|------|
| auth-admin | `/auth-admin` | 9099 | `http://127.0.0.1:6015` | 统一认证管理后台，自注册心跳 |
| sickbed | `/sickbed` | 8083 | `http://localhost` | 使用 `try_register_and_heartbeat` |
| tdssr | `/tdssr` | 5800 | — | 提灯快反后台 |
| topic | `/topic` | 8050 | `http://58.20.184.66:6002` | Dapr Topic 测试 |
| baseinfo | `/baseinfo` | 8011 | `http://58.20.184.66:6015` | 基础信息服务 |
| cdc | （未配置） | 8010 | （未配置） | 不走 nginx 代理 |
| hub-service | `/hub` | 6800 | — | 提灯中枢服务 |

### 5.2 配置一致性检查规则

- `servlet_path` 必须与 nginx.conf 中的 location 路径一致
- `server_url` 端口必须与 nginx.conf 中 `proxy_pass` 的端口一致
- `gateway` 应设为 `http://127.0.0.1:6015` 或 `http://localhost`

---

## 6. main.rs 路由挂载模式

后端路由始终挂载在 `/` 下，不区分环境。nginx 负责 `servlet_path` 前缀剥离。

### 6.1 核心模式

```rust
let app_router = Router::new()
    .push(k8s_health_check())           // 健康检查
    .push(genies::dapr_event_router())  // Dapr 事件路由
    .push(auth_full_router())
    .push(auth_public_router())
    .push(
        Router::with_path("/")
            .push(business_router),     // 业务路由
    );

// OpenAPI / SwaggerUI
let app_router = app_router
    .unshift(doc.into_router("/api-doc/openapi.json"))
    .unshift(
        SwaggerUi::new("/api-doc/openapi.json")
            .into_router("/swagger-ui"),
    );
```

### 6.2 关键注意事项

- **k8s 健康检查** 和 **Dapr 事件路由** 始终挂根路径，不经过 nginx（通过 K8s / Dapr sidecar 直连）
- **OpenAPI / SwaggerUI** 路径直接写 `/api-doc/...`、`/swagger-ui`，由 nginx 加前缀后访问
- 不再需要 `debug` 分支判断——nginx 统一处理前缀，后端路由逻辑与环境无关

---

## 7. 前端路由与二级目录

前端应用和后端 API 一样，通过 nginx gateway 的二级目录进行区分访问。以 auth-admin 前端为例。

### 7.1 技术方案

| 特性 | 方案 | 说明 |
|------|------|------|
| 路由模式 | Hash 路由 (`createWebHashHistory`) | `#` 之后的内容不发送到服务端，无需 nginx `try_files` |
| 构建路径 | 相对路径 (`vite.config.ts` 中 `base: './'`) | JS/CSS 资源使用 `./assets/xxx.js`，不依赖部署前缀 |
| API 前缀 | 运行时动态探测 (`getApiBaseUrl()`) | 解析当前 URL 自动提取 `servlet_path` |
| 静态资源 | `rust-embed` 编译嵌入 | 前端构建产物直接嵌入 Rust 二进制，无需外部文件服务 |

### 7.2 vite.config.ts 构建配置

文件位置：[crates/auth-admin/web/vite.config.ts](crates/auth-admin/web/vite.config.ts)

```typescript
export default defineConfig({
  plugins: [vue()],
  base: './',              // 关键：相对路径，适配任意部署前缀
  build: {
    outDir: '../static',   // 输出到 crates/auth-admin/static/
    emptyOutDir: true
  }
})
```

**为什么用 `'./'` 而非 `'/auth-admin/'`**：
- 绝对路径 `/auth-admin/` 在直连后端 (`http://localhost:9099/ui/`) 时会找不到资源
- 相对路径让两种访问方式都能正确加载：直连和通过 nginx

### 7.3 动态探测 servlet_path

文件位置：[crates/auth-admin/web/src/utils/path.ts](crates/auth-admin/web/src/utils/path.ts)

```typescript
export function getApiBaseUrl(): string {
  const path = window.location.pathname
  // 匹配 /xxx/ui/ 格式，提取 /xxx 作为 API 前缀
  const match = path.match(/^(\/[^/]+)\/ui/)
  return match ? match[1] : ''
}
```

**行为示例**：

| 访问方式 | window.location.pathname | getApiBaseUrl() 返回 | axios 请求路径 |
|---------|-------------------------|---------------------|---------------|
| 直连后端 | `/ui/` | `""` | `POST /login` |
| 通过 nginx | `/auth-admin/ui/` | `"/auth-admin"` | `POST /auth-admin/login` |

此返回值在 `api/index.ts` 中设置为 axios 的 `baseURL`：

```typescript
const api = axios.create({
  baseURL: getApiBaseUrl(),
  timeout: 30000,
})
```

### 7.4 Nginx 中 SPA 的 trailing-slash 问题

对于 SPA 前端，`/auth-admin/ui`（不带尾部斜杠）是一个特殊问题：

```
用户访问: /auth-admin/ui (无斜杠)
  → index.html 中引用了 ./assets/index.js
  → 浏览器解析基路径为 /auth-admin/ui
  → 请求 /auth-admin/assets/index.js  ← 错误！应该是 /auth-admin/ui/assets/index.js
```

解决方案：在 nginx 中添加精确匹配重定向：

```nginx
# 精确匹配 /auth-admin/ui（无斜杠）→ 301 重定向到 /auth-admin/ui/
location = /auth-admin/ui {
    return 301 /auth-admin/ui/;
}
```

这样浏览器始终以 `/auth-admin/ui/` 为基路径，相对路径资源加载正确。

### 7.5 后端静态资源服务

文件位置：[crates/auth-admin/src/interfaces/admin_ui.rs](crates/auth-admin/src/interfaces/admin_ui.rs)

```rust
#[derive(Embed)]
#[folder = "static/"]
struct AdminUiAssets;

pub fn auth_admin_ui_router() -> Router {
    Router::with_path("ui")
        .get(serve_admin_ui_entry)           // /ui 或 /ui/
        .push(Router::with_path("{**path}").get(serve_admin_ui))  // /ui/assets/...
}
```

**关键设计**：
- 路由挂载在 `ui` 上（不依赖 servlet_path），与 nginx 配合：
  - 直连: `http://localhost:9099/ui/` → Router 的 `ui` 路径匹配
  - nginx: `http://gateway/auth-admin/ui/` → nginx 去前缀 → 后端收到 `/ui/` → Router 匹配
- 使用 `rust-embed` 将前端编译进二进制，无需外部文件
- SPA fallback：未匹配路径返回 `index.html`
- **不做 trailing-slash 重定向**：注释明确说明"避免 nginx 反代场景丢失前缀"，由 nginx 处理

### 7.6 前端访问方式对比

| 方式 | 访问地址 | API 前缀 | 适用场景 |
|------|---------|---------|---------|
| **通过 nginx** | `http://192.168.1.182/auth-admin/ui/` | `"/auth-admin"` | 完整链路验证，与生产一致 |
| **直连后端** | `http://localhost:9099/ui/` | `""` | 快速后端调试 |

### 7.7 开发流程

1. 修改前端代码 → `npm run build`（产物输出到 `../static/`）
2. 重启 Rust 后端（需重新编译嵌入的 `static/` 资源）
3. 通过 nginx 访问 `http://192.168.1.182/auth-admin/ui/` 验证完整链路

> 当前 vite.config.ts 未配置 `server.proxy` 指向后端，所以 `npm run dev` 的独立 dev server（`localhost:5173`）主要用于前端组件开发，API 调用需额外配置 proxy 或通过 nginx 访问。

---

## 8. 跨微服务调用

### 8.1 config_gateway! 宏

定义在 [crates/genies/src/lib.rs:93-118](crates/genies/src/lib.rs)，用于生成 `feignhttp` 的 gateway URL。

```rust
// 在 remote 模块中声明目标服务的基础 URL
pub static Sickbed: Lazy<String> = genies::config_gateway!("/sickbed");
```

**展开逻辑**：

| gateway 字段值 | 展开结果 |
|---------------|---------|
| `"http://127.0.0.1:6015"` | `http://127.0.0.1:6015/sickbed` |
| `"http://localhost"` | `http://localhost/sickbed` |
| `""` 或 非 HTTP 值 | `http://localhost:3500/v1.0/invoke/sickbed-service/method` (Dapr sidecar) |

### 8.2 在 #[remote] 中使用

```rust
use genies::config_gateway;

pub static Sickbed: Lazy<String> = config_gateway!("/sickbed");

#[remote]
#[get(url = Sickbed, path = "/api/wards")]
pub async fn list_wards() -> feignhttp::Result<WardListVO> { impled!() }
```

### 8.3 实际请求 URL 链路

- **开发环境**（gateway=`http://localhost`）：`http://localhost/sickbed/api/wards` → 本地 nginx → `http://localhost:8083/api/wards`
- **Dapr 模式**（gateway 为空）：`http://localhost:3500/v1.0/invoke/sickbed-service/method/api/wards` → Dapr sidecar → sickbed 服务

---

## 9. 服务实例注册

实现位于 [crates/auth/src/service_registry.rs](crates/auth/src/service_registry.rs)。

### 9.1 base_url 构建逻辑

```rust
base_url: {
    // 优先使用 gateway 字段（非空时）
    let base = match config.gateway.as_deref() {
        Some(gw) if !gw.is_empty() => gw.trim_end_matches('/').to_string(),
        _ => format!("http://{}", config.server_url),  // 回退到 server_url
    };
    let path = config.servlet_path.as_str();
    // 确保 path 以 / 开头
    let path = if path.starts_with('/') { path.to_string() } else { format!("/{}", path) };
    format!("{}{}", base, path)
    // 例: http://127.0.0.1:6015 + /auth-admin → http://127.0.0.1:6015/auth-admin
}
```

### 9.2 注册调用

```rust
// 向 auth-admin 注册实例并启动心跳
let _registry_guard = genies_auth::try_register_and_heartbeat(&CONTEXT.config).await;
```

`try_register_and_heartbeat` 会在后台启动心跳协程，服务退出时自动向 auth-admin 注销。

### 9.3 开发环境注意事项

- `gateway` 必须设置正确，否则注册的 `base_url` 可能变成 `http://0.0.0.0:8083`（不可访问）
- `auth_admin_url` 也需要设置（通常指向 `http://localhost:9099`）
- auth-admin 本身不自注册心跳，而是自己实现

---

## 10. 新增服务检查清单

新增一个微服务时，按以下步骤确保 gateway/proxy 配置正确：

1. **确定 servlet_path** — 选择一个不与现有服务冲突的路径前缀（如 `/my-service`）
2. **配置 application.yml** — 设置 `servlet_path`、`server_url`、`gateway`（`debug` 固定 `false`）
3. **修改 main.rs** — 路由挂载在 `/` 下，无需环境判断（参考第 6 章）
4. **配置本地 nginx** — 在 `D:\tools\nginx\conf\nginx.conf` 中添加对应 location
5. **（如有前端）配置 vite.config.ts** — 确保 `base: './'`，使用相对路径
6. **（如有前端）处理 trailing-slash** — 在 nginx 中添加 `location = <servlet_path>/ui { return 301 .../ui/; }`
7. **测试 nginx 配置** — 运行 `nginx.exe -t`
8. **重载 nginx** — 运行 `nginx.exe -s reload`
9. **验证后端连通性** — 访问 `http://192.168.1.182/<servlet_path>/actuator/health/liveness`
10. **（如有前端）验证前端加载** — 访问 `http://192.168.1.182/<servlet_path>/ui/`，确认页面和 API 调用正常
11. **确认跨服务调用** — 检查其他服务中是否有通过 `config_gateway!` 引用新服务的 `servlet_path`
12. **确认实例注册** — 检查 auth-admin 管理界面中是否出现了新实例及其 base_url
13. **（如依赖外部服务）添加远程代理** — 将本地未部署的服务路径代理到生产网关（参考 4.6）

---

## 11. 故障排查

| 问题 | 可能原因 | 排查步骤 |
|------|---------|---------|
| 404 Not Found | nginx `proxy_pass` 末尾缺少 `/` | 检查 `proxy_pass http://backend/;` 末尾是否有 `/` |
| 404 Not Found | `servlet_path` 与 nginx location 不一致 | 对比 `application.yml` 和 `nginx.conf` |
| 连接被拒绝 (Connection Refused) | 后端服务未启动或端口不对 | `netstat -ano \| findstr <port>` 检查端口 |
| 跨服务调用 404 | `gateway` 配置不正确 | 检查 gateway 值能否访问到本地 nginx |
| 实例注册的 base_url 错误 | `gateway` 为空或格式不对 | 检查 auth-admin 管理界面中实例的 `base_url` 字段 |
| SwaggerUI 加载失败 | OpenAPI 路由前缀配置错误 | 检查 `doc.into_router(prefix + "/api-doc/openapi.json")` 的 prefix 值 |
| nginx 启动失败 / 端口占用 | 80 端口被其他程序占用 | `netstat -ano \| findstr :80`，停用冲突程序后重试 |
| 修改 nginx.conf 后未生效 | 未执行 reload | 运行 `nginx.exe -s reload` |
| 前端页面空白 / JS 资源 404 | vite `base` 不是 `'./'` | 检查 `vite.config.ts` 中 `base: './'` |
| 前端 API 调用 404 | `getApiBaseUrl()` 探测失败 | 确认访问路径格式为 `/<servlet_path>/ui/`，含尾部斜杠 |
| 前端通过 nginx 访问后登录跳转失败 | 缺少 `location = /xxx/ui` 精确匹配重定向 | 添加 `return 301 /xxx/ui/;` 规则 |
| 远程代理 502/504 | 远程服务器不可达或路径不存在 | 确认目标服务器 `58.20.184.66:6015` 可达，路径在远程存在 |

---

## 12. Related Skills

- **[config-usage](.qoder/skills/config-usage/SKILL.md)** — `ApplicationConfig` 字段详解、YAML 配置加载、环境变量覆盖
- **[genies-usage](.qoder/skills/genies-usage/SKILL.md)** — `config_gateway!` 宏、`#[remote]` 远程服务调用
- **[genies-ddd-microservice](.qoder/skills/genies-ddd-microservice/SKILL.md)** — 微服务项目结构、main.rs 启动模板
- **[auth-admin-usage](.qoder/skills/auth-admin-usage/SKILL.md)** — 实例注册/心跳机制、服务管理界面
- **[dapr-usage](.qoder/skills/dapr-usage/SKILL.md)** — Dapr sidecar 模式下的服务调用（当 `gateway` 不使用 HTTP 时的回退方式）
