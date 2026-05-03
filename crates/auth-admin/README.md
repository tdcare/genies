# genies_auth_admin

Unified Authentication Administration Center for the Genies (神灯) framework — a full-featured management backend for users, roles, permissions, departments, and multi-application authorization.

## Overview

genies_auth_admin is the **management interface** of the Genies permission system. While `genies_auth` serves as the core RBAC permission engine library (Casbin Enforcer, middleware, field-level filtering), `genies_auth_admin` provides a complete admin backend with:

- A full set of RESTful APIs for managing users, roles, permissions, departments, and applications
- Built-in JWT-based authentication (login / logout / token refresh)
- A Vue 3 + Element Plus web UI embedded directly into the binary via `rust-embed`
- Multi-application API proxy — manage Casbin policies of remote microservices from a single dashboard
- Domain-Driven Design (DDD) layered architecture

## Features

- **User Management**: CRUD, status toggle, password reset, role assignment, permission query, batch delete
- **Role Management**: CRUD, user listing, permission assignment / revocation
- **Permission Management**: CRUD for fine-grained permission items
- **Department Management**: CRUD, tree-based move, user listing by department
- **Application Registry**: Register microservices with their base URLs and manage their authorization remotely
- **API Proxy**: Forward policy / role / group / schema / reload requests to target microservices' `/auth/*` endpoints
- **Local JWT Auth**: Self-contained login flow with bcrypt password hashing and JWT token issuance
- **Casbin Integration**: JWT authentication + Casbin RBAC permission check on all protected routes
- **Field-Level Permission Filtering**: Inherits `genies_auth`'s `#[casbin]` macro for response field filtering
- **Embedded Web UI**: SPA frontend served from `/auth-admin/ui/` with intelligent cache control
- **OpenAPI Auto-Sync**: Extracts schemas from OpenAPI docs and syncs to the permission system
- **Flyway Migrations**: Auto-creates all required database tables on startup
- **Dapr Event Bus**: Publishes CloudEvents after CRUD operations; downstream `genies_auth` syncs Casbin rules

## Architecture

### DDD Layered Structure

```
src/
├── main.rs                    # Entry point — init, migration, routing, server start
├── lib.rs                     # Library root
├── interfaces/                # Interface Layer
│   ├── router.rs              #   Route definitions (public + protected)
│   ├── admin_ui.rs            #   Embedded SPA static asset serving
│   ├── handler/               #   HTTP handlers
│   │   ├── auth_handler.rs    #     Login / Logout / Refresh / Me / Change password
│   │   ├── user_handler.rs    #     User CRUD + role assignment
│   │   ├── role_handler.rs    #     Role CRUD + permission assignment
│   │   ├── permission_handler.rs  # Permission CRUD
│   │   ├── department_handler.rs  # Department CRUD + move
│   │   ├── application_handler.rs # Application registry CRUD
│   │   └── app_proxy_handler.rs   # Multi-app API proxy
│   └── dto/                   #   Request / Response DTOs
├── application/               # Application Layer
│   ├── service.rs             #   AuthService, UserService, RoleService, etc.
│   ├── app_service.rs         #   ApplicationAppService
│   └── dto.rs                 #   Shared DTOs (LoginResponse, PageQuery, etc.)
├── domain/                    # Domain Layer
│   ├── entity/                #   AdminUser, AdminRole, AdminPermission, AdminDepartment, ApplicationEntity
│   ├── aggregate/             #   Aggregate roots (User, Role, Permission, Department)
│   ├── service/               #   UserDomainService, RoleDomainService, ApplicationDomainService
│   ├── repository/            #   RBatis repository implementations
│   └── event/                 #   Domain events (UserEvent, RoleEvent)
└── infrastructure/            # Infrastructure Layer
    └── migration.rs           #   Flyway migration runner
```

### Middleware Flow

```
Request → JWT Auth (local_auth) → Casbin RBAC (casbin_auth) → Handler → Writer (field filter) → Response
```

