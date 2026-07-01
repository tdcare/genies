---
name: flyway-usage
description: Guide for using flyway-rs database migration framework with RBatis. Use when implementing database schema migrations, managing SQL changelog files, configuring migration runners, handling multi-database migrations, or integrating database versioning into Genies microservices.
---

# flyway-rs Database Migration Framework

## Overview

flyway-rs 是一个 Rust 数据库迁移框架（v0.6），基于 RBatis 驱动，支持 MySQL、PostgreSQL、SQLite、MSSQL、TDengine。

> **优先级**：推荐在**运行时读取 SQL 文件**（无需重新编译即可更新迁移脚本），避免使用 `#[migrations]` 宏将 SQL 编译进二进制。

**核心 Crates：**
- `flyway` — 主框架：MigrationRunner + 核心 trait
- `flyway-rbatis` — RBatis 数据库驱动实现
- `flyway-codegen` — `#[migrations]` 过程宏
- `flyway-sql-changelog` — SQL 文件解析与语句分割

## Quick Start

### 1. 添加依赖

```toml
[dependencies]
flyway = "0.6"
flyway-rbatis = "0.6"
rbatis = { version = "4.8", features = ["debug_mode"] }
rbdc-mysql = "4.5"        # MySQL 驱动（按需选择）
# rbdc-pg = "4.5"         # PostgreSQL 驱动
# rbdc-tdengine = "4.5"   # TDengine 驱动
```

### 2. 创建 SQL 迁移文件

文件命名格式：`V<version>__<description>.sql`，放在 migrations 目录下。

```
migrations/
├── V1__create_users.sql
├── V2__create_orders.sql
└── V3__add_indexes.sql
```

**SQL 文件示例** (`V1__create_users.sql`)：
```sql
CREATE TABLE IF NOT EXISTS users (
    id BIGINT PRIMARY KEY,
    username VARCHAR(255) NOT NULL,
    email VARCHAR(255),
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
);
```

### 3. 编写迁移代码（推荐：运行时加载）

> 使用 `ChangelogFile::from_path()` 在运行时扫描目录，SQL 文件变更无需重新编译。

```rust
use std::path::Path;
use std::sync::Arc;
use flyway::{ChangelogFile, MigrationRunner, MigrationsError, MigrationStore};
use flyway_rbatis::RbatisMigrationDriver;
use rbatis::RBatis;

/// 运行时迁移存储：启动时从目录动态加载所有 V*.sql 文件
pub struct RuntimeMigrationStore {
    changelogs: Vec<ChangelogFile>,
}

impl RuntimeMigrationStore {
    /// 从指定目录扫描所有 .sql 文件
    pub fn from_dir(dir: &str) -> Result<Self, MigrationsError> {
        let dir_path = Path::new(dir);
        let mut entries: Vec<_> = std::fs::read_dir(dir_path)
            .map_err(|e| MigrationsError::custom_error(e.to_string(), None))?
            .filter_map(|e| e.ok())
            .filter(|e| e.path().extension().map_or(false, |ext| ext == "sql"))
            .collect();
        // 按文件名排序保证版本顺序
        entries.sort_by_key(|e| e.file_name());

        let changelogs = entries
            .iter()
            .filter_map(|e| ChangelogFile::from_path(&e.path()).ok())
            .collect();

        Ok(Self { changelogs })
    }
}

impl MigrationStore for RuntimeMigrationStore {
    fn changelogs(&self) -> Vec<ChangelogFile> {
        self.changelogs.clone()
    }
}

pub async fn run_migrations(rb: Arc<RBatis>) -> Result<(), MigrationsError> {
    let driver = Arc::new(RbatisMigrationDriver::new(rb, None));
    let store = RuntimeMigrationStore::from_dir("migrations")?;
    let runner = MigrationRunner::new(
        store,
        driver.clone(),
        driver.clone(),
        false,
    );
    runner.migrate().await?;
    Ok(())
}
```

