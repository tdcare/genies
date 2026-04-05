# Genies Project RBatis Patterns

## Multi-Database Driver Factory

The Genies project uses a driver factory pattern with Cargo feature flags for compile-time database selection.

### Feature Flags (crates/context/Cargo.toml)

```toml
[features]
default = ["mysql"]
mysql = ["rbdc-mysql"]
postgres = ["rbdc-pg"]
sqlite = ["rbdc-sqlite"]
mssql = ["rbdc-mssql"]
oracle = ["rbdc-oracle"]
tdengine = ["rbdc-tdengine"]
all-db = ["mysql", "postgres", "sqlite", "mssql", "oracle", "tdengine"]
```

### Driver Factory Function

```rust
fn create_db_driver(url: &str) -> Box<dyn rbdc::db::Driver> {
    let scheme = url.split("://").next().unwrap_or("");
    match scheme {
        #[cfg(feature = "mysql")]
        "mysql" => Box::new(rbdc_mysql::driver::MysqlDriver {}),
        #[cfg(feature = "postgres")]
        "postgres" | "postgresql" => Box::new(rbdc_pg::driver::PgDriver {}),
        #[cfg(feature = "sqlite")]
        "sqlite" => Box::new(rbdc_sqlite::driver::SqliteDriver {}),
        #[cfg(feature = "mssql")]
        "mssql" | "sqlserver" => Box::new(rbdc_mssql::driver::MssqlDriver {}),
        #[cfg(feature = "oracle")]
        "oracle" => Box::new(rbdc_oracle::driver::OracleDriver {}),
        #[cfg(feature = "tdengine")]
        "taos" | "taos+ws" => Box::new(rbdc_tdengine::driver::TaosDriver {}),
        _ => panic!("Unsupported database scheme: '{}'. Enable the corresponding Cargo feature.", scheme),
    }
}
```

## Global Context Singleton

```rust
// crates/context/src/lib.rs
lazy_static! {
    pub static ref CONTEXT: ApplicationContext = ApplicationContext::default();
}

// Initialize at startup
CONTEXT.init_database().await;

// Use in business code
let rb = &CONTEXT.rbatis;
let data = Activity::select_by_map(rb, value!{"id": "1"}).await?;
```

## Idempotent Database Initialization

```rust
// crates/context/src/app_context.rs
pub async fn init_database(&self) {
    self.db_init_once.call_once(|| {
        let driver = create_db_driver(&self.config.database_url);
        let _ = self.rbatis.init(driver, &self.config.database_url).unwrap();
        let pool = self.rbatis.get_pool().unwrap();
        pool.set_max_open_conns(self.config.max_connections as u64);
        pool.set_max_idle_conns(self.config.wait_timeout as u64);
        pool.set_conn_max_lifetime(Some(Duration::from_secs(self.config.max_lifetime)));
    });
    let _ = self.rbatis.get_pool().unwrap().get().await;
}
```

## Database Configuration (application.yml)

```yaml
database_url: "mysql://root:password@127.0.0.1:3306/my_service"
max_connections: 20
min_connections: 0
wait_timeout: 60
max_lifetime: 1800
create_timeout: 120
```

## Query Pattern: Direct SQL with rbs Deserialization

```rust
// SELECT with rbs::from_value deserialization
let sql = "SELECT * FROM my_table WHERE status = ?";
let value = CONTEXT.rbatis.query(sql, vec![value!(1)]).await?;
let rows: Vec<MyStruct> = rbs::from_value(value)?;

// EXEC with ExecResult
let sql = "DELETE FROM my_table WHERE id = ?";
let result = CONTEXT.rbatis.exec(sql, vec![value!(id)]).await?;
// result.rows_affected
```

## Custom Method Pattern with Executor Trait

```rust
impl Message {
    pub async fn select_by_status(
        executor: &dyn Executor,
        status: u32,
        limit: u64,
    ) -> Result<Vec<Message>, Error> {
        let sql = format!(
            "SELECT * FROM message WHERE published = {} LIMIT {}",
            status, limit
        );
        let value = executor.query(&sql, vec![]).await?;
        rbatis::decode(value)
    }

    pub async fn update_status(
        executor: &dyn Executor,
        id: String,
        status: u32,
    ) -> Result<ExecResult, Error> {
        let sql = "UPDATE message SET published = ? WHERE id = ?";
        executor.exec(sql, vec![
            Value::U32(status),
            Value::String(id),
        ]).await
    }
}
```

## Multi-Pool Management (CDC Module)

```rust
use std::sync::LazyLock;
use std::collections::HashMap;

pub static POOLS: LazyLock<HashMap<String, RBatis>> = LazyLock::new(|| {
    make_pools(&CONTEXT.config.cdc_configs)
        .expect("Failed to initialize database pools")
});

fn make_pools(configs: &HashMap<String, CdcConfig>) -> Result<HashMap<String, RBatis>> {
    let mut pools = HashMap::new();
    for (name, config) in configs {
        let rb = RBatis::new();
        rb.init(MysqlDriver {}, config.database_url.as_ref().unwrap())?;
        pools.insert(name.clone(), rb);
    }
    Ok(pools)
}
```

## Key File Locations

| Purpose | Path |
|---------|------|
| Global context | `crates/context/src/lib.rs` |
| Database init | `crates/context/src/app_context.rs` |
| Casbin adapter | `crates/auth/src/adapter.rs` |
| Admin API (query/exec examples) | `crates/auth/src/admin_api.rs` |
| Message model (custom SQL) | `crates/cdc/src/message.rs` |
| Multi-pool config | `crates/cdc/src/config/mod.rs` |
| App config | `application.yml` |
| DB migrations | `crates/auth/migrations/` |
