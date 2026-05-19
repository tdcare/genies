# auth-admin 认证安全功能增强

## Context

当前 auth-admin 仅支持"用户名+密码"登录，缺少以下企业级安全特性：

- **双因素认证 (2FA)**：登录后无第二验证因子
- **登录验证码 (CAPTCHA)**：登录接口无防暴力破解保护
- **密码强度策略**：密码无复杂度校验（如最小长度、字符类型要求）

需要将这些功能设计为**可选开关**，管理员可通过 Web 页面动态启用/配置，无需重启服务。

## Architecture Overview

```
┌─────────────────────────────────────────────────────────────┐
│                    用户登录流程 (改造后)                      │
├─────────────────────────────────────────────────────────────┤
│                                                              │
│  前端                          后端                          │
│   │                             │                            │
│   │  GET /captcha               │                            │
│   │◄── {id, image_base64} ──────│  (如果 captcha.enabled)    │
│   │                             │                            │
│   │  POST /login                │                            │
│   │  {user, pwd, captcha}       │                            │
│   │              ┌──────────────▼──────────────────┐        │
│   │              │ 1. 验证 CAPTCHA (如开启)         │        │
│   │              │ 2. 验证密码 + 状态检查            │        │
│   │              │ 3. 检查密码强度策略               │        │
│   │              │ 4. 检查用户是否启用 2FA          │        │
│   │              │    ├── 无 2FA → 直接颁发 JWT     │        │
│   │              │    └── 有 2FA → 颁发 preauth_token│       │
│   │              └──────────────┬──────────────────┘        │
│   │                             │                            │
│   │  ◄── {require_2fa: true,   │                            │
│   │       preauth_token,        │                            │
│   │       methods: ["totp"]}    │                            │
│   │                             │                            │
│   │  POST /2fa/verify           │                            │
│   │  {preauth_token, code}      │                            │
│   │              ┌──────────────▼──────────────────┐        │
│   │              │ 验证 TOTP/SMS/Second Password    │        │
│   │              │ 颁发完整 JWT                      │        │
│   │              └──────────────┬──────────────────┘        │
│   │                             │                            │
│   │  ◄── {access_token} ────────│                            │
└─────────────────────────────────────────────────────────────┘
```

## Implementation Plan

### 阶段 1：设置系统 (Settings)

> 所有功能的基础依赖，提供数据库持久化的运行时配置开关。

#### 1.1 数据库迁移 — V11

**新建:** `crates/auth-admin/migrations/V11__create_auth_admin_settings.sql`

```sql
CREATE TABLE IF NOT EXISTS auth_admin_settings (
    id BIGINT AUTO_INCREMENT PRIMARY KEY,
    setting_key VARCHAR(128) NOT NULL UNIQUE,
    setting_value JSON NOT NULL,
    description VARCHAR(256) DEFAULT '',
    created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP ON UPDATE CURRENT_TIMESTAMP
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4;

-- 默认值：所有新功能默认关闭
INSERT INTO auth_admin_settings (setting_key, setting_value, description) VALUES
('auth.2fa', '{"enabled":false,"methods":[]}', '双因素认证配置'),
('auth.captcha', '{"enabled":false}', '登录验证码配置'),
('auth.password', '{"min_length":6,"require_uppercase":false,"require_lowercase":false,"require_digit":false,"require_special":false}', '密码强度策略');
```

#### 1.2 领域层

| 文件 | 内容 |
|------|------|
| `domain/entity/settings_entity.rs` | `AdminSetting` 实体 (id, setting_key, setting_value, description...) — 使用 `crud!` 宏 |
| `domain/repository/settings_repository.rs` | `find_by_key()`、`upsert_setting()`、`list_all()` — 用 `#[py_sql]` 宏 |
| `domain/service/settings_service.rs` | `SettingsDomainService`：缓存穿透读取 (Redis/memory) + 写入自动失效 |

#### 1.3 应用层 & 接口层

| 文件 | 内容 |
|------|------|
| `application/settings_service.rs` | `SettingsAppService`: `get_password_policy()` / `get_captcha_settings()` / `get_2fa_settings()` 和对应的 `update_*()` 方法 |
| `interfaces/handler/settings_handler.rs` | HTTP 端点：`GET /settings`、`PUT /settings/auth.password`、`PUT /settings/auth.captcha`、`PUT /settings/auth.2fa` |

#### 1.4 路由注册