### 4. 备选：编译时嵌入（不推荐）

仅当必须将 SQL 编译进二进制（如嵌入式场景）时才使用 `#[migrations]` 宏：

```rust
use flyway::migrations;

// 编译时将 SQL 内容嵌入二进制，每次修改 SQL 都需重新编译
#[migrations("migrations")]
pub struct Migrations {}
```

> **权衡**：编译时嵌入会增大二进制体积，且任何 SQL 变更都需重新编译和发布整个服务。运行时加载只需重启服务即可生效。

## Core API

### MigrationRunner

主要执行器，泛型参数：`S: MigrationStore, M: MigrationStateManager, E: MigrationExecutor`

```rust
// 创建
let runner = MigrationRunner::new(store, state_manager, executor, fail_continue);

// 执行迁移（返回最高已部署版本号）
let version: Option<u32> = runner.migrate().await?;
```

**`fail_continue` 参数：**
- `false` — 遇到 SQL 执行错误立即停止（推荐生产环境）
- `true` — 跳过失败的 changelog（标记为 skipped），继续执行后续文件

### #[migrations] 宏（编译时嵌入，不推荐）

编译时读取 SQL 文件并嵌入到二进制中。**优先使用运行时加载**，仅在嵌入式等特殊场景使用此宏：

```rust
#[migrations("path/to/migrations")]  // 相对于 CARGO_MANIFEST_DIR
pub struct MyMigrations {}

// 自动生成：
// impl MigrationStore for MyMigrations {
//     fn changelogs(&self) -> Vec<ChangelogFile> { ... }
// }
```

### RbatisMigrationDriver

同时实现 `MigrationStateManager` 和 `MigrationExecutor`：

```rust
use flyway_rbatis::RbatisMigrationDriver;

// 使用默认表名 "flyway_migrations"
let driver = RbatisMigrationDriver::new(rb, None);

// 自定义迁移历史表名
let driver = RbatisMigrationDriver::new(rb, Some("custom_migrations"));

// 获取数据库驱动类型
let db_type = driver.driver_type()?;
```

### MigrationStateManager Trait

```rust
#[async_trait]
pub trait MigrationStateManager {
    async fn prepare(&self) -> Result<()>;                              // 初始化迁移表
    async fn lowest_version(&self) -> Result<Option<MigrationState>>;   // 最低已部署版本
    async fn highest_version(&self) -> Result<Option<MigrationState>>;  // 最高已部署版本
    async fn list_versions(&self) -> Result<Vec<MigrationState>>;       // 所有已部署版本
    async fn begin_version(&self, cf: &ChangelogFile) -> Result<()>;    // 开始版本迁移
    async fn finish_version(&self, cf: &ChangelogFile) -> Result<()>;   // 完成版本迁移
    async fn skip_version(&self, cf: &ChangelogFile) -> Result<()>;     // 跳过失败版本
}
```

### MigrationExecutor Trait

```rust
#[async_trait]
pub trait MigrationExecutor {
    async fn begin_transaction(&self) -> Result<()>;
    async fn execute_changelog_file(&self, cf: &ChangelogFile) -> Result<()>;
    async fn commit_transaction(&self) -> Result<()>;
    async fn rollback_transaction(&self) -> Result<()>;
}
```

### 错误处理

```rust
pub enum MigrationsErrorKind {
    MigrationDatabaseStepFailed(Option<Box<dyn Error + Send + Sync>>),
    MigrationDatabaseFailed(Option<Box<dyn Error + Send + Sync>>),
    MigrationSetupFailed(Option<Box<dyn Error + Send + Sync>>),
    MigrationVersioningFailed(Option<Box<dyn Error + Send + Sync>>),
    CustomErrorMessage(String, Option<Box<dyn Error + Send + Sync>>),
}

// 获取错误信息
match result {
    Err(e) => {
        let kind = e.kind();
        let last_version = e.last_successful_version();
        log::error!("Migration failed: {}, last ok version: {:?}", e, last_version);
    }
    Ok(_) => log::info!("Migration success"),
}
```

