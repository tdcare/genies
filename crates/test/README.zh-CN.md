# genies_test

Genies 框架的测试基础设施工具库，提供通用的 Java/Rust 接口对比测试、数据库快照差异对比和断言辅助功能。

## 概述

genies_test 为跨语言服务迁移验证提供可复用的测试构建模块：

- **HTTP 客户端**：预配置的支持认证的 HTTP 客户端
- **数据库工具**：快照、差异对比和还原，用于验证数据库副作用
- **Deep Diff**：递归 JSON 对比，支持动态字段过滤
- **断言辅助**：严格和宽松的差异断言工具
- **Mutation 测试**：端到端的变更操作对比流程，包含数据库验证

## 核心特性

- **Java/Rust 对比**：并行对比 Java 和 Rust 服务的 API 响应
- **数据库快照与差异**：捕获操作前后的表状态并计算变更
- **动态字段过滤**：自动忽略非确定性字段（ID、时间戳等）
- **全局 Mutation 锁**：串行化 mutation 测试，防止数据库冲突
- **环境变量覆盖**：所有配置值均可通过环境变量覆盖
- **零框架依赖**：不依赖其他 Genies 内部 crate

## 模块参考

### config — HTTP 客户端工厂

提供预配置的支持认证的 HTTP 客户端。

| 函数 | 说明 |
|------|------|
| `http_client()` | 创建禁用代理的 HTTP 客户端，支持通过 `TOKEN` 环境变量注入 Bearer 认证 |

### db — 数据库测试工具

基于快照的差异对比/还原工作流，用于验证数据库副作用。

| 函数 / 类型 | 说明 |
|-------------|------|
| `db_snapshot()` | 对指定表拍摄快照 |
| `db_diff()` | 计算两个快照之间的差异 |
| `db_restore()` | 根据差异还原数据库状态 |
| `ChangeType` | 枚举：`Insert`、`Delete`、`Update` |
| `DbChange` | 表示单行变更的结构体 |
| `AffectedTable` | 描述待快照表的结构体（表名、主键、排序、过滤条件） |

### diff — Deep Diff 工具

递归 JSON 值对比和字段过滤。

```rust
use genies_test::diff::{deep_diff, filter_fields, filter_dynamic_diffs};

// 递归对比两个 JSON 值
let diffs = deep_diff("root", &json_a, &json_b);

// 从 JSON 对象中移除指定字段
let filtered = filter_fields(&json_obj, &["id", "createTime"]);

// 过滤动态字段引起的差异（id、时间戳等）
let significant = filter_dynamic_diffs(&diffs);
```

### assertions — 断言辅助

基于差异对比的便捷测试断言封装。

```rust
use genies_test::assertions::{assert_no_diffs, assert_no_significant_diffs};

// 严格断言：存在任何差异即失败
assert_no_diffs("test_name", &diffs);

// 宽松断言：仅非动态字段差异时失败
assert_no_significant_diffs("test_name", &diffs);
```

### mutation — Mutation 测试工作流

端到端的写操作对比工作流，包含数据库验证。

```rust
use genies_test::mutation::{
    DB_MUTATION_LOCK, compare_db_changes, test_mutation_with_db_diff,
};

// 全局互斥锁确保 mutation 测试串行执行
let _lock = DB_MUTATION_LOCK.lock().await;

// 对比两组 DbChange 向量
compare_db_changes("operation", &java_changes, &rust_changes, &dynamic_fields);

// 完整的 8 步 mutation 对比测试
test_mutation_with_db_diff(
    &client, &rb, "operation_name",
    &java_url, &rust_url,
    Some(&request_body),
    &affected_tables,
    &dynamic_fields,
).await;
```

## 环境变量

| 变量名 | 说明 | 默认值 |
|--------|------|--------|
| `TOKEN` | Bearer 认证 token | *（无）* |

> **注意**：业务特定配置（服务 URL、数据库 URL、测试 ID）应在各项目的测试模块中定义。参见 sickbed 示例项目。

## 快速开始

### 1. 添加依赖

```sh
cargo add genies_test
```

> 也可以手动在 `Cargo.toml` 中添加依赖，请前往 [crates.io](https://crates.io) 查看最新版本。

### 2. 查询对比测试

```rust
use genies_test::*;
use serde_json::Value;

// 在项目测试模块中定义业务特定 URL
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

### 3. 带数据库差异的 Mutation 对比测试

```rust
use genies_test::*;

#[tokio::test]
async fn compare_mutation() {
    let client = http_client();
    // 初始化你自己的 RBatis 连接
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

### 4. 宽松断言与动态字段过滤

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

## 8 步 Mutation 测试流程

`test_mutation_with_db_diff` 执行以下步骤：

1. **快照** — 捕获受影响表的初始数据库状态
2. **Java 调用** — 向 Java 服务发送变更请求
3. **Java 差异** — 再次快照并计算 Java 产生的数据库变更
4. **还原** — 将数据库回滚到初始状态
5. **快照** — 捕获新的基线
6. **Rust 调用** — 向 Rust 服务发送相同的变更请求
7. **Rust 差异** — 再次快照并计算 Rust 产生的数据库变更
8. **对比** — 断言 Java 和 Rust 产生了等效的数据库变更

## 依赖项

- **once_cell** — 延迟静态初始化
- **rbatis** — 数据库访问
- **reqwest** — HTTP 客户端
- **serde_json** — JSON 序列化与对比
- **tokio** — 异步运行时
- **rbs** — RBatis 序列化辅助

## 与其他 Crate 的关系

genies_test 作为独立的测试工具库设计，不依赖其他 Genies 内部 crate，可独立用于任何需要 Java/Rust 接口对比测试的项目。

## 许可证

请参阅项目根目录的许可证信息。
