---
name: auth-admin-usage
description: Guide for using the genies_auth_admin unified authentication management backend. Use when working with user/role/permission/department/application/instance CRUD, auth-admin API endpoints, admin UI, local JWT authentication, DDD architecture in auth-admin, Dapr event integration, app-proxy permission management, or microservice instance registration/heartbeat. Also use when the user asks about auth-admin startup, configuration, database migrations, or department member_count.
---

# Auth Admin (genies_auth_admin)

## Overview

genies_auth_admin is a standalone unified authentication management backend service built on the genies framework. It provides comprehensive user, role, permission, department (organization), and application management capabilities. All CRUD mutations publish CloudEvents via Dapr pub/sub, which downstream `genies_auth` instances consume to synchronize Casbin rules.

**Key Features:**
- Full CRUD for users, roles, permissions, departments, and applications
- Local JWT authentication (login, logout, token refresh, password management)
- **Two-factor authentication (2FA)** — TOTP (authenticator app), SMS, and second_password methods
- **Login captcha** — CAPTCHA image verification during login
- **Password policy** — configurable strength requirements (min length, uppercase, lowercase, digit, special)
- **System settings management** — runtime-configurable 2FA/captcha/password policy via API
- Role-based access control with role-permission and user-role assignments
- Department (organization) tree with member management and member_count
- Application registry with permission proxy to downstream microservices
- Microservice instance registry with heartbeat and stale detection
- Dapr pub/sub event-driven synchronization to genies_auth
- Embedded Vue 3 Admin UI (rust-embed)
- Casbin-based API-level + field-level permission enforcement
- OpenAPI/SwaggerUI support via Salvo
- Flyway database migrations via flyway-rbatis

**Crate:** `genies_auth_admin` (binary: `auth-admin`)

## Architecture

### DDD Four-Layer Structure

```
crates/auth-admin/src/
├── interfaces/          # Interface layer — HTTP handlers, routes, DTOs, Admin UI
│   ├── router.rs        # Route assembly (public / internal / protected)
│   ├── admin_ui.rs      # Embedded SPA static file server
│   ├── internal_auth.rs # Service-to-service JWT verification handler
│   ├── handler/         # HTTP endpoint handlers
│   │   ├── auth_handler.rs         # Login, logout, refresh, me, change password,
│   │   │                            #   captcha, 2FA verification
│   │   ├── two_factor_handler.rs   # User 2FA binding & management (TOTP/SMS/second_password)
│   │   ├── settings_handler.rs     # System settings CRUD (2FA/captcha/password policy)
│   │   ├── user_handler.rs         # User CRUD + role/department assignment
│   │   ├── role_handler.rs         # Role CRUD + permission assignment
│   │   ├── permission_handler.rs   # Permission CRUD
│   │   ├── department_handler.rs   # Department CRUD + move + members
│   │   ├── application_handler.rs  # Application CRUD
│   │   ├── instance_handler.rs     # Instance registration/heartbeat/deregister + admin query
│   │   ├── app_proxy_handler.rs    # Proxy to downstream /auth/* endpoints
│   │   └── sync_handler.rs         # Export user-role mappings (internal)
│   └── dto/             # Interface-layer DTOs (ApplicationVO, InstanceVO, etc.)
│       └── instance_dto.rs         # Instance DTOs (RegisterInstanceRequest, HeartbeatRequest,
│                                   #   DeregisterRequest, InstanceVO)
├── application/         # Application layer — use-case orchestration
│   ├── service.rs       # AuthService (login with captcha/2FA), UserAppService,
│   │                    #   RoleAppService, PermissionAppService, DepartmentAppService,
│   │                    #   UserDepartmentAppService, SyncAppService
│   ├── two_factor_service.rs  # TwoFactorAppService (TOTP/SMS/second_password setup & verification)
│   ├── settings_service.rs    # SettingsAppService (2FA/captcha/password policy CRUD)
│   ├── app_service.rs         # ApplicationAppService
│   └── dto.rs           # PageQuery, PageResult, LoginRequest/Response, ChangePasswordRequest,
│                        #   TwoFactorVerifyRequest, PreAuthClaims
├── domain/              # Domain layer — entities, repositories, domain services
│   ├── entity/          # RBatis entities (AdminUser, AdminRole, AdminPermission,
│   │                    #   AdminDepartment, UserRole, RolePermission,
│   │                    #   UserDepartment, ApplicationEntity, AppInstanceEntity,
│   │                    #   UserRoleMapping, UserTwoFactor, AdminSetting)
│   │   ├── user_2fa_entity.rs     # UserTwoFactor (auth_admin_user_2fa)
│   │   ├── settings_entity.rs     # AdminSetting (auth_admin_settings)
│   │   └── app_instance_entity.rs  # AppInstanceEntity (auth_app_instances)
│   ├── repository/      # RBatis SQL queries (html_sql, py_sql)
│   │   ├── user_2fa_repository.rs  # User 2FA queries (upsert/update/delete)
│   │   └── app_instance_repository.rs  # Instance queries
│   └── service/         # Domain services
│       ├── captcha_service.rs     # CaptchaService — image generation + cache verification
│       ├── totp_service.rs        # TotpService — TOTP secret/QCR/verify + backup codes
│       ├── sms_service.rs         # SmsService — SMS code send/verify via SmsGateway trait
│       ├── second_password_service.rs # SecondPasswordService — bcrypt hash/verify
│       ├── settings_service.rs    # SettingsDomainService — typed JSON settings CRUD
│       ├── password_policy_service.rs # PasswordPolicyService — policy validation
│       └── app_instance_service.rs     # Instance domain service
│                        #   — DB writes + Dapr event publishing
└── infrastructure/      # Infrastructure layer
    ├── crypto.rs        # CryptoUtil — AES-256-GCM encrypt/decrypt for TOTP secrets
    ├── migration.rs     # Flyway migration runner
    └── mod.rs
```