## SQL Changelog Features

### 语句分割

SQL 文件中的语句以 `;` 分隔，支持：
- 单引号/双引号/反引号字符串（含转义）
- `--` 行注释
- 多条语句按顺序执行

### 语句注解（实验性）

SQL 注释中可使用 `--!` 前缀 + YAML 格式添加注解：

```sql
--! may_fail: true
CREATE INDEX idx_users_email ON users(email);
```

`may_fail: true` — 标记此语句允许失败。执行失败时跳过并记录警告，不中断迁移流程。

### ChangelogFile API

```rust
use flyway::ChangelogFile;

// 从文件加载
let cf = ChangelogFile::from_path(Path::new("V1__init.sql"))?;

// 从字符串创建
let cf = ChangelogFile::from_string(1, "init", "CREATE TABLE ...")?;

// 属性
cf.version();   // u64 版本号
cf.content();   // SQL 内容
cf.checksum;    // u64 校验和（SipHash）
cf.name;        // 文件名描述

// 迭代 SQL 语句
for stmt in cf.iter() {
    println!("SQL: {}", stmt.statement);
    if let Some(ann) = &stmt.annotation {
        // 处理注解
    }
}
```

## Multi-Database Migrations

### 按数据库类型分目录（推荐）

```
migrations/
├── mysql/
│   ├── V1__create_tables.sql
│   └── V2__add_indexes.sql
├── postgres/
│   ├── V1__create_tables.sql
│   └── V2__add_indexes.sql
└── taos/
    └── V1__create_tables.sql
```

### 运行时自动派发

```rust
use flyway_rbatis::{RbatisMigrationDriver, RbatisDbDriverType};

pub async fn migrate(rb: Arc<RBatis>) -> Result<(), MigrationsError> {
    let driver = Arc::new(RbatisMigrationDriver::new(rb.clone(), None));
    let db_type = driver.driver_type()
        .map_err(|e| MigrationsError::migration_database_failed(None, Some(e.into())))?;

    // 根据数据库类型选择对应的迁移目录（运行时加载）
    let store = match db_type {
        RbatisDbDriverType::MySql => RuntimeMigrationStore::from_dir("migrations/mysql/")?,
        RbatisDbDriverType::Pg => RuntimeMigrationStore::from_dir("migrations/postgres/")?,
        RbatisDbDriverType::TDengine => RuntimeMigrationStore::from_dir("migrations/taos/")?,
        _ => return Err(MigrationsError::migration_setup_failed(None)),
    };

    MigrationRunner::new(store, driver.clone(), driver.clone(), true)
        .migrate().await.map(|_| ())
}
```

### 支持的数据库类型

```rust
pub enum RbatisDbDriverType {
    MySql,
    Pg,
    Sqlite,
    MsSql,
    TDengine,
    Other(String),
}
```

## Genies Framework Integration

在 Genies 项目中使用 flyway-rs 的标准模式：

### Cargo.toml 配置

```toml
# workspace Cargo.toml
[workspace.dependencies]
flyway = "0.6.0"
flyway-rbatis = "0.6.0"

# crate Cargo.toml
[dependencies]
flyway = { workspace = true }
flyway-rbatis = { workspace = true }
```

### 标准迁移模块（推荐：运行时加载）