修改 `interfaces/router.rs` 和 `interfaces/handler/mod.rs`，将 settings_handler 加入 `protected_routes()`。

---

### 阶段 2：密码强度策略

> 依赖阶段 1，使用 Settings 中的 `auth.password.*` 配置。

#### 2.1 密码验证器

**新建:** `domain/service/password_policy_service.rs`

`PasswordPolicyService::validate(password, policy) -> Result<(), Vec<String>>`

检查规则（每项独立返回中文提示）：
1. 最小长度不足 → "密码长度不能少于{N}位"
2. 缺少大写字母 → "密码需包含至少一个大写字母"
3. 缺少小写字母 → "密码需包含至少一个小写字母"
4. 缺少数字 → "密码需包含至少一个数字"
5. 缺少特殊字符 → "密码需包含至少一个特殊字符(!@#$%^&*)"

#### 2.2 集成点

修改 `application/service.rs`：

- `UserAppService::create()` — 创建用户时校验
- `AuthService::change_password()` — 修改密码时校验
- `UserAppService::reset_password()` — 重置密码时校验

模式：先获取 `SettingsAppService::get_password_policy()`，若 `min_length > 0` 则调用 `PasswordPolicyService::validate()`。

---

### 阶段 3：验证码 (CAPTCHA)

> 依赖阶段 1，后端生成扭曲字符图片。

#### 3.1 Cargo 依赖

在 `Cargo.toml` 添加：
```toml
captcha = "0.9"
base64 = "0.22"
uuid = { version = "1", features = ["v4"] }
```

#### 3.2 验证码服务

**新建:** `domain/service/captcha_service.rs`

- `generate() -> (captcha_id: String, image_base64: String)` — 生成 4 位随机字母数字图片，文本存入缓存 `captcha:{id}`，TTL 300 秒
- `verify(captcha_id, text) -> Result<(), String>` — 一次一用，验证后立即删除

#### 3.3 接口

在 `interfaces/handler/auth_handler.rs` 的 `public_routes()` 添加：
- `GET /captcha` — 检查设置 → 生成验证码 → 返回 `{captcha_id, image_base64}`

#### 3.4 登录集成

修改 `application/dto.rs` — `LoginRequest` 增加：
```rust
pub captcha_id: Option<String>,
pub captcha_text: Option<String>,
```

修改 `AuthService::login()` — 在密码校验之前：
1. 调用 `SettingsAppService::get_captcha_settings()`
2. 若启用，验证 captcha_id + captcha_text，失败返回"验证码错误"

---

### 阶段 4：双因素认证 (2FA)

> 支持三种方式：TOTP 验证器、短信验证码、二次密码

#### 4.1 数据库迁移 — V12

**新建:** `crates/auth-admin/migrations/V12__create_auth_admin_user_2fa.sql`

```sql
CREATE TABLE IF NOT EXISTS auth_admin_user_2fa (
    id BIGINT AUTO_INCREMENT PRIMARY KEY,
    user_id BIGINT NOT NULL UNIQUE,
    method VARCHAR(32) NOT NULL COMMENT 'totp / sms / second_password',
    enabled TINYINT NOT NULL DEFAULT 0,
    secret VARCHAR(256) DEFAULT '' COMMENT '加密的TOTP密钥/bcrypt二次密码',
    phone VARCHAR(32) DEFAULT '' COMMENT 'SMS手机号',
    backup_codes TEXT COMMENT '备用恢复码(JSON数组, bcrypt哈希)',
    created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP ON UPDATE CURRENT_TIMESTAMP,
    INDEX idx_user_id (user_id)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4;
```

#### 4.2 Cargo 依赖

```toml
totp-rs = { version = "5", features = ["gen_secret", "otpauth"] }
aes-gcm = "0.10"
```

#### 4.3 加密工具

**新建:** `infrastructure/crypto.rs`

`CryptoUtil::encrypt/decrypt` — AES-256-GCM，密钥从 `ApplicationConfig` 新增字段 `2fa_encryption_key` 读取。

在 `config/src/app_config.rs` 添加：
```rust
#[serde(default)]
pub two_fa_encryption_key: String,  // 32字节 hex，不配置则自动生成
```

#### 4.4 领域层