### Request Flow

```
Request → public_routes (no auth)     → Handler → Service → Repository → DB
        → internal_routes (JWT only)  → Handler → Service → Repository → DB
        → protected_routes            → local_auth(JWT) → casbin_auth → Handler → Service → Repository → DB
```

Three route groups:
1. **Public** — `/auth-admin/login`, `/auth-admin/logout`, `/auth-admin/refresh`, `/auth-admin/captcha`, `/auth-admin/2fa/verify`, `/auth-admin/ui/**`
2. **Internal** — `/auth-admin/sync/user-roles`, `/auth-admin/internal/instances/register`, `/auth-admin/internal/instances/heartbeat`, `/auth-admin/internal/instances/deregister` (service-to-service, JWT signature only)
3. **Protected** — All management APIs (JWT + Casbin permission check), including user self-service 2FA endpoints (`/auth-admin/me/2fa/**`) and system settings (`/auth-admin/settings/**`)

## API Endpoint Reference

All endpoints use the `/auth-admin` prefix. Response format: `RespVO<T>` (`{ code, msg, data }`).

### Authentication (tag: `auth`)

| Endpoint | Method | Auth | Description |
|----------|--------|------|-------------|
| `/auth-admin/login` | POST | Public | Login with username/password (supports captcha & 2FA) |
| `/auth-admin/logout` | POST | Public | Logout (client discards token) |
| `/auth-admin/refresh` | POST | Public | Refresh JWT token |
| `/auth-admin/captcha` | GET | Public | Get captcha image (when captcha is enabled) |
| `/auth-admin/2fa/verify` | POST | Public | Verify 2FA code, exchange preauth_token for full JWT |
| `/auth-admin/me` | GET | Protected | Get current user profile |
| `/auth-admin/me/password` | PUT | Protected | Change current user password |

**Login request (with optional captcha fields):**
```json
{
  "username": "admin",
  "password": "123456",
  "captcha_id": "",
  "captcha_text": ""
}
```

**Login response (without 2FA — full JWT):**
```json
{
  "code": "0",
  "data": {
    "access_token": "eyJ...",
    "token_type": "Bearer",
    "expires_in": 7200,
    "username": "admin",
    "display_name": "Admin",
    "require_2fa": false,
    "preauth_token": null,
    "available_methods": [],
    "require_2fa_setup": false,
    "two_fa_setup_deadline": null
  }
}
```

**Login response (with 2FA required — preauth token):**
```json
{
  "code": "0",
  "data": {
    "access_token": "",
    "token_type": "Bearer",
    "expires_in": 300,
    "username": "admin",
    "display_name": "Admin",
    "require_2fa": true,
    "preauth_token": "eyJ...",
    "available_methods": ["totp", "sms"],
    "require_2fa_setup": false,
    "two_fa_setup_deadline": null
  }
}
```

**Login response (2FA enforcement — user must set up 2FA):**
```json
{
  "code": "0",
  "data": {
    "access_token": "eyJ...",
    "token_type": "Bearer",
    "expires_in": 7200,
    "username": "newuser",
    "display_name": "New User",
    "require_2fa": false,
    "preauth_token": null,
    "available_methods": [],
    "require_2fa_setup": true,
    "two_fa_setup_deadline": 1715875200
  }
}
```

Response fields:
| Field | Type | Description |
|-------|------|-------------|
| `access_token` | String | Full JWT token (empty string if 2FA required) |
| `token_type` | String | Always "Bearer" |
| `expires_in` | usize | Token expiry in seconds (default 7200, 300 for preauth) |
| `username` | String | Login username |
| `display_name` | String | User display name |
| `require_2fa` | bool | Whether 2FA verification is required |
| `preauth_token` | Option\<String\> | Short-lived (5 min) preauth JWT for 2FA verification |
| `available_methods` | Vec\<String\> | Available 2FA methods when `require_2fa=true` |
| `require_2fa_setup` | bool | System requires user to set up 2FA (grace period exceeded) |
| `two_fa_setup_deadline` | Option\<usize\> | 2FA setup deadline (UNIX timestamp seconds, null=immediate) |

---

### Login Captcha

**GET `/auth-admin/captcha`** — obtain a captcha image (only works when captcha is enabled in settings)

Response:
```json
{
  "code": "0",
  "data": {
    "captcha_id": "550e8400-e29b-41d4-a716-446655440000",
    "image_base64": "iVBORw0KGgo..."
  }
}
```

- Captcha images are 160x60 PNG, 4 characters, with noise/wave/dot filters
- Captcha has 5-minute TTL and is one-time use (deleted on verification)
- Case-insensitive comparison
- Captcha must be enabled via settings (`PUT /auth-admin/settings/auth/captcha` with `{"enabled": true}`)

---

### 2FA Verification

**POST `/auth-admin/2fa/verify`** — exchange the preauth_token for a full JWT by providing the 2FA code

Request:
```json
{
  "preauth_token": "eyJ...",
  "code": "123456",
  "method": "totp"
}
```