```rust
//! 数据库迁移模块
use std::sync::Arc;
use flyway::{MigrationRunner, ChangelogFile, MigrationStore, MigrationsError};
use flyway_rbatis::RbatisMigrationDriver;
use genies::context::CONTEXT;

pub struct RuntimeMigrationStore {
    changelogs: Vec<ChangelogFile>,
}

impl RuntimeMigrationStore {
    pub fn from_dir(dir: &str) -> Result<Self, MigrationsError> {
        use std::path::Path;
        let dir_path = Path::new(dir);
        let mut entries: Vec<_> = std::fs::read_dir(dir_path)
            .map_err(|e| MigrationsError::custom_error(e.to_string(), None))?
            .filter_map(|e| e.ok())
            .filter(|e| e.path().extension().map_or(false, |ext| ext == "sql"))
            .collect();
        entries.sort_by_key(|e| e.file_name());
        let changelogs = entries.iter()
            .filter_map(|e| ChangelogFile::from_path(&e.path()).ok())
            .collect();
        Ok(Self { changelogs })
    }
}

impl MigrationStore for RuntimeMigrationStore {
    fn changelogs(&self) -> Vec<ChangelogFile> {
        self.changelogs.clone()
    }
}

/// 在应用启动时调用，运行时加载 migrations/ 目录下的 SQL 文件
/// 修改 SQL 只需重启服务，无需重新编译
pub async fn run_migrations() {
    let rbatis = Arc::new(CONTEXT.rbatis.clone());
    let driver = Arc::new(RbatisMigrationDriver::new(rbatis, None));
    let store = RuntimeMigrationStore::from_dir("migrations").unwrap();
    let runner = MigrationRunner::new(store, driver.clone(), driver.clone(), false);
    runner.migrate().await.expect("数据库迁移失败");
}
```

### 备选：编译时嵌入（不推荐）

仅在嵌入式等特殊场景使用（每次 SQL 变更都需重新编译）：

```rust
use flyway::migrations;

#[migrations("migrations")]
pub struct Migrations {}

pub async fn run_migrations() {
    // ... 使用 Migrations {} 作为 store
}

### 迁移文件命名约定

```
migrations/
├── V1__create_casbin_rules.sql
├── V2__create_casbin_model.sql
├── V3__create_auth_api_schemas.sql
├── V4__seed_casbin_model_and_policies.sql
└── V5__add_description_fields.sql
```

**命名规则：**
- 格式：`V<版本号>_<描述>.sql`（解析时以第一个 `_` 为分隔符）
- Genies 项目约定使用双下划线 `V<版本号>__<描述>.sql`（兼容 Java Flyway 风格）
- 版本号必须为正整数，决定执行顺序
- 描述使用 snake_case
- 已部署的迁移文件**不可修改**（checksum 校验）

## Migration State Table

flyway-rs 自动创建 `flyway_migrations` 表来跟踪迁移状态：

| 字段 | 类型 | 说明 |
|------|------|------|
| version | INTEGER (PK) | 迁移版本号 |
| ts | VARCHAR(255) | 执行时间戳 |
| name | VARCHAR(255) | 迁移名称 |
| checksum | VARCHAR(255) | 内容校验和 |
| status | VARCHAR(16) | 状态：InProgress / deployed / skipped |

## Troubleshooting

| 问题 | 原因 | 解决 |
|------|------|------|
| "Migration setup failed" | 无法创建迁移状态表 | 检查数据库连接和权限 |
| "Migration step failed" | SQL 语句执行错误 | 检查 SQL 语法兼容性 |
| 运行时找不到迁移文件 | 工作目录不对 | 确保以 cargo 工作目录启动，或使用绝对路径 |
| 编译时找不到迁移文件 | 路径错误 | `#[migrations]` 路径相对于 `CARGO_MANIFEST_DIR` |
| 迁移被跳过 | 版本已标记为 deployed | 检查 `flyway_migrations` 表 |
| checksum 不匹配 | 已部署的迁移文件被修改 | 恢复原始文件或清理状态表 |
| 新增 SQL 文件未执行 | 编译时嵌入模式未重新编译 | 优先换用运行时加载，或重新 `cargo build` |

## Known Limitations

- 事务管理仅支持"每 changelog 一个事务"模式
- `last_successful_version` 在部分错误场景不准确
- 当前仅有 RBatis 驱动实现
- MSSQL 支持可能需要特殊语法处理
