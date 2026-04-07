---
name: test-usage
description: Guide for using genies_test testing infrastructure. Use when implementing Java/Rust API comparison tests, database snapshot/diff/restore operations, deep JSON diff analysis, or integrating test utilities into Genies microservices.
---

# Test Module (genies_test)

## Overview

genies_test 是 Genies 框架的 **Java/Rust 微服务对比测试基础设施**，提供完整的 API 响应对比、数据库变更验证、Deep JSON Diff 分析能力。

**解决的问题：** 在 Java → Rust 微服务迁移过程中，需要验证 Rust 实现与 Java 行为完全一致——包括 API 响应和数据库副作用。

**核心特性：**
- HTTP 客户端：支持 Token 认证的通用 HTTP client 工厂
- DB 快照/差异/还原：对任意表进行 snapshot → diff → restore，实现无副作用测试
- Deep Diff：递归比对 JSON 结构，精确定位字段级差异
- 断言辅助：严格模式 + 动态字段过滤模式
- Mutation 测试流程：完整的 8 步提交→对比→还原流程，验证写入操作一致性

> **注意**：业务特定配置（服务 URL、数据库 URL、测试 ID、cleanup 函数）应在各项目的测试模块中定义，不在 genies_test 中。参见 sickbed 示例的 `tests/common/mod.rs`。

## Quick Reference

### 配置

```rust
use genies_test::config::http_client;

let client = http_client();        // 禁用代理，自动注入 TOKEN
```

> 业务特定 URL（如 `java_base_url()`、`rust_base_url()`）和测试 ID（如 `test_dept_id()`）应在项目测试模块中定义。

### 数据库操作

```rust
use genies_test::db::{db_snapshot, db_diff, db_restore};

// 需要自行创建 RBatis 连接
let rb = rbatis::RBatis::new();
rb.init(rbdc_mysql::MysqlDriver {}, &database_url()).unwrap();

let before = db_snapshot(&rb, "MyTable", "deptId = ?", "name").await;
// ... 执行操作 ...
let after = db_snapshot(&rb, "MyTable", "deptId = ?", "name").await;
let changes = db_diff(&before, &after, "id");
db_restore(&rb, "MyTable", "id", &changes).await;
```

### Deep Diff

```rust
use genies_test::diff::{deep_diff, filter_fields, filter_dynamic_diffs};

let diffs = deep_diff("root", &java_json, &rust_json);
let significant = filter_dynamic_diffs(&diffs);
```

### 断言

```rust
use genies_test::assertions::{assert_no_diffs, assert_no_significant_diffs};

assert_no_diffs("查询接口", &diffs);              // 严格：任何差异都失败
assert_no_significant_diffs("业务接口", &diffs);   // 宽松：过滤动态字段
```

### Mutation 测试

```rust
use genies_test::mutation::{test_mutation_with_db_diff, DB_MUTATION_LOCK, AffectedTable};

test_mutation_with_db_diff(
    &client, &rb, "创建记录",
    &format!("{}/create", java_base_url()),
    &format!("{}/create", rust_base_url()),
    Some(&body),
    &[AffectedTable {
        table: "MyTable",
        pk_field: "id",
        order_by: "name",
        where_clause: format!("deptId = '{}'", dept_id),
    }],
    &["createTime", "updateTime"],
).await;
```

## 模块详解

### config 模块

genies_test 的 config 模块只提供通用的 `http_client()` 函数：

| 函数 | 环境变量 | 说明 |
|------|----------|------|
| `http_client()` | `TOKEN` | 创建禁用代理的 HTTP client，自动注入 Bearer 认证 |

> 业务特定配置（如 `java_base_url()`、`rust_base_url()`、`test_dept_id()` 等）应在各项目的 `tests/common/mod.rs` 中定义。

#### http_client

```rust
pub fn http_client() -> Client
```

创建禁用代理的 reqwest Client。若环境变量 `TOKEN` 存在，自动注入 `Authorization: Bearer {TOKEN}` 到所有请求的默认 Header。

```rust
// 示例：设置 TOKEN 后所有请求都带认证
// export TOKEN="eyJhbGciOiJSUzI1NiIs..."
let client = http_client();
let resp = client.get(&format!("{}/list", java_base_url())).send().await?;
```

### db 模块

db 模块提供通用的数据库快照/差异/还原工具。数据库连接和业务特定 cleanup 函数应在项目测试模块中定义。

#### db_snapshot

```rust
pub async fn db_snapshot(
    rb: &RBatis,
    table: &str,
    where_clause: &str,
    order_by: &str,
) -> Vec<Value>
```

对指定表执行 `SELECT * FROM {table} WHERE {where_clause} ORDER BY {order_by}`，返回 JSON 行数组。

```rust
// 快照某条件下的所有记录
let rows = db_snapshot(&rb, "MyTable",
    &format!("deptId = '{}'", dept_id),
    "name"
).await;
println!("当前 {} 条记录", rows.len());
```

#### db_diff

```rust
pub fn db_diff(
    before: &[Value],
    after: &[Value],
    pk_field: &str,
) -> Vec<DbChange>
```