| Field | Type | Description |
|-------|------|-------------|
| `preauth_token` | String | The preauth_token from login response |
| `code` | String | 2FA code (TOTP: 6-digit, SMS: 6-digit, second_password: user's password) |
| `method` | String | `"totp"`, `"sms"`, or `"second_password"` |

Response: Same as full JWT login response (above).

### User Self-Service 2FA (tag: `2fa`)

All endpoints require JWT authentication (Protected). The user's ID is extracted from JWT claims.

| Endpoint | Method | Description |
|----------|--------|-------------|
| `/auth-admin/me/2fa` | GET | Get current user's 2FA status |
| `/auth-admin/me/2fa/totp/setup` | POST | Initiate TOTP binding (returns secret + QR code) |
| `/auth-admin/me/2fa/totp/confirm` | POST | Confirm TOTP binding with verification code |
| `/auth-admin/me/2fa/second-password` | POST | Set a second password |
| `/auth-admin/me/2fa/sms/setup` | POST | Initiate SMS 2FA binding (sends code to phone) |
| `/auth-admin/me/2fa/sms/send` | POST | Re-send SMS verification code |
| `/auth-admin/me/2fa/sms/verify` | POST | Verify SMS code and enable SMS 2FA |
| `/auth-admin/me/2fa` | DELETE | Disable 2FA entirely |

**GET `/auth-admin/me/2fa`** — get current user's 2FA status

Response:
```json
{
  "code": "0",
  "data": {
    "enabled": true,
    "method": "totp",
    "phone": "",
    "allowed_methods": ["totp", "sms", "second_password"]
  }
}
```

**POST `/auth-admin/me/2fa/totp/setup`** — initiate TOTP binding

Response:
```json
{
  "code": "0",
  "data": {
    "secret": "JBSWY3DPEHPK3PXP",
    "otpauth_url": "otpauth://totp/auth-admin:username?secret=JBSWY3DPEHPK3PXP&issuer=auth-admin&algorithm=SHA1&digits=6&period=30",
    "qr_svg": "<svg>...</svg>"
  }
}
```

**POST `/auth-admin/me/2fa/totp/confirm`** — confirm TOTP binding

Request: `{ "code": "123456" }`

Response:
```json
{
  "code": "0",
  "data": {
    "backup_codes": ["a1b2c3d4", "e5f6a7b8", "c9d0e1f2", "a3b4c5d6", "e7f8a9b0", "c1d2e3f4", "a5b6c7d8", "e9f0a1b2"]
  }
}
```

- Returns 8 backup codes (8-digit hex, plaintext) — **only returned once** on confirm. Store them securely.

**POST `/auth-admin/me/2fa/second-password`** — set a second password

Request: `{ "password": "my_second_password" }` (min 4 characters)

- The second password is bcrypt-hashed and stored. It is NOT the same as the main login password.

**POST `/auth-admin/me/2fa/sms/setup`** — initiate SMS 2FA binding

Request: `{ "phone": "13800138000" }`

- Sends a 6-digit SMS verification code to the phone. Use `/sms/verify` to confirm.

**POST `/auth-admin/me/2fa/sms/verify`** — confirm SMS 2FA binding

Request: `{ "code": "123456" }`

**DELETE `/auth-admin/me/2fa`** — disable 2FA

Response: `{ "code": "0", "msg": "ok", "data": null }`

### Admin 2FA Management (tag: `admin-2fa`)

| Endpoint | Method | Description |
|----------|--------|-------------|
| `/auth-admin/admin/users/{id}/2fa/reset` | POST | Admin force-reset a user's 2FA |

### System Settings (tag: `settings`)

| Endpoint | Method | Description |
|----------|--------|-------------|
| `/auth-admin/settings` | GET | Get all system settings (2FA + captcha + password policy) |
| `/auth-admin/settings/auth/2fa` | PUT | Update 2FA settings |
| `/auth-admin/settings/auth/captcha` | PUT | Update captcha settings |
| `/auth-admin/settings/auth/password` | PUT | Update password policy settings |

**GET `/auth-admin/settings`** response:
```json
{
  "code": "0",
  "data": {
    "two_fa": {
      "enabled": true,
      "methods": ["totp", "sms"],
      "grace_days": 7,
      "enabled_at": "2025-01-01T00:00:00+00:00"
    },
    "captcha": { "enabled": true },
    "password": {
      "min_length": 8,
      "require_uppercase": true,
      "require_lowercase": true,
      "require_digit": true,
      "require_special": false
    }
  }
}
```

TwoFactorSettings:
| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `enabled` | bool | false | Enable 2FA globally |
| `methods` | Vec\<String\> | [] | Allowed 2FA methods: `"totp"`, `"sms"`, `"second_password"`. Empty means all supported methods allowed. |
| `grace_days` | u32 | 0 | Grace period days for existing users to set up 2FA. 0=immediate enforcement. |
| `enabled_at` | Option\<String\> | auto | ISO8601 timestamp when 2FA was first enabled (auto-managed by system) |

CaptchaSettings:
| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `enabled` | bool | false | Enable login captcha |

PasswordPolicySettings:
| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `min_length` | u32 | 6 | Minimum password length |
| `require_uppercase` | bool | false | Require at least one uppercase letter |
| `require_lowercase` | bool | false | Require at least one lowercase letter |
| `require_digit` | bool | false | Require at least one digit |
| `require_special` | bool | false | Require at least one special character |

---

## 2FA Login Flow

### Full Login Sequence

```
┌──────────────────────────────────────────────────────────────────────────┐
│  1. GET /auth-admin/captcha                                               │
│     (if captcha enabled) Get captcha image + captcha_id                    │
│     Returns: { captcha_id, image_base64 }                                 │
└──────────────────────────────────────────────────────────────────────────┘
                                    │
                                    ▼
┌──────────────────────────────────────────────────────────────────────────┐
│  2. POST /auth-admin/login                                                │
│     Body: { username, password, [captcha_id, captcha_text] }              │
│     ├── Validates credentials + captcha                                   │
│     ├── Checks if 2FA is enabled globally AND user has configured 2FA     │
│     │                                                                     │
│     ├── [2FA NOT required] → Returns full JWT in access_token             │
│     │   { access_token: "eyJ...", require_2fa: false, ... }              │
│     │                                                                     │
│     ├── [2FA required] → Returns preauth_token (5-min expiry)             │
│     │   { access_token: "", require_2fa: true,                            │
│     │     preauth_token: "eyJ...", available_methods: [...] }            │
│     │                                                                     │
│     └── [2FA enforcement] → Returns full JWT + setup prompt               │
│         { access_token: "eyJ...", require_2fa_setup: true,                │
│           two_fa_setup_deadline: 1715875200 }                             │
└──────────────────────────────────────────────────────────────────────────┘
                                    │
                          (if require_2fa=true)
                                    ▼
┌──────────────────────────────────────────────────────────────────────────┐
│  3. POST /auth-admin/2fa/verify                                           │
│     Body: { preauth_token, code, method }                                 │
│     ├── Validates preauth JWT (purpose="2fa_preauth", 5-min expiry)       │
│     ├── Verifies 2FA code against user's configured method:               │
│     │   * totp: 6-digit TOTP code (SHA1, 30s) or backup code              │
│     │   * sms: 6-digit SMS code (cached, one-time)                        │
│     │   * second_password: bcrypt-verified password                       │
│     └── Returns full JWT in access_token                                  │
└──────────────────────────────────────────────────────────────────────────┘
```

### 2FA Preauth Token

When 2FA is required, login returns a `preauth_token` with these JWT claims:
- `uid`: user ID
- `iat`: issued-at timestamp
- `exp`: expiration (5 minutes after issue)
- `purpose`: `"2fa_preauth"` (validated during verification to prevent token type confusion)

The preauth token is short-lived (5 minutes) and can ONLY be used for the `/auth-admin/2fa/verify` endpoint.

### 2FA Grace Period

When 2FA is first enabled globally:
1. `enabled_at` is auto-set to the current ISO8601 timestamp
2. `grace_days` defines how many days existing users have to set up 2FA
3. Before the deadline: users who haven't set up 2FA get `require_2fa_setup: false` (can skip)
4. After the deadline (`now >= enabled_at + grace_days * 86400`): users get `require_2fa_setup: true` with a full JWT, but frontend should redirect them to the 2FA setup page
5. `grace_days = 0` means immediate enforcement for all existing users

### Three 2FA Methods

| Method | Key | Verification | Backup | Notes |
|--------|-----|-------------|--------|-------|
| `totp` | TOTP secret (AES-256-GCM encrypted) | 6-digit code, SHA1, 30s window | 8 backup codes (8-digit hex, bcrypt hashed) | Returns plaintext backup codes once on confirm |
| `sms` | Phone number | 6-digit code via cache (5-min TTL) | None | Requires SmsGateway implementation |
| `second_password` | bcrypt-hashed password | Direct bcrypt comparison | None | Min 4 characters, separate from main password |

### Backup Codes (TOTP only)

- Generated on TOTP confirm: 8 codes, each 8 hex digits (e.g., `a1b2c3d4`)
- Plaintext returned ONCE in the confirm response — user must save them
- Stored in DB as bcrypt-hashed JSON array
- Can be used as an alternative to TOTP code during /2fa/verify
- Each code can be used multiple times (not single-use)

---

### User Management (tag: `users`)

| Endpoint | Method | Description |
|----------|--------|-------------|
| `/auth-admin/users` | GET | Paginated user list (`?page=1&size=10&keyword=`) |
| `/auth-admin/users` | POST | Create user |
| `/auth-admin/users/{id}` | GET | Get user detail |
| `/auth-admin/users/{id}` | PUT | Update user |
| `/auth-admin/users/{id}` | DELETE | Delete user |
| `/auth-admin/users/batch-delete` | POST | Batch delete users (`{ "ids": [1,2,3] }`) |
| `/auth-admin/users/{id}/status` | PUT | Enable/disable user (`{ "status": 1 }`) |
| `/auth-admin/users/{id}/reset-password` | PUT | Reset password (`{ "password": "newpass" }`) |
| `/auth-admin/users/{id}/roles` | GET | Get user's assigned roles |
| `/auth-admin/users/{id}/roles` | POST | Assign role (`{ "role_id": 1 }`) |
| `/auth-admin/users/{id}/roles/{role_id}` | DELETE | Revoke role |
| `/auth-admin/users/{id}/permissions` | GET | Get user's effective permissions (merged from all roles) |
| `/auth-admin/users/{id}/departments` | GET | Get user's departments |
| `/auth-admin/users/{id}/departments` | POST | Assign departments (body: `[1, 2, 3]`) |

### Role Management (tag: `roles`)

| Endpoint | Method | Description |
|----------|--------|-------------|
| `/auth-admin/roles` | GET | List all roles |
| `/auth-admin/roles` | POST | Create role |
| `/auth-admin/roles/{id}` | GET | Get role detail |
| `/auth-admin/roles/{id}` | PUT | Update role |
| `/auth-admin/roles/{id}` | DELETE | Delete role |
| `/auth-admin/roles/{id}/users` | GET | List users with this role |
| `/auth-admin/roles/{id}/permissions` | GET | List role's permissions |
| `/auth-admin/roles/{id}/permissions` | POST | Assign permission (`{ "permission_id": 1 }`) |
| `/auth-admin/roles/{id}/permissions/{perm_id}` | DELETE | Revoke permission |

### Permission Management (tag: `permissions`)

| Endpoint | Method | Description |
|----------|--------|-------------|
| `/auth-admin/permissions` | GET | List all permissions |
| `/auth-admin/permissions` | POST | Create permission |
| `/auth-admin/permissions/{id}` | GET | Get permission detail |
| `/auth-admin/permissions/{id}` | PUT | Update permission |
| `/auth-admin/permissions/{id}` | DELETE | Delete permission |

### Department Management (tag: `departments`)

| Endpoint | Method | Description |
|----------|--------|-------------|
| `/auth-admin/departments` | GET | List all departments (includes `member_count` for each) |
| `/auth-admin/departments` | POST | Create department |
| `/auth-admin/departments/{id}` | GET | Get department detail |
| `/auth-admin/departments/{id}` | PUT | Update department |
| `/auth-admin/departments/{id}` | DELETE | Delete department |
| `/auth-admin/departments/{id}/move/{parent_id}` | PUT | Move department to new parent |
| `/auth-admin/departments/{id}/users` | GET | List department members |

> **member_count**: When listing departments, each `AdminDepartment` includes a `member_count` field that is populated by querying `UserDepartment::count_members_by_department`. Departments with no members show `member_count: 0`.

### Application Management (tag: `apps`)

| Endpoint | Method | Description |
|----------|--------|-------------|
| `/auth-admin/apps` | GET | Paginated application list (`?page=1&size=10&keyword=`) |
| `/auth-admin/apps` | POST | Create application |
| `/auth-admin/apps/{id}` | GET | Get application detail |
| `/auth-admin/apps/{id}` | PUT | Update application |
| `/auth-admin/apps/{id}` | DELETE | Delete application |

### Application Permission Proxy (tag: `app-proxy`)

These endpoints proxy requests to downstream microservices' `/auth/*` endpoints based on the application's `base_url`. Responses are forwarded transparently (not wrapped in RespVO).

### Instance Management — Internal (tag: `instances`)

| Endpoint | Method | Auth | Description |
|----------|--------|------|-------------|
| `/auth-admin/internal/instances/register` | POST | Internal (JWT) | Register microservice instance |
| `/auth-admin/internal/instances/heartbeat` | POST | Internal (JWT) | Instance heartbeat |
| `/auth-admin/internal/instances/deregister` | POST | Internal (JWT) | Deregister instance |

### Instance Management — Protected (tag: `instances`)

| Endpoint | Method | Description |
|----------|--------|-------------|
| `/auth-admin/apps/{app_id}/instances` | GET | List instances of an application |
| `/auth-admin/instances` | GET | List all instances (paginated, `?page=1&size=10&keyword=`) |

### Application Permission Proxy — Endpoints (tag: `app-proxy`)

| Endpoint | Method | Description |
|----------|--------|-------------|
| `/auth-admin/apps/{id}/schemas` | GET | Proxy: list target app's API schemas |
| `/auth-admin/apps/{id}/policies` | GET | Proxy: list target app's Casbin policies |
| `/auth-admin/apps/{id}/policies` | POST | Proxy: add policy to target app |
| `/auth-admin/apps/{id}/policies/{policy_id}` | DELETE | Proxy: remove policy from target app |
| `/auth-admin/apps/{id}/roles` | GET | Proxy: list target app's role mappings |
| `/auth-admin/apps/{id}/roles` | POST | Proxy: add role mapping to target app |
| `/auth-admin/apps/{id}/roles/{role_id}` | DELETE | Proxy: remove role mapping from target app |
| `/auth-admin/apps/{id}/groups` | GET | Proxy: list target app's resource groups |
| `/auth-admin/apps/{id}/groups` | POST | Proxy: add group to target app |
| `/auth-admin/apps/{id}/groups/{group_id}` | DELETE | Proxy: remove group from target app |
| `/auth-admin/apps/{id}/reload` | POST | Proxy: reload target app's Casbin enforcer |
| `/auth-admin/apps/{id}/sync-user-roles` | POST | Push all active user-role mappings to target app |

### Data Sync — Internal (tag: `sync`)

| Endpoint | Method | Auth | Description |
|----------|--------|------|-------------|
| `/auth-admin/sync/user-roles` | GET | Internal (JWT signature) | Export all active user-role mappings (Casbin 'g' format) |

## Data Models

### Core Entities

**AdminUser** (`auth_admin_users`)
| Field | Type | Description |
|-------|------|-------------|
| id | i64 (auto) | Primary key |
| username | String | Unique login name |
| password_hash | String | bcrypt hashed password |
| display_name | String | Display name |
| email | Option\<String\> | Email address |
| phone | Option\<String\> | Phone number |
| avatar | Option\<String\> | Avatar URL |
| status | i8 | 1=active, 0=disabled |
| last_login_at | Option\<DateTime\> | Last login timestamp |
| created_at | Option\<DateTime\> | Creation time |
| updated_at | Option\<DateTime\> | Last update time |

**AdminRole** (`auth_admin_roles`)
| Field | Type | Description |
|-------|------|-------------|
| id | i64 (auto) | Primary key |
| name | String | Role identifier (e.g., "admin") |
| display_name | String | Display name |
| description | Option\<String\> | Description |
| parent_id | Option\<i64\> | Parent role (hierarchy) |
| status | i8 | 1=active, 0=disabled |

**AdminPermission** (`auth_admin_permissions`)
| Field | Type | Description |
|-------|------|-------------|
| id | i64 (auto) | Primary key |
| name | String | Permission name |
| resource | String | Resource path (e.g., "/auth-admin/users") |
| action | String | HTTP method (GET/POST/PUT/DELETE) |
| description | Option\<String\> | Description |
| status | i8 | 1=active, 0=disabled |

**AdminDepartment** (`auth_admin_departments`)
| Field | Type | Description |
|-------|------|-------------|
| id | i64 (auto) | Primary key |
| name | String | Department name |
| parent_id | Option\<i64\> | Parent department (tree) |
| sort_order | i32 | Sort order |
| description | Option\<String\> | Description |
| status | i8 | 1=active, 0=disabled |
| member_count | Option\<i64\> | Computed field (not in DB), populated on list query |

**ApplicationEntity** (`auth_applications`)
| Field | Type | Description |
|-------|------|-------------|
| id | i64 (auto) | Primary key |
| app_name | Option\<String\> | Application identifier |
| display_name | Option\<String\> | Display name |
| description | Option\<String\> | Description |
| base_url | Option\<String\> | Target microservice URL for proxy |
| status | Option\<i8\> | 1=active, 0=disabled |

**AppInstanceEntity** (`auth_app_instances`)
| Field | Type | Description |
|-------|------|-------------|
| id | i64 (auto) | Primary key |
| app_name | String | Application name (matches auth_applications.app_name) |
| instance_id | i64 | Snowflake ID, generated on startup |
| base_url | String | Instance access URL |
| version | String | Service version |
| status | i8 | 1=online, 0=offline |
| last_heartbeat_at | DateTime | Last heartbeat timestamp |
| registered_at | DateTime | Registration timestamp |
| metadata | Option\<String\> | JSON extension data |

**UserTwoFactor** (`auth_admin_user_2fa`) — user's 2FA configuration
| Field | Type | Description |
|-------|------|-------------|
| id | i64 (auto) | Primary key |
| user_id | i64 | User ID (unique, one 2FA record per user) |
| method | String | 2FA method: `"totp"`, `"sms"`, or `"second_password"` |
| enabled | i8 | 1=enabled, 0=pending confirmation |
| secret | String | AES-256-GCM encrypted TOTP secret, or bcrypt-hashed second password |
| phone | String | Phone number (used for SMS method) |
| backup_codes | Option\<String\> | JSON array of bcrypt-hashed backup codes (TOTP only) |

**AdminSetting** (`auth_admin_settings`) — system settings (key-value with JSON values)
| Field | Type | Description |
|-------|------|-------------|
| id | i64 (auto) | Primary key |
| setting_key | String | Unique setting key: `"auth.2fa"`, `"auth.captcha"`, `"auth.password"` |
| setting_value | String (JSON) | JSON-encoded setting value |
| description | String | Human-readable description |
| created_at | DateTime | Creation time |
| updated_at | DateTime | Last update time |

### Association Tables

| Table | Fields | Description |
|-------|--------|-------------|
| `auth_admin_user_roles` | user_id, role_id | User-Role many-to-many |
| `auth_admin_role_permissions` | role_id, permission_id | Role-Permission many-to-many |
| `auth_admin_user_departments` | user_id, department_id | User-Department many-to-many |

### Entity Relationships

```
AdminUser ──M:N──> AdminRole ──M:N──> AdminPermission
    │
    ├──1:1──> UserTwoFactor (2FA configuration)
    │
    └──M:N──> AdminDepartment (tree via parent_id)

AdminSetting (key-value, independent of other entities)

ApplicationEntity (independent, used for permission proxy)
    └──1:N──> AppInstanceEntity (running instances)
```

## Configuration (application.yml)

```yaml
# Debug mode
debug: true

# Service name and route prefix
server_name: "auth-admin"
servlet_path: "/auth-admin"

# HTTP server address
server_url: "0.0.0.0:9099"

# Gateway address
gateway: "http://127.0.0.1:6015"

# Cache
cache_type: "redis"
redis_url: "redis://127.0.0.1:6379"

# Database
database_url: "mysql://root:password@127.0.0.1:3306/auth_admin_service"

# Connection pool
max_connections: 20
min_connections: 0
wait_timeout: 60
max_lifetime: 1800
create_timeout: 120

# Logging
log_level: "debug,flyway=info"

# JWT configuration (auth-admin specific)
jwt_secret: "auth_admin_jwt_secret_change_in_production"
auth_mode: "local"
# JWT 过期时间（秒），默认 7200（2小时）
jwt_expires_in_secs: 7200
# 2FA 加密密钥（32字节 hex，用于加密 TOTP 密钥。留空则自动生成随机密钥）
two_fa_encryption_key: "781b58689844f9be5e71ae751ed818f60d844851b7930165696978866b410ced"

# Dapr pub/sub
dapr_pubsub_name: messagebus
dapr_pub_message_limit: 50
dapr_cdc_message_period: 5000

# API whitelist (no auth required)
white_list_api:
  - "/auth-admin/login"
  - "/auth-admin/logout"
  - "/auth-admin/refresh-token"
  - "/dapr/*"
  - "/system/*"
  - "/actuator/*"
```

Key configuration notes:
- `auth_mode: "local"` — uses local JWT authentication (not Keycloak)
- `jwt_secret` — shared secret for JWT signing; **must change in production**
- `jwt_expires_in_secs` — JWT token expiry in seconds (default 7200 = 2 hours)
- `two_fa_encryption_key` — 64-char hex string encoding a 32-byte AES-256-GCM key for TOTP secret encryption. If left empty or invalid, a random key is auto-generated on startup (**warning:** this invalidates all stored TOTP secrets on restart)
- `servlet_path: "/auth-admin"` — all API paths are prefixed with `/auth-admin`

## Integration with genies_auth

auth-admin integrates with the `genies_auth` Casbin permission system in two ways:

### 1. As a Protected Service

auth-admin itself uses `genies_auth` middleware for access control on protected routes:

```rust
let protected_router = Router::new()
    .hoop(affix_state::inject(auth_config))
    .hoop(genies_auth::local_auth)           // JWT verification
    .hoop(affix_state::inject(mgr.clone()))   // Inject EnforcerManager
    .hoop(casbin_auth)                        // Casbin permission check
    .push(genies_auth_admin::interfaces::router::protected_routes())
    .push(auth_router());               // genies_auth Admin API (14 endpoints)
```

The middleware chain: `local_auth` (JWT) → `inject(EnforcerManager)` → `casbin_auth` (Casbin check)

### 2. As a Central Permission Manager

auth-admin manages users/roles/permissions and publishes events via Dapr when mutations occur:
- `UserCreatedEvent`, `UserUpdatedEvent`, `UserDeletedEvent`
- `UserRoleAssignedEvent`, `UserRoleRevokedEvent`
- `RoleCreatedEvent`, `RoleUpdatedEvent`, `RoleDeletedEvent`
- `RolePermissionAssignedEvent`, `RolePermissionRevokedEvent`

Downstream `genies_auth` instances subscribe to these events and update their local Casbin rules.

### 3. Application Permission Proxy

auth-admin acts as a central hub to manage Casbin policies across all registered applications:
- Register applications with their `base_url`
- Use `/auth-admin/apps/{id}/policies`, `/auth-admin/apps/{id}/roles`, etc. to manage target app's Casbin rules
- Use `/auth-admin/apps/{id}/sync-user-roles` to push user-role mappings to target apps
- All proxy requests forward the `Authorization` header for authentication

### 4. OpenAPI Schema Sync

On startup, auth-admin extracts OpenAPI schemas and syncs them to the `auth_api_schemas` table:

```rust
let doc = OpenApi::new("auth-admin", "1.0.0").merge_router(&router);
extract_and_sync_schemas(&doc).await.ok();
```

## Web UI

### Access Path

The Admin UI is accessible at: **`/auth-admin/ui/`** (trailing slash required)

### Build and Deploy

The frontend is a Vue 3 + Vite application located in `crates/auth-admin/web/`:

```bash
# Install dependencies
cd crates/auth-admin/web
npm install

# Build — output goes to crates/auth-admin/static/
npm run build
```

The build output in `static/` is embedded into the Rust binary using `rust-embed`:

```rust
#[derive(Embed)]
#[folder = "static/"]
struct AdminUiAssets;
```

Features:
- **SPA fallback** — unmatched paths return `index.html`
- **Smart caching** — `assets/*` gets `max-age=31536000` (immutable), `index.html` gets `no-cache`
- **Auto MIME detection** via `mime_guess`

### Vite Configuration

```typescript
// crates/auth-admin/web/vite.config.ts
export default defineConfig({
  plugins: [vue()],
  base: './',           // Relative paths for embedded serving
  build: {
    outDir: '../static', // Output to static/ for rust-embed
    emptyOutDir: true
  }
})
```

## Database Migrations

Flyway migrations are in `crates/auth-admin/migrations/`. On startup, both `genies_auth` and auth-admin migrations run in order:

```rust
// 1. genies_auth migrations (casbin tables)
genies_auth::models::run_migrations().await;
// 2. auth-admin migrations (admin tables)
genies_auth_admin::infrastructure::migration::run_migrations().await;
```

### Migration Files

| Version | File | Description |
|---------|------|-------------|
| V1 | `V1__create_auth_admin_users.sql` | Users table |
| V2 | `V2__create_auth_admin_roles.sql` | Roles table |
| V3 | `V3__create_auth_admin_permissions.sql` | Permissions table |
| V4 | `V4__create_auth_admin_departments.sql` | Departments table |
| V5 | `V5__create_auth_admin_user_roles.sql` | User-Role association |
| V6 | `V6__create_auth_admin_role_permissions.sql` | Role-Permission association |
| V7 | `V7__create_message_table.sql` | Dapr message outbox |
| V8 | `V8__create_auth_applications.sql` | Applications registry |
| V9 | `V9__create_auth_admin_user_departments.sql` | User-Department association |
| V10 | `V10__create_auth_app_instances.sql` | Application instances registry |
| V11 | `V11__create_auth_admin_settings.sql` | System settings table (2FA/captcha/password) |
| V12 | `V12__create_auth_admin_user_2fa.sql` | User 2FA configuration table |

## Startup

### Working Directory Requirement

**CRITICAL:** auth-admin MUST be started from `crates/auth-admin/` directory to load the correct `application.yml`:

```bash
cd crates/auth-admin
cargo run -p genies_auth_admin --bin auth-admin
```

### Startup Sequence

1. Initialize logging (`genies::config::log_config::init_log()`)
2. Initialize database connection (`CONTEXT.init_database().await`)
3. Run Flyway migrations (auth tables → admin tables)
4. Initialize `LocalAuthConfig` with JWT secret
5. Create `EnforcerManager` (with empty-policy fallback on failure)
6. Build routes (public → internal → protected)
7. Generate OpenAPI doc and sync schemas
8. Self-register instance (generate snowflake ID, insert into auth_app_instances)
9. Start background heartbeat loop (every `heartbeat_interval` seconds, default 30s)
10. Start background stale instance cleanup task (every 60s, mark instances offline if no heartbeat for 90s)
11. Start HTTP server on configured `server_url` (default: `0.0.0.0:9099`)

### Dependencies

```toml
[dependencies]
genies = { workspace = true }
genies_core = { workspace = true }
genies_derive = { workspace = true }
genies_config = { workspace = true }
genies_context = { workspace = true }
genies_dapr = { workspace = true }
genies_ddd = { workspace = true }
genies_auth = { workspace = true }
genies_cache = { workspace = true }
salvo = { workspace = true }
rbatis = { workspace = true }
bcrypt = "0.15"
jsonwebtoken = "7.2.0"
flyway = { workspace = true }
flyway-rbatis = { workspace = true }
rust-embed = { version = "8" }
casbin = { version = "2.10.1", features = ["runtime-tokio"] }
reqwest = { version = "0.12", features = ["json"] }
```

## Service Instance Registration

auth-admin includes a built-in microservice instance registry that tracks all running service instances.

### Self-Registration (auth-admin)

On startup, auth-admin directly calls its own domain service (`AppInstanceService`) to register itself as an instance. A snowflake ID is generated as the `instance_id`, and the instance record is inserted into the `auth_app_instances` table.

### Downstream Microservice Registration

Other microservices register themselves by calling the internal HTTP API endpoints. The `genies_auth` crate provides a convenience function `genies_auth::try_register_and_heartbeat` that handles:
- Sending a `POST /auth-admin/internal/instances/register` request on startup
- Starting a background heartbeat loop that periodically calls `POST /auth-admin/internal/instances/heartbeat`

### Heartbeat Mechanism

Registered instances periodically send heartbeat requests to update their `last_heartbeat_at` timestamp. The default heartbeat interval is 30 seconds (configurable via `heartbeat_interval`).

### Stale Instance Detection

A background cleanup task runs every 60 seconds on auth-admin. It checks all instances and marks any instance as offline (`status = 0`) if no heartbeat has been received for 90 seconds.

### Frontend Display

The admin UI's application list supports expanding each application to view its instance details, including:
- Online/offline status indicator
- Last heartbeat timestamp
- Instance base URL and version
- Registration time

## Key Files

- [main.rs](file:///d:/tdcare/genies/crates/auth-admin/src/main.rs) — Application entry point
- [lib.rs](file:///d:/tdcare/genies/crates/auth-admin/src/lib.rs) — Module structure
- [router.rs](file:///d:/tdcare/genies/crates/auth-admin/src/interfaces/router.rs) — Route assembly
- [service.rs](file:///d:/tdcare/genies/crates/auth-admin/src/application/service.rs) — Application services (AuthService with login/2FA)
- [two_factor_service.rs](file:///d:/tdcare/genies/crates/auth-admin/src/application/two_factor_service.rs) — TwoFactorAppService (TOTP/SMS/second_password setup & verify)
- [settings_service.rs](file:///d:/tdcare/genies/crates/auth-admin/src/application/settings_service.rs) — SettingsAppService (2FA/captcha/password CRUD)
- [app_service.rs](file:///d:/tdcare/genies/crates/auth-admin/src/application/app_service.rs) — Application management service
- [dto.rs](file:///d:/tdcare/genies/crates/auth-admin/src/application/dto.rs) — Request/Response DTOs (LoginRequest/Response, TwoFactorVerifyRequest, PreAuthClaims)
- [admin_ui.rs](file:///d:/tdcare/genies/crates/auth-admin/src/interfaces/admin_ui.rs) — Embedded UI server
- [auth_handler.rs](file:///d:/tdcare/genies/crates/auth-admin/src/interfaces/handler/auth_handler.rs) — Auth endpoints (login/captcha/2fa-verify/refresh/me)
- [two_factor_handler.rs](file:///d:/tdcare/genies/crates/auth-admin/src/interfaces/handler/two_factor_handler.rs) — User 2FA self-service endpoints
- [settings_handler.rs](file:///d:/tdcare/genies/crates/auth-admin/src/interfaces/handler/settings_handler.rs) — System settings CRUD endpoints
- [migration.rs](file:///d:/tdcare/genies/crates/auth-admin/src/infrastructure/migration.rs) — Flyway migrations
- [crypto.rs](file:///d:/tdcare/genies/crates/auth-admin/src/infrastructure/crypto.rs) — AES-256-GCM encryption for TOTP secrets
- [app_proxy_handler.rs](file:///d:/tdcare/genies/crates/auth-admin/src/interfaces/handler/app_proxy_handler.rs) — Permission proxy
- [instance_handler.rs](file:///d:/tdcare/genies/crates/auth-admin/src/interfaces/handler/instance_handler.rs) — Instance management endpoints
- [instance_dto.rs](file:///d:/tdcare/genies/crates/auth-admin/src/interfaces/dto/instance_dto.rs) — Instance DTOs
- [app_instance_entity.rs](file:///d:/tdcare/genies/crates/auth-admin/src/domain/entity/app_instance_entity.rs) — AppInstanceEntity
- [user_2fa_entity.rs](file:///d:/tdcare/genies/crates/auth-admin/src/domain/entity/user_2fa_entity.rs) — UserTwoFactor entity
- [settings_entity.rs](file:///d:/tdcare/genies/crates/auth-admin/src/domain/entity/settings_entity.rs) — AdminSetting entity
- [app_instance_repository.rs](file:///d:/tdcare/genies/crates/auth-admin/src/domain/repository/app_instance_repository.rs) — Instance repository
- [app_instance_service.rs](file:///d:/tdcare/genies/crates/auth-admin/src/domain/service/app_instance_service.rs) — Instance domain service
- [captcha_service.rs](file:///d:/tdcare/genies/crates/auth-admin/src/domain/service/captcha_service.rs) — Captcha image generation + verification
- [totp_service.rs](file:///d:/tdcare/genies/crates/auth-admin/src/domain/service/totp_service.rs) — TOTP secret generation/QR/verify/backup codes
- [sms_service.rs](file:///d:/tdcare/genies/crates/auth-admin/src/domain/service/sms_service.rs) — SMS code send/verify via SmsGateway trait
- [second_password_service.rs](file:///d:/tdcare/genies/crates/auth-admin/src/domain/service/second_password_service.rs) — Second password bcrypt hash/verify
- [application.yml](file:///d:/tdcare/genies/crates/auth-admin/application.yml) — Configuration
- [V11__create_auth_admin_settings.sql](file:///d:/tdcare/genies/crates/auth-admin/migrations/V11__create_auth_admin_settings.sql) — Settings table migration
- [V12__create_auth_admin_user_2fa.sql](file:///d:/tdcare/genies/crates/auth-admin/migrations/V12__create_auth_admin_user_2fa.sql) — User 2FA table migration