| 文件 | 内容 |
|------|------|
| `domain/entity/user_2fa_entity.rs` | `UserTwoFactor` 实体，`crud!` 宏映射 `auth_admin_user_2fa` |
| `domain/service/totp_service.rs` | `generate_secret()` / `verify_code()` / `generate_backup_codes()` — 使用 `totp-rs` |
| `domain/service/sms_service.rs` | `send_code(phone)` / `verify_code(phone, code)` — 定义 `SmsGateway` trait + `LogOnlySmsGateway` 开发实现 |
| `domain/service/second_password_service.rs` | `set_password()` / `verify_password()` — bcrypt 哈希 |

#### 4.5 应用层

**新建:** `application/two_factor_service.rs`

`TwoFactorAppService`:
- `get_my_2fa_status(user_id)` — 当前 2FA 状态
- `setup_totp(user_id)` → 返回 secret + QR URL
- `confirm_totp(user_id, code)` → 验证激活码，标记启用，生成备用码
- `disable_2fa(user_id)` — 关闭 2FA
- `setup_second_password(user_id, password)` — 设置二次密码
- `get_backup_codes(user_id)` / `regenerate_backup_codes(user_id)`

#### 4.6 接口层

**新建:** `interfaces/handler/two_factor_handler.rs`

用户自服务 (Protected)：
- `GET /me/2fa` — 获取 2FA 状态
- `POST /me/2fa/totp/setup` — 发起 TOTP 绑定
- `POST /me/2fa/totp/confirm` — 确认 TOTP
- `POST /me/2fa/second-password` — 设置二次密码
- `DELETE /me/2fa` — 关闭 2FA

管理员 (Protected)：
- `POST /admin/users/{id}/2fa/reset` — 强制重置某用户的 2FA

#### 4.7 登录流程改造 (核心)

**新增 DTO** (在 `application/dto.rs`)：
```rust
pub struct PreAuthClaims { pub uid: i64, pub iat: usize, pub exp: usize, pub purpose: String }
pub struct TwoFactorVerifyRequest { pub preauth_token: String, pub code: String, pub method: String }
```

**修改 `LoginResponse`**：添加 `require_2fa: bool`、`preauth_token: Option<String>`、`available_methods: Vec<String>`

**修改 `AuthService::login()`**：
```
现有流程 (验证密码 + 状态) 
    ↓
5. 查询 UserTwoFactor (用户是否启用 2FA) + Settings (全局是否开启)
6. 若需要 2FA:
   → 颁发 PreAuthClaims JWT (5分钟有效期, purpose = "2fa_preauth")
   → 返回 { require_2fa: true, preauth_token, available_methods }
7. 若不需要 2FA:
   → 颁发完整 JWT (原逻辑不变)
```

**新增 `AuthService::verify_2fa(preauth_token, code, method)`**：
1. 解码验证 PreAuthToken
2. 根据 method 调用对应服务验证
3. 支持备用恢复码 (TOTP only)
4. 颁发完整 JWT

**在 `auth_handler.rs` 的 `public_routes()` 添加**：
- `POST /2fa/verify` — 公开端点（仅需有效的 preauth_token）

---

### 阶段 5：前端改造

#### 5.1 登录页 (`views/Login.vue`)

改造点：
1. 页面加载时调用 `GET /captcha`，若返回图片则显示验证码区域（图片 + 输入框 + 刷新按钮）
2. 提交 `/login` 后判断响应中 `require_2fa` 字段
3. 若 `require_2fa: true`：显示 2FA 验证码输入界面，用户输入后调 `/2fa/verify`
4. 2FA 验证成功后接收 JWT，正常跳转

#### 5.2 系统设置页 (新建 `views/Settings.vue`)

三个 Tab 页：
- **验证码设置**：启用/禁用开关
- **密码策略**：5 个设置项的表单（最小长度 + 4 个复杂度开关）
- **双因素认证**：启用/禁用开关，允许的方式多选

路由：`/settings`，需在侧边栏添加菜单项，设置 `requireAuth: true`。

#### 5.3 个人中心 (`views/Profile.vue`)

新增 "双因素认证" 区域：
- 显示当前状态
- TOTP 设置流程（显示 QR 码 → 输入验证码确认）
- 二次密码设置
- 关闭 2FA 按钮

#### 5.4 API 层 (`api/index.ts`)

新增函数：
- `getCaptcha()` → `GET /captcha`
- `verify2FA(preauthToken, code, method)` → `POST /2fa/verify`
- `get2FAStatus()` → `GET /me/2fa`
- `setupTOTP()` → `POST /me/2fa/totp/setup`
- `confirmTOTP(code)` → `POST /me/2fa/totp/confirm`
- `setupSecondPassword(pwd)` → `POST /me/2fa/second-password`
- `disable2FA()` → `DELETE /me/2fa`
- `getSettings()` → `GET /settings`
- `updateSetting(key, value)` → `PUT /settings/{key}`

