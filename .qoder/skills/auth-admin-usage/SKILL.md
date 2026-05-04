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
│   │   ├── auth_handler.rs         # Login, logout, refresh, me, change password
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
│   ├── service.rs       # AuthService, UserAppService, RoleAppService,
│   │                    # PermissionAppService, DepartmentAppService,
│   │                    # UserDepartmentAppService, SyncAppService
│   ├── app_service.rs   # ApplicationAppService
│   └── dto.rs           # PageQuery, PageResult, LoginRequest/Response, ChangePasswordRequest
├── domain/              # Domain layer — entities, repositories, domain services
│   ├── entity/          # RBatis entities (AdminUser, AdminRole, AdminPermission,
│   │                    #   AdminDepartment, UserRole, RolePermission,
│   │                    #   UserDepartment, ApplicationEntity, AppInstanceEntity, UserRoleMapping)
│   │   └── app_instance_entity.rs  # AppInstanceEntity (auth_app_instances)
│   ├── repository/      # RBatis SQL queries (html_sql, py_sql)
│   │   └── app_instance_repository.rs  # Instance queries
│   └── service/         # Domain services (UserDomainService, RoleDomainService)
│       └── app_instance_service.rs     # Instance domain service
│                        #   — DB writes + Dapr event publishing
└── infrastructure/      # Infrastructure layer
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
1. **Public** — `/auth-admin/login`, `/auth-admin/logout`, `/auth-admin/refresh`, `/auth-admin/ui/**`
2. **Internal** — `/auth-admin/sync/user-roles`, `/auth-admin/internal/instances/register`, `/auth-admin/internal/instances/heartbeat`, `/auth-admin/internal/instances/deregister` (service-to-service, JWT signature only)
3. **Protected** — All management APIs (JWT + Casbin permission check)

## API Endpoint Reference

All endpoints use the `/auth-admin` prefix. Response format: `RespVO<T>` (`{ code, msg, data }`).

### Authentication (tag: `auth`)

| Endpoint | Method | Auth | Description |
|----------|--------|------|-------------|
| `/auth-admin/login` | POST | Public | Login with username/password, returns JWT |
| `/auth-admin/logout` | POST | Public | Logout (client discards token) |
| `/auth-admin/refresh` | POST | Public | Refresh JWT token |
| `/auth-admin/me` | GET | Protected | Get current user profile |
| `/auth-admin/me/password` | PUT | Protected | Change current user password |

**Login request:**
```json
{ "username": "admin", "password": "123456" }
```

**Login response:**
```json
{
  "code": "0",
  "data": {
    "access_token": "eyJ...",
    "token_type": "Bearer",
    "expires_in": 86400,
    "username": "admin",
    "display_name": "Admin"
  }
}
```

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
    └──M:N──> AdminDepartment (tree via parent_id)

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
    .push(auth_admin_router());               // genies_auth Admin API (14 endpoints)
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
- [service.rs](file:///d:/tdcare/genies/crates/auth-admin/src/application/service.rs) — Application services
- [app_service.rs](file:///d:/tdcare/genies/crates/auth-admin/src/application/app_service.rs) — Application management service
- [dto.rs](file:///d:/tdcare/genies/crates/auth-admin/src/application/dto.rs) — Request/Response DTOs
- [admin_ui.rs](file:///d:/tdcare/genies/crates/auth-admin/src/interfaces/admin_ui.rs) — Embedded UI server
- [migration.rs](file:///d:/tdcare/genies/crates/auth-admin/src/infrastructure/migration.rs) — Flyway migrations
- [app_proxy_handler.rs](file:///d:/tdcare/genies/crates/auth-admin/src/interfaces/handler/app_proxy_handler.rs) — Permission proxy
- [instance_handler.rs](file:///d:/tdcare/genies/crates/auth-admin/src/interfaces/handler/instance_handler.rs) — Instance management endpoints
- [instance_dto.rs](file:///d:/tdcare/genies/crates/auth-admin/src/interfaces/dto/instance_dto.rs) — Instance DTOs
- [app_instance_entity.rs](file:///d:/tdcare/genies/crates/auth-admin/src/domain/entity/app_instance_entity.rs) — AppInstanceEntity
- [app_instance_repository.rs](file:///d:/tdcare/genies/crates/auth-admin/src/domain/repository/app_instance_repository.rs) — Instance repository
- [app_instance_service.rs](file:///d:/tdcare/genies/crates/auth-admin/src/domain/service/app_instance_service.rs) — Instance domain service
- [application.yml](file:///d:/tdcare/genies/crates/auth-admin/application.yml) — Configuration