对比两个快照，找出增/删/改变更。通过 `pk_field` 作为主键关联行。

返回 `Vec<DbChange>`，每个 DbChange 包含：
- `change_type`: `Insert(row)` / `Delete(row)` / `Update { before, after }`
- `pk_value`: 主键值

#### db_restore

```rust
pub async fn db_restore(
    rb: &RBatis,
    table: &str,
    pk_field: &str,
    changes: &[DbChange],
)
```

根据 db_diff 结果还原数据库：
- `Insert` → 执行 DELETE（撤销插入）
- `Delete` → 执行 INSERT（恢复删除）
- `Update` → 执行 UPDATE 恢复到 before 状态

### diff 模块

#### deep_diff

```rust
pub fn deep_diff(path: &str, java: &Value, rust: &Value) -> Vec<String>
```

递归比对两个 JSON Value，返回人可读的差异列表。

规则：
- **Object**：忽略字段顺序，逐 key 比对；报告 missing / extra 字段
- **Array**：先比长度，再按索引逐元素比对
- **Scalar**：直接 `!=` 比较

```rust
let diffs = deep_diff("resp", &java_json, &rust_json);
// 输出示例：
// "resp.data.name: Java=\"张三\" Rust=\"张三 \""
// "resp.data.items[2].status: Java=1 Rust=0"
// "resp.data.extra_field: extra in Rust"
```

#### filter_fields

```rust
pub fn filter_fields(value: &Value, ignore_fields: &[&str]) -> Value
```

从 JSON Object 中移除指定字段。用于 DB 变更对比时忽略时间戳等动态字段。

```rust
let filtered = filter_fields(&row, &["createTime", "updateTime", "id"]);
```

#### filter_dynamic_diffs

```rust
pub fn filter_dynamic_diffs(diffs: &[String]) -> Vec<&String>
```

过滤掉已知动态字段差异。默认过滤：`.id:`, `time:`, `timestamp:`, `createtime:`, `updatetime:`, `created_at:`, `updated_at:`（不区分大小写）。

```rust
let all_diffs = deep_diff("root", &java, &rust);
let significant = filter_dynamic_diffs(&all_diffs);
// significant 仅包含非动态字段的差异
```

### assertions 模块

#### assert_no_diffs

```rust
pub fn assert_no_diffs(label: &str, diffs: &[String])
```

严格断言：diffs 为空则通过，否则 panic 并打印所有差异。适用于查询接口的精确比对。

#### assert_no_significant_diffs

```rust
pub fn assert_no_significant_diffs(label: &str, diffs: &[String])
```

宽松断言：先调用 `filter_dynamic_diffs` 过滤动态字段，剩余差异为空则通过。适用于业务接口的比对。

### mutation 模块

#### DB_MUTATION_LOCK

```rust
pub static DB_MUTATION_LOCK: Lazy<Mutex<()>> = Lazy::new(|| Mutex::new(()));
```

全局 tokio Mutex，确保所有 mutation 测试串行执行，避免并发 db_restore 冲突。`test_mutation_with_db_diff` 内部自动获取该锁。

#### compare_db_changes

```rust
pub fn compare_db_changes(
    java_changes: &[DbChange],
    rust_changes: &[DbChange],
    ignore_fields: &[&str],
) -> Vec<String>
```

对比 Java 和 Rust 的数据库变更是否一致：
1. 检查变更数量是否相同
2. 逐条检查变更类型（INSERT/DELETE/UPDATE）
3. 对变更内容做 deep_diff（忽略指定字段）

#### test_mutation_with_db_diff

```rust
pub async fn test_mutation_with_db_diff(
    client: &Client,
    rb: &RBatis,
    name: &str,
    mutation_url_java: &str,
    mutation_url_rust: &str,
    mutation_body: Option<&Value>,
    affected_tables: &[AffectedTable],
    ignore_fields: &[&str],
)
```

完整的 Mutation 对比测试，**8 步流程**：

| 步骤 | 操作 | 说明 |
|------|------|------|
| 1/8 | 快照受影响表 | 记录 before 状态 |
| 2/8 | 调用 Java 接口 | POST 提交到 Java 服务 |
| 3/8 | 读取 Java DB 变更 | 快照 after，计算 java_diff |
| 4/8 | 还原数据库 | 用 db_restore 回滚 Java 的变更 |
| 5/8 | 调用 Rust 接口 | POST 提交到 Rust 服务 |
| 6/8 | 读取 Rust DB 变更 | 快照 after，计算 rust_diff |
| 7/8 | 对比变更 | compare_db_changes(java, rust) |
| 8/8 | 还原 Rust 变更 | 清理 Rust 的数据库副作用 |

**AffectedTable 结构**：

```rust
pub struct AffectedTable {
    pub table: &'static str,       // 表名
    pub pk_field: &'static str,    // 主键字段
    pub order_by: &'static str,    // 排序字段（确保快照一致）
    pub where_clause: String,      // WHERE 条件（限定范围）
}
```

## 使用模式

### 查询接口对比测试

