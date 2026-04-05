---
name: rbatis-usage
description: Guide for using RBatis v4 Rust ORM framework. Use when implementing database CRUD operations, writing dynamic SQL with py_sql or html_sql macros, configuring database connections, managing transactions, implementing interceptors, syncing table structures, or integrating RBatis into Genies microservices. Also use when the user asks about RBatis usage patterns, query building, or database operations in Rust.
---

# RBatis v4 ORM Usage Guide

RBatis is a high-performance compile-time ORM framework for Rust. It compiles dynamic SQL to native Rust code at compile time, achieving zero runtime overhead.

## Dependencies (Cargo.toml)

```toml
[dependencies]
rbatis = { version = "4.8", features = ["debug_mode"] }  # Remove debug_mode in production
rbs = { version = "4.7" }
rbdc = { version = "4.7", default-features = false }

# Choose your database driver(s):
rbdc-mysql = { version = "4.7" }
# rbdc-pg = { version = "4.7" }
# rbdc-sqlite = { version = "4.7" }
# rbdc-mssql = { version = "4.7" }

serde = { version = "1", features = ["derive"] }
tokio = { version = "1", features = ["full"] }
```

## Initialization

```rust
use rbatis::RBatis;

let rb = RBatis::new();

// MySQL
rb.init(rbdc_mysql::driver::MysqlDriver {}, "mysql://root:123456@localhost:3306/test").unwrap();
// PostgreSQL
rb.init(rbdc_pg::driver::PgDriver {}, "postgres://postgres:123456@localhost:5432/postgres").unwrap();
// SQLite
rb.init(rbdc_sqlite::driver::SqliteDriver {}, "sqlite://target/sqlite.db").unwrap();
// MSSQL
rb.init(rbdc_mssql::driver::MssqlDriver {}, "mssql://jdbc:sqlserver://localhost:1433;User=SA;Password={Pass};Database=master;").unwrap();
```

### Connection Pool Configuration

```rust
let pool = rb.get_pool().unwrap();
pool.set_max_open_conns(20);
pool.set_max_idle_conns(10);
pool.set_conn_max_lifetime(Some(Duration::from_secs(1800)));
```

## Table Model Definition

```rust
use rbatis::rbdc::datetime::DateTime;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub struct Activity {
    pub id: Option<String>,
    pub name: Option<String>,
    pub status: Option<i32>,
    pub create_time: Option<DateTime>,
}
```

**Supported types**: `Option<T>`, `Vec<T>`, `HashMap`, all Rust primitives, `rbatis::rbdc::types::{DateTime, Date, Time, Timestamp, Decimal, Bytes, Json}`, `serde_json::Value`, `rbs::Value`.

## CRUD Macro (Auto-Generated Methods)

```rust
use rbatis::crud;
use rbs::value;

// Generate: insert, insert_batch, select_by_map, update_by_map, delete_by_map
crud!(Activity {});
// With custom table name:
// crud!(Activity {}, "my_table");
```

### Insert

```rust
let activity = Activity { id: Some("1".into()), name: Some("Test".into()), ..Default::default() };
Activity::insert(&rb, &activity).await?;

// Batch insert (batch_size = 10)
let items = vec![activity1, activity2];
Activity::insert_batch(&rb, &items, 10).await?;
```

### Select

```rust
// By exact match
let data = Activity::select_by_map(&rb, value!{"id": "1"}).await?;
// Multiple conditions
let data = Activity::select_by_map(&rb, value!{"id": "1", "name": "Test"}).await?;
// LIKE
let data = Activity::select_by_map(&rb, value!{"name like ": "%Test%"}).await?;
// Greater than
let data = Activity::select_by_map(&rb, value!{"id > ": "2"}).await?;
// IN
let data = Activity::select_by_map(&rb, value!{"id": &["1", "2", "3"]}).await?;
```

### Update

```rust
// Update all fields by condition
Activity::update_by_map(&rb, &activity, value!{"id": "1"}).await?;
// Update specific columns only
Activity::update_by_map(&rb, &activity, value!{"id": "1", "column": ["name", "status"]}).await?;
```

### Delete

```rust
Activity::delete_by_map(&rb, value!{"id": "1"}).await?;
Activity::delete_by_map(&rb, value!{"id": &["1", "2", "3"]}).await?;
```

## Raw SQL with `#[sql]` Macro

For static SQL with `?` parameter placeholders:

```rust
use rbatis::sql;

#[sql("select * from activity where delete_flag = ? and status = ?")]
async fn select_by_flag(rb: &RBatis, delete_flag: &i32, status: &i32) -> rbatis::Result<Vec<Activity>> {
    impled!()
}

let result = select_by_flag(&rb, &0, &1).await?;
```