Public routes (login, logout, refresh, admin UI) bypass authentication.

## Tech Stack

| Category | Technology |
|----------|-----------|
| Web Framework | [Salvo](https://salvo.rs) |
| ORM | [RBatis](https://rbatis.github.io/rbatis.io/) |
| Authorization | [Casbin](https://casbin.org/) 2.10 |
| Password Hashing | bcrypt |
| Token | jsonwebtoken (JWT) |
| Database Migration | Flyway (flyway + flyway-rbatis) |
| Database | MySQL |
| Event Bus | Dapr pub/sub (CloudEvents) |
| Caching | Redis |
| Static Embedding | rust-embed |
| Frontend | Vue 3 + Element Plus + Vue Router + Axios |
| Build Tool | Vite 5 + TypeScript |

## API Reference

### Public Routes (No Auth)

| Endpoint | Method | Description |
|----------|--------|-------------|
| `/auth-admin/login` | POST | Login with username & password |
| `/auth-admin/logout` | POST | Logout |
| `/auth-admin/refresh` | POST | Refresh JWT token |
| `/auth-admin/ui/` | GET | Admin Web UI |

### Protected Routes (JWT + Casbin)

#### Auth

| Endpoint | Method | Description |
|----------|--------|-------------|
| `/auth-admin/me` | GET | Get current user info |
| `/auth-admin/me/password` | PUT | Change own password |

#### Users

| Endpoint | Method | Description |
|----------|--------|-------------|
| `/auth-admin/users` | GET | List users (paginated) |
| `/auth-admin/users` | POST | Create user |
| `/auth-admin/users/{id}` | GET | Get user detail |
| `/auth-admin/users/{id}` | PUT | Update user |
| `/auth-admin/users/{id}` | DELETE | Delete user |
| `/auth-admin/users/{id}/status` | PUT | Toggle user status |
| `/auth-admin/users/{id}/reset-password` | PUT | Reset user password |
| `/auth-admin/users/{id}/roles` | GET | Get user's roles |
| `/auth-admin/users/{id}/roles` | POST | Assign role to user |
| `/auth-admin/users/{id}/roles/{role_id}` | DELETE | Revoke role from user |
| `/auth-admin/users/{id}/permissions` | GET | Get user's permissions |
| `/auth-admin/users/batch-delete` | POST | Batch delete users |

#### Roles

| Endpoint | Method | Description |
|----------|--------|-------------|
| `/auth-admin/roles` | GET | List roles |
| `/auth-admin/roles` | POST | Create role |
| `/auth-admin/roles/{id}` | GET | Get role detail |
| `/auth-admin/roles/{id}` | PUT | Update role |
| `/auth-admin/roles/{id}` | DELETE | Delete role |
| `/auth-admin/roles/{id}/users` | GET | List users under role |
| `/auth-admin/roles/{id}/permissions` | GET | Get role's permissions |
| `/auth-admin/roles/{id}/permissions` | POST | Assign permission to role |
| `/auth-admin/roles/{id}/permissions/{perm_id}` | DELETE | Revoke permission from role |

#### Permissions

| Endpoint | Method | Description |
|----------|--------|-------------|
| `/auth-admin/permissions` | GET | List permissions |
| `/auth-admin/permissions` | POST | Create permission |
| `/auth-admin/permissions/{id}` | GET | Get permission detail |
| `/auth-admin/permissions/{id}` | PUT | Update permission |
| `/auth-admin/permissions/{id}` | DELETE | Delete permission |

#### Departments

| Endpoint | Method | Description |
|----------|--------|-------------|
| `/auth-admin/departments` | GET | List departments |
| `/auth-admin/departments` | POST | Create department |
| `/auth-admin/departments/{id}` | GET | Get department detail |
| `/auth-admin/departments/{id}` | PUT | Update department |
| `/auth-admin/departments/{id}` | DELETE | Delete department |
| `/auth-admin/departments/{id}/move/{parent_id}` | PUT | Move department |
| `/auth-admin/departments/{id}/users` | GET | List users in department |

#### Applications

| Endpoint | Method | Description |
|----------|--------|-------------|
| `/auth-admin/apps` | GET | List registered applications |
| `/auth-admin/apps` | POST | Register application |
| `/auth-admin/apps/{id}` | GET | Get application detail |
| `/auth-admin/apps/{id}` | PUT | Update application |
| `/auth-admin/apps/{id}` | DELETE | Delete application |

#### App Proxy (Forward to Target Microservice)

| Endpoint | Method | Description |
|----------|--------|-------------|
| `/auth-admin/apps/{id}/schemas` | GET | Proxy: list target app's API schemas |
| `/auth-admin/apps/{id}/policies` | GET | Proxy: list target app's Casbin policies |
| `/auth-admin/apps/{id}/policies` | POST | Proxy: add policy to target app |
| `/auth-admin/apps/{id}/policies/{policy_id}` | DELETE | Proxy: remove policy from target app |
| `/auth-admin/apps/{id}/roles` | GET | Proxy: list target app's role mappings |
| `/auth-admin/apps/{id}/roles` | POST | Proxy: add role mapping to target app |
| `/auth-admin/apps/{id}/roles/{role_id}` | DELETE | Proxy: remove role mapping from target app |
| `/auth-admin/apps/{id}/groups` | GET | Proxy: list target app's groups |
| `/auth-admin/apps/{id}/groups` | POST | Proxy: add group to target app |
| `/auth-admin/apps/{id}/groups/{group_id}` | DELETE | Proxy: remove group from target app |
| `/auth-admin/apps/{id}/reload` | POST | Proxy: reload target app's Enforcer |

## Database Tables

Auto-created via Flyway migrations:

| Table | Description |
|-------|-------------|
| `auth_admin_users` | Admin user accounts |
| `auth_admin_roles` | Role definitions |
| `auth_admin_permissions` | Permission items |
| `auth_admin_departments` | Department / organization tree |
| `auth_admin_user_roles` | User-role associations |
| `auth_admin_role_permissions` | Role-permission associations |
| `auth_admin_applications` | Registered microservice applications |
| `message` | Dapr message outbox |

Tables from `genies_auth` migrations (created first):

| Table | Description |
|-------|-------------|
| `casbin_rules` | Casbin policy rules |
| `casbin_model` | Casbin model definition |
| `auth_api_schemas` | API schema metadata |

## Configuration

Key fields in `application.yml`:

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

## Getting Started

### Prerequisites

- Rust 1.75+
- MySQL 5.7+ / 8.0
- Redis
- (Optional) Dapr runtime for event-driven sync

### Run the Service

```bash
cargo run -p genies_auth_admin
```

The service starts at `http://127.0.0.1:9099`.

### Access the Web UI

Open your browser and navigate to:

```
http://127.0.0.1:9099/auth-admin/ui/
```

> **Note**: The trailing slash `/` is required for the SPA to load correctly.

### Build the Frontend (Development)

```bash
cd crates/auth-admin/web
npm install
npm run dev      # Dev server with hot reload
npm run build    # Production build → ../static/
```

## Relationship with genies_auth

| Crate | Role |
|-------|------|
| `genies_auth` | **Permission Engine Library** — Casbin Enforcer, middleware (`casbin_auth`), field-level filtering (`#[casbin]` macro), Admin API for policy CRUD, OpenAPI schema sync |
| `genies_auth_admin` | **Management Backend** — User / role / permission / department / app CRUD, local JWT login, Web UI, multi-app proxy; depends on `genies_auth` for authentication and authorization |

`genies_auth_admin` uses `genies_auth` as its authentication and authorization backbone: all protected routes pass through `genies_auth`'s `local_auth` (JWT verification) and `casbin_auth` (RBAC check) middleware.

## License

See the project root for license information.