适用于 GET 查询接口，只比对 API 响应：

```rust
#[tokio::test]
async fn test_list_records() {
    let client = http_client();
    let dept_id = test_dept_id();

    let java_url = format!("{}/list?deptId={}", java_base_url(), dept_id);
    let rust_url = format!("{}/list?deptId={}", rust_base_url(), dept_id);

    let java_resp: Value = client.get(&java_url).send().await.unwrap()
        .json().await.unwrap();
    let rust_resp: Value = client.get(&rust_url).send().await.unwrap()
        .json().await.unwrap();

    let diffs = deep_diff("list_records", &java_resp, &rust_resp);
    assert_no_significant_diffs("list_records", &diffs);
}
```

### Mutation 接口对比测试（含 DB Diff）

适用于 POST 写入接口，验证 API 响应 + 数据库副作用：

```rust
#[tokio::test]
async fn test_create_record() {
    let client = http_client();
    let rb = init_test_rbatis().await;
    let dept_id = test_dept_id();

    let body = serde_json::json!({
        "deptId": dept_id,
        "name": "CMP-TEST-001",
        "type": 1
    });

    test_mutation_with_db_diff(
        &client,
        &rb,
        "创建记录",
        &format!("{}/create", java_base_url()),
        &format!("{}/create", rust_base_url()),
        Some(&body),
        &[AffectedTable {
            table: "MyTable",
            pk_field: "id",
            order_by: "name",
            where_clause: format!("deptId = '{}'", dept_id),
        }],
        &["createTime", "updateTime"],
    ).await;
}
```

### 自定义 Deep Diff 过滤

当默认 `filter_dynamic_diffs` 不够用时，可自定义过滤逻辑：

```rust
let diffs = deep_diff("root", &java_json, &rust_json);

// 自定义过滤：忽略特定字段
let significant: Vec<&String> = diffs.iter()
    .filter(|d| {
        let dl = d.to_lowercase();
        !dl.contains(".id:")
            && !dl.contains("time:")
            && !dl.contains(".version:")
            && !dl.contains(".operator:")
    })
    .collect();

assert!(significant.is_empty(),
    "发现 {} 处有意义差异:\n{}",
    significant.len(),
    significant.iter().map(|s| s.as_str()).collect::<Vec<_>>().join("\n")
);
```

### 多表 Mutation 测试

当一个操作影响多张表时：

```rust
test_mutation_with_db_diff(
    &client, &rb, "转移操作",
    &java_url, &rust_url,
    Some(&body),
    &[
        AffectedTable {
            table: "MainTable",
            pk_field: "id",
            order_by: "name",
            where_clause: format!("deptId = '{}'", dept_id),
        },
        AffectedTable {
            table: "RecordTable",
            pk_field: "id",
            order_by: "createTime",
            where_clause: format!("wardId = '{}'", ward_id),
        },
    ],
    &["createTime", "updateTime", "operateTime"],
).await;
```

## 环境变量配置表

genies_test 本身仅使用以下环境变量：

| 变量名 | 说明 | 默认值 |
|--------|------|--------|
| `TOKEN` | Bearer 认证 token | (无，不设则不加认证头) |

> 业务特定环境变量（如 `JAVA_BASE_URL`、`RUST_BASE_URL`、`TEST_DATABASE_URL`、`TEST_DEPT_ID` 等）应在各项目的测试模块中处理。

## 与其他 Crate 集成

| Crate | 集成方式 |
|-------|----------|
| sickbed (example) | 在 `tests/common/mod.rs` 定义业务特定配置，`tests/` 中编写对比测试 |
| reqwest | http_client() 构建统一的 HTTP client |
| rbatis | db_snapshot / db_diff / db_restore 操作数据库 |
| serde_json | deep_diff / filter_fields 操作 JSON Value |

### 在 sickbed 项目中使用

```toml
# examples/sickbed/Cargo.toml
[dev-dependencies]
genies_test = { path = "../../crates/test" }
tokio = { version = "1", features = ["full"] }
serde_json = "1"
```

```rust
// examples/sickbed/tests/sickbed_comparison_tests.rs
mod common;
use common::*;

#[tokio::test]
async fn test_sickbed_list() {
    let client = http_client();
    // java_base_url() / rust_base_url() / test_dept_id() 等
    // 来自 common/mod.rs 中的业务特定定义
    // ...
}
```

## Key Files

- [examples/sickbed/tests/common/mod.rs](file:///d:/tdcare/genies/examples/sickbed/tests/common/mod.rs) - sickbed 业务特定测试配置（URL、数据库、测试 ID、cleanup）
- [examples/sickbed/tests/sickbed_comparison_tests.rs](file:///d:/tdcare/genies/examples/sickbed/tests/sickbed_comparison_tests.rs) - 床位对比测试用例
- [examples/sickbed/tests/ward_comparison_tests.rs](file:///d:/tdcare/genies/examples/sickbed/tests/ward_comparison_tests.rs) - 病房对比测试用例
- [crates/test/src/lib.rs](file:///d:/tdcare/genies/crates/test/src/lib.rs) - genies_test crate 入口