## Dynamic SQL with `#[py_sql]` Macro

Python-style conditional SQL with `if`, `for`, `choose/when/otherwise`, `trim`, `bind`:

```rust
use rbatis::py_sql;
use rbatis::executor::Executor;
use rbatis::rbdc::ExecResult;

#[py_sql(
    "`select * from activity where delete_flag = 0`
      if name != '':
        ` and name = #{name}`
      if !ids.is_empty():
        ` and id in `
        ${ids.sql()}"
)]
async fn py_select(rb: &dyn Executor, name: &str, ids: &[i32]) -> rbatis::Result<Vec<Activity>> {
    impled!()
}

// Functional macro form (alternative):
rbatis::pysql!(py_select2(rb: &dyn Executor, name: &str) -> Result<Vec<Activity>, rbatis::Error> =>
    "`select * from activity`
      if name != '':
        ` and name = #{name}`");
```

**Placeholder rules**:
- `#{arg}` — precompiled parameter (safe, prevents SQL injection)
- `${arg}` — direct string replacement (use for dynamic SQL fragments like `ids.sql()`)
- `.sql()` — converts a collection to SQL `(1, 2, 3)` format
- `.csv()` — converts a collection to comma-separated values

## Dynamic SQL with `#[html_sql]` Macro

MyBatis-style XML template SQL with `<if>`, `<where>`, `<set>`, `<foreach>`, `<choose>`, `<trim>`:

```rust
use rbatis::html_sql;
use rbatis::executor::Executor;

// Inline HTML SQL
#[html_sql(
    r#"<select id="select_by_condition">
        `select * from activity`
        <where>
            <if test="name != ''">
                ` and name like #{name}`
            </if>
            <if test="status != null">
                ` and status = #{status}`
            </if>
            <choose>
                <when test="id != ''">
                    ` and id = #{id}`
                </when>
                <otherwise>
                    ` and id != '-1'`
                </otherwise>
            </choose>
        </where>
    </select>"#
)]
async fn select_by_condition(
    rb: &dyn Executor,
    name: &str,
    status: &Option<i32>,
    id: &str,
) -> rbatis::Result<Vec<Activity>> {
    impled!()
}

// Or load from file:
// #[html_sql("example/example.html")]
```

### HTML SQL Tags Reference

| Tag | Purpose | Example |
|-----|---------|---------|
| `<if test="expr">` | Conditional include | `<if test="name != ''">` |
| `<where>` | Auto `WHERE` + strip leading `AND/OR` | `<where>...</where>` |
| `<set>` | Auto `SET` + strip trailing commas | `<set>...</set>` |
| `<choose>/<when>/<otherwise>` | Switch-case logic | See example above |
| `<foreach collection="ids" item="id" separator=",">` | Loop collection | `#{id}` |
| `<trim prefix="" suffix="" prefixOverrides="" suffixOverrides="">` | Trim SQL fragments | Strip extra `AND` |
| `<bind name="pattern" value="'%' + name + '%'"/>` | Bind variable | For LIKE patterns |

### Pagination with html_sql

```rust
use rbatis::plugin::page::{Page, PageRequest};

#[html_sql(
    r#"<select id="select_page">
        `select * from activity`
        <where>
            <if test="name != ''">` and name like #{name}`</if>
        </where>
    </select>"#
)]
async fn select_page(
    rb: &dyn Executor,
    page_req: &PageRequest,
    name: &str,
) -> rbatis::Result<Page<Activity>> {
    impled!()
}

// Usage:
let page_req = PageRequest::new(1, 10); // page 1, size 10
let page: Page<Activity> = select_page(&rb, &page_req, "test").await?;
// page.records, page.total, page.page_no, page.page_size
```

## Direct Query/Exec Methods

For maximum flexibility, use `rb.query()` and `rb.exec()` directly:

```rust
// SELECT — returns rbs::Value, decode with rbs::from_value
let sql = "SELECT * FROM activity WHERE status = ?";
let value = rb.query(sql, vec![rbs::to_value!(1)]).await?;
let rows: Vec<Activity> = rbs::from_value(value)?;

// INSERT/UPDATE/DELETE — returns ExecResult
let sql = "DELETE FROM activity WHERE id = ?";
let result = rb.exec(sql, vec![rbs::to_value!("1")]).await?;
// result.rows_affected, result.last_insert_id

// Using value! macro for parameters
let value = rb.query("SELECT * FROM activity WHERE name = ?", vec![value!("test")]).await?;
```

## Transactions

```rust
use rbatis::executor::RBatisTxExecutor;

