# genies_test

Testing infrastructure and utilities for the Genies framework. Provides generic tools for Java/Rust API comparison testing, database snapshot diffing, and assertion helpers.

## Overview

genies_test provides reusable testing building blocks for cross-language service migration validation:

- **HTTP Client**: Pre-configured HTTP client with auth support
- **Database Utilities**: Snapshot, diff, and restore for verifying database side effects
- **Deep Diff**: Recursive JSON comparison with dynamic field filtering
- **Assertions**: Strict and tolerant diff assertion helpers
- **Mutation Testing**: End-to-end mutation comparison workflow with database verification

## Features

- **Java/Rust Comparison**: Side-by-side API response comparison between Java and Rust services
- **Database Snapshot & Diff**: Capture table state before/after operations and compute changes
- **Dynamic Field Filtering**: Automatically ignore non-deterministic fields (IDs, timestamps)
- **Global Mutation Lock**: Serialize mutation tests to prevent database conflicts
- **Environment Variable Override**: All configuration values can be overridden via environment variables
- **Zero Framework Dependency**: Does not depend on other Genies internal crates

## Module Reference

### config — HTTP Client Factory

Provides a pre-configured HTTP client with optional Bearer authentication.

| Function | Description |
|----------|-------------|
| `http_client()` | Create proxy-disabled HTTP client with optional Bearer auth via `TOKEN` env var |

### db — Database Test Utilities

Snapshot-based diff/restore workflow for verifying database side effects.

| Function / Type | Description |
|-----------------|-------------|
| `db_snapshot()` | Capture a snapshot of specified tables |
| `db_diff()` | Compute differences between two snapshots |
| `db_restore()` | Restore database state from a diff |
| `ChangeType` | Enum: `Insert`, `Delete`, `Update` |
| `DbChange` | Struct representing a single row change |
| `AffectedTable` | Struct describing a table to snapshot (name, PK, order, where clause) |

### diff — Deep Diff Utilities

Recursive JSON value comparison and field filtering.

```rust
use genies_test::diff::{deep_diff, filter_fields, filter_dynamic_diffs};

// Recursively compare two JSON values
let diffs = deep_diff("root", &json_a, &json_b);

// Remove specific fields from a JSON object
let filtered = filter_fields(&json_obj, &["id", "createTime"]);

// Filter out diffs caused by dynamic fields (id, timestamps, etc.)
let significant = filter_dynamic_diffs(&diffs);
```

### assertions — Assertion Helpers

Convenient wrappers for diff-based test assertions.

```rust
use genies_test::assertions::{assert_no_diffs, assert_no_significant_diffs};

// Strict: fail if any differences exist
assert_no_diffs("test_name", &diffs);

// Tolerant: fail only if non-dynamic differences exist
assert_no_significant_diffs("test_name", &diffs);
```

### mutation — Mutation Test Workflow

End-to-end comparison workflow for write operations with database verification.

```rust
use genies_test::mutation::{
    DB_MUTATION_LOCK, compare_db_changes, test_mutation_with_db_diff,
};

// Global mutex lock ensures mutation tests run serially
let _lock = DB_MUTATION_LOCK.lock().await;

// Compare two sets of DbChange vectors
compare_db_changes("operation", &java_changes, &rust_changes, &dynamic_fields);

// Full 8-step mutation comparison test
test_mutation_with_db_diff(
    &client, &rb, "operation_name",
    &java_url, &rust_url,
    Some(&request_body),
    &affected_tables,
    &dynamic_fields,
).await;
```

## Environment Variables

| Variable | Description | Default |
|----------|-------------|---------|
| `TOKEN` | Bearer authentication token | *(none)* |

> **Note**: Business-specific configuration (service URLs, database URLs, test IDs) should be defined in your project's test module. See the sickbed example for reference.

## Quick Start

### 1. Add Dependency

Use `cargo add` to add dependencies (automatically fetches the latest version):

```sh
cargo add genies_test
```

You can also manually add dependencies in `Cargo.toml`. Visit [crates.io](https://crates.io) for the latest versions.

### 2. Query Comparison Test

```rust
use genies_test::*;
use serde_json::Value;

// Define your project-specific URLs
fn java_base_url() -> String {
    std::env::var("JAVA_BASE_URL").unwrap_or_else(|_| "http://localhost:8080/api".to_string())
}
fn rust_base_url() -> String {
    std::env::var("RUST_BASE_URL").unwrap_or_else(|_| "http://localhost:8081/api".to_string())
}

#[tokio::test]
async fn compare_query() {
    let client = http_client();

    let java_resp: Value = client
        .get(format!("{}/list", java_base_url()))
        .send().await.unwrap()
        .json().await.unwrap();

    let rust_resp: Value = client
        .get(format!("{}/list", rust_base_url()))
        .send().await.unwrap()
        .json().await.unwrap();

    let diffs = deep_diff("api/list", &java_resp, &rust_resp);
    assert_no_diffs("api/list", &diffs);
}
```

### 3. Mutation Comparison Test with Database Diff

```rust
use genies_test::*;

#[tokio::test]
async fn compare_mutation() {
    let client = http_client();
    // Initialize your own RBatis connection
    let rb = rbatis::RBatis::new();
    rb.init(rbdc_mysql::MysqlDriver {}, "mysql://user:pass@localhost:3306/mydb").unwrap();

    let body = serde_json::json!({ "name": "test" });

    test_mutation_with_db_diff(
        &client, &rb, "create_entity",
        "http://localhost:8080/api/create",
        "http://localhost:8081/api/create",
        Some(&body),
        &[AffectedTable {
            table: "my_table",
            pk_field: "id",
            order_by: "id",
            where_clause: "name = 'test'".to_string(),
        }],
        &["id", "createTime", "updateTime"],
    ).await;
}
```

### 4. Tolerant Assertion with Dynamic Field Filtering

```rust
use genies_test::*;

#[tokio::test]
async fn compare_with_tolerance() {
    let client = http_client();

    let java_resp: Value = client
        .get("http://localhost:8080/api/detail")
        .send().await.unwrap()
        .json().await.unwrap();

    let rust_resp: Value = client
        .get("http://localhost:8081/api/detail")
        .send().await.unwrap()
        .json().await.unwrap();

    let diffs = deep_diff("api/detail", &java_resp, &rust_resp);
    assert_no_significant_diffs("api/detail", &diffs);
}
```

## The 8-Step Mutation Test Flow

`test_mutation_with_db_diff` executes the following steps:

1. **Snapshot** — Capture initial database state for affected tables
2. **Java Call** — Send the mutation request to the Java service
3. **Java Diff** — Snapshot again and compute database changes from Java
4. **Restore** — Roll back database to the initial state
5. **Snapshot** — Capture a fresh baseline
6. **Rust Call** — Send the same mutation request to the Rust service
7. **Rust Diff** — Snapshot again and compute database changes from Rust
8. **Compare** — Assert that Java and Rust produced equivalent database changes

## Dependencies

- **once_cell** — Lazy static initialization
- **rbatis** — Database access
- **reqwest** — HTTP client
- **serde_json** — JSON serialization and comparison
- **tokio** — Async runtime
- **rbs** — RBatis serialization helpers

## Integration with Other Crates

genies_test is designed as a standalone testing utility. It does not depend on any other Genies internal crates and can be used independently in any project that requires Java/Rust API comparison testing.

## License

See the project root for license information.