---

## 文件变更清单

### 新建文件

| 文件 | 说明 |
|------|------|
| `migrations/V11__create_auth_admin_settings.sql` | 设置表 |
| `migrations/V12__create_auth_admin_user_2fa.sql` | 2FA 表 |
| `domain/entity/settings_entity.rs` | 设置实体 |
| `domain/entity/user_2fa_entity.rs` | 2FA 实体 |
| `domain/repository/settings_repository.rs` | 设置仓储 |
| `domain/service/settings_service.rs` | 设置缓存服务 |
| `domain/service/password_policy_service.rs` | 密码策略验证 |
| `domain/service/captcha_service.rs` | 验证码服务 |
| `domain/service/totp_service.rs` | TOTP 服务 |
| `domain/service/sms_service.rs` | SMS 服务 |
| `domain/service/second_password_service.rs` | 二次密码服务 |
| `application/settings_service.rs` | 设置应用服务 |
| `application/two_factor_service.rs` | 2FA 应用服务 |
| `interfaces/handler/settings_handler.rs` | 设置接口 |
| `interfaces/handler/two_factor_handler.rs` | 2FA 接口 |
| `infrastructure/crypto.rs` | 加密工具 |
| `web/src/views/Settings.vue` | 系统设置页 |

### 修改文件

| 文件 | 变更 |
|------|------|
| `Cargo.toml` | 添加 `captcha`, `base64`, `uuid`, `totp-rs`, `aes-gcm` |
| `config/src/app_config.rs` | 添加 `two_fa_encryption_key` 字段 |
| `domain/entity/mod.rs` | 注册新实体 |
| `domain/repository/mod.rs` | 注册新仓储 |
| `domain/service/mod.rs` | 注册新服务 |
| `application/service.rs` | login 改造 + 密码策略集成 |
| `application/dto.rs` | 新增 PreAuthClaims、LoginResponse 扩展等 |
| `interfaces/handler/mod.rs` | 注册新 handler |
| `interfaces/handler/auth_handler.rs` | 添加 captcha/2fa_verify 端点 |
| `interfaces/router.rs` | 注册新路由 |
| `infrastructure/mod.rs` | 注册 crypto 模块 |
| `application.yml` | 添加 `two_fa_encryption_key` 配置项 |
| `web/src/api/index.ts` | 添加所有新 API 函数 |
| `web/src/router/index.ts` | 添加 settings 路由 |
| `web/src/views/Login.vue` | 验证码 + 2FA 流程 |
| `web/src/views/Profile.vue` | 2FA 设置区域 |
| `web/src/AppLayout.vue` | 侧边栏添加设置菜单项 |

---

## 实现顺序

```
阶段1: 设置系统          (无依赖, 先做)
 ├── V11 migration
 ├── Entity → Repository → DomainService → AppService → Handler
 └── 路由注册 + mod.rs 更新

阶段2: 密码强度策略       (依赖 阶段1)
 ├── PasswordPolicyService
 └── 集成到 create/change_password/reset_password

阶段3: 验证码             (依赖 阶段1)
 ├── Cargo 依赖
 ├── CaptchaService
 ├── GET /captcha 端点
 └── 集成到登录流程

阶段4: 双因素认证         (依赖 阶段1)
 ├── V12 migration + Cargo 依赖
 ├── Crypto 工具 + ApplicationConfig
 ├── Entity + TOTP/SMS/SecondPassword 服务
 ├── TwoFactorAppService + Handler
 ├── PreAuthToken 机制
 └── 改造 AuthService::login()

阶段5: 前端               (依赖 阶段 2-4)
 ├── API 函数
 ├── 登录页改造
 ├── 设置页
 ├── 个人中心 2FA
 └── 路由 + 菜单
```

---

## Verification

1. **编译**: `cargo check -p genies_config -p genies_auth_admin`
2. **前端构建**: `cd crates/auth-admin/web && npm run build`
3. **功能验证**:
   - 默认状态 (所有开关关闭)：登录行为不变，向后兼容
   - 启用验证码：登录页出现验证码，错误验证码被拒绝
   - 启用密码策略：弱密码被拒绝，给出具体提示
   - 启用 2FA (TOTP)：登录后进入二次验证，扫码绑定后正常签发 JWT