// Method 1: acquire_begin (auto-rollback on drop)
let tx = rb.acquire_begin().await?;
let tx = tx.defer_async(|tx| async move {
    if !tx.done() {
        let _ = tx.rollback().await;
    }
});
Activity::insert(&tx, &activity).await?;
tx.commit().await?;

// Method 2: From connection
let conn = rb.acquire().await?;
let tx = conn.begin().await?;
Activity::insert(&tx, &activity).await?;
tx.commit().await?;
```

## Interceptor Plugin

```rust
use rbatis::intercept::{Intercept, ResultType};
use rbatis::{async_trait, Error, Action};
use rbatis::executor::Executor;
use rbatis::rbdc::ExecResult;
use rbs::Value;
use std::sync::Arc;

#[derive(Debug)]
pub struct MyIntercept;

#[async_trait]
impl Intercept for MyIntercept {
    async fn before(
        &self,
        _task_id: i64,
        _rb: &dyn Executor,
        sql: &mut String,
        args: &mut Vec<Value>,
        _result: ResultType<&mut Result<ExecResult, Error>, &mut Result<Value, Error>>,
    ) -> Result<Action, Error> {
        log::info!("SQL: {} | Args: {:?}", sql, args);
        Ok(Action::Next) // Continue execution; use Action::Return to skip
    }

    async fn after(
        &self,
        _task_id: i64,
        _rb: &dyn Executor,
        _sql: &mut String,
        _args: &mut Vec<Value>,
        _result: ResultType<&mut Result<ExecResult, Error>, &mut Result<Value, Error>>,
    ) -> Result<Action, Error> {
        Ok(Action::Next)
    }
}

// Register interceptor
rb.intercepts.push(Arc::new(MyIntercept {}));
// Retrieve interceptor
let intercept = rb.get_intercept::<MyIntercept>();
```

## Table Sync Plugin

Auto-create/update table structures from Rust structs:

```rust
use rbatis::table_sync;

// Choose mapper for your database:
let mapper = &table_sync::MysqlTableMapper {};
// let mapper = &table_sync::PGTableMapper {};
// let mapper = &table_sync::SqliteTableMapper {};
// let mapper = &table_sync::MssqlTableMapper {};

// Sync from struct instance (field types inferred from Rust types)
RBatis::sync(
    &rb.acquire().await?,
    mapper,
    &Activity {
        id: Some(String::new()),
        name: Some(String::new()),
        status: Some(0),
        create_time: Some(DateTime::now()),
    },
    "activity",  // table name
).await?;

// Or sync from explicit column types using value! map
RBatis::sync(
    &rb.acquire().await?,
    mapper,
    &value!{
        "id": "VARCHAR(64)",
        "name": "VARCHAR(256)",
        "status": "INT",
        "create_time": "DATETIME"
    },
    "activity",
).await?;
```

## Genies Project Patterns

For Genies project-specific patterns, see [genies-patterns.md](genies-patterns.md).

## Supported Databases

| Database | Driver Crate | URL Format |
|----------|-------------|------------|
| MySQL/MariaDB/TiDB | `rbdc-mysql` | `mysql://user:pass@host:3306/db` |
| PostgreSQL/CockroachDB | `rbdc-pg` | `postgres://user:pass@host:5432/db` |
| SQLite | `rbdc-sqlite` | `sqlite://path/to/db.sqlite` |
| MSSQL | `rbdc-mssql` | `mssql://...` |
| Oracle | `rbdc-oracle` | `oracle://user:pass@host:1521/db` |
| TDengine | `rbdc-tdengine` | `taos+ws://host:6041/db` |

## Key Utility Macros

| Macro | Purpose |
|-------|---------|
| `crud!(Struct{})` | Generate insert/insert_batch/select_by_map/update_by_map/delete_by_map |
| `#[sql("...")]` | Static raw SQL binding |
| `#[py_sql("...")]` | Python-style dynamic SQL |
| `pysql!(fn_name(...) => "...")` | Functional py_sql |
| `#[html_sql("...")]` | MyBatis-style XML dynamic SQL |
| `htmlsql!(fn_name(...) => "...")` | Functional html_sql |
| `htmlsql_select_page!()` | html_sql with pagination |
| `pysql_select_page!()` | py_sql with pagination |
| `value!{...}` | Create `rbs::Value` for parameters |
| `impled!()` | Placeholder body for macro-generated functions |
| `field_name!(Struct.field)` | Get field name as `&str` |
| `table!{}` | Construct struct with Default trait |
