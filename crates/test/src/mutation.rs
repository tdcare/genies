//! Mutation 测试流程 — 全局锁、DB 变更对比、8 步提交测试。

use once_cell::sync::Lazy;
use tokio::sync::Mutex;

use crate::db::*;
use crate::diff::filter_fields;

/// 全局互斥锁，确保所有 POST mutation 测试串行执行
/// 避免多个测试同时修改数据库导致 db_restore 冲突
pub static DB_MUTATION_LOCK: Lazy<Mutex<()>> = Lazy::new(|| Mutex::new(()));

/// 对比两组 DbChange 是否一致（忽略指定字段）
pub fn compare_db_changes(
    java_changes: &[DbChange],
    rust_changes: &[DbChange],
    ignore_fields: &[&str],
) -> Vec<String> {
    let mut diffs = Vec::new();

    if java_changes.len() != rust_changes.len() {
        diffs.push(format!(
            "变更数量不同 Java={} Rust={}",
            java_changes.len(),
            rust_changes.len()
        ));
        return diffs;
    }

    for (i, (jc, rc)) in java_changes.iter().zip(rust_changes.iter()).enumerate() {
        let prefix = format!("changes[{}]", i);

        let j_type = match &jc.change_type {
            ChangeType::Insert(_) => "INSERT",
            ChangeType::Delete(_) => "DELETE",
            ChangeType::Update { .. } => "UPDATE",
        };
        let r_type = match &rc.change_type {
            ChangeType::Insert(_) => "INSERT",
            ChangeType::Delete(_) => "DELETE",
            ChangeType::Update { .. } => "UPDATE",
        };
        if j_type != r_type {
            diffs.push(format!("{}.type: Java={} Rust={}", prefix, j_type, r_type));
            continue;
        }

        let (j_row, r_row) = match (&jc.change_type, &rc.change_type) {
            (ChangeType::Insert(j), ChangeType::Insert(r)) => (j, r),
            (ChangeType::Delete(j), ChangeType::Delete(r)) => (j, r),
            (ChangeType::Update { after: j, .. }, ChangeType::Update { after: r, .. }) => (j, r),
            _ => continue,
        };

        let j_filtered = filter_fields(j_row, ignore_fields);
        let r_filtered = filter_fields(r_row, ignore_fields);
        let field_diffs = crate::diff::deep_diff(&prefix, &j_filtered, &r_filtered);
        diffs.extend(field_diffs);
    }

    diffs
}

/// 业务提交接口测试（基于数据库变更对比 8 步流程）
pub async fn test_mutation_with_db_diff(
    client: &reqwest::Client,
    rb: &rbatis::RBatis,
    name: &str,
    mutation_url_java: &str,
    mutation_url_rust: &str,
    mutation_body: Option<&serde_json::Value>,
    affected_tables: &[AffectedTable],
    ignore_fields: &[&str],
) {
    println!("\n[mutation_db_diff] {name}");

    // 获取全局锁，确保 mutation 测试串行执行，避免并发 db_restore 冲突
    let _lock = DB_MUTATION_LOCK.lock().await;

    // 步骤 1: 提交前快照
    println!("  [1/8] 快照受影响表...");
    let mut before_snapshots: Vec<Vec<serde_json::Value>> = Vec::new();
    for t in affected_tables {
        let snapshot = db_snapshot(rb, t.table, &t.where_clause, t.order_by).await;
        println!("    {} : {} 条记录", t.table, snapshot.len());
        before_snapshots.push(snapshot);
    }

    // 步骤 2: 调用 Java 接口提交
    println!("  [2/8] 调用 Java 接口...");
    let java_resp = if let Some(body) = mutation_body {
        client.post(mutation_url_java).json(body).send().await
    } else {
        client.post(mutation_url_java).send().await
    }
    .unwrap_or_else(|e| panic!("{name}: Java 请求失败: {e}"));
    println!("    Java HTTP {}", java_resp.status());

    // 步骤 3: 快照 & 计算 java_diff
    println!("  [3/8] 读取 Java DB 变更...");
    let mut java_diffs_per_table: Vec<Vec<DbChange>> = Vec::new();
    for (i, t) in affected_tables.iter().enumerate() {
        let after_snapshot = db_snapshot(rb, t.table, &t.where_clause, t.order_by).await;
        let changes = db_diff(&before_snapshots[i], &after_snapshot, t.pk_field);
        println!("    {} : {} 条变更", t.table, changes.len());
        java_diffs_per_table.push(changes);
    }

    // 步骤 4: 还原数据库
    println!("  [4/8] 还原数据库...");
    for (i, t) in affected_tables.iter().enumerate() {
        db_restore(rb, t.table, t.pk_field, &java_diffs_per_table[i]).await;
    }
    println!("    还原成功");

    // 步骤 5: 调用 Rust 接口提交
    println!("  [5/8] 调用 Rust 接口...");
    let rust_resp = if let Some(body) = mutation_body {
        client.post(mutation_url_rust).json(body).send().await
    } else {
        client.post(mutation_url_rust).send().await
    }
    .unwrap_or_else(|e| panic!("{name}: Rust 请求失败: {e}"));
    println!("    Rust HTTP {}", rust_resp.status());

    // 步骤 6: 快照 & 计算 rust_diff
    println!("  [6/8] 读取 Rust DB 变更...");
    let mut rust_diffs_per_table: Vec<Vec<DbChange>> = Vec::new();
    for (i, t) in affected_tables.iter().enumerate() {
        let after_snapshot = db_snapshot(rb, t.table, &t.where_clause, t.order_by).await;
        let changes = db_diff(&before_snapshots[i], &after_snapshot, t.pk_field);
        println!("    {} : {} 条变更", t.table, changes.len());
        rust_diffs_per_table.push(changes);
    }

    // 步骤 7: 对比
    println!("  [7/8] 对比 Java 和 Rust 的 DB 变更...");
    let mut all_ok = true;
    for (i, t) in affected_tables.iter().enumerate() {
        let diffs = compare_db_changes(
            &java_diffs_per_table[i],
            &rust_diffs_per_table[i],
            ignore_fields,
        );
        if diffs.is_empty() {
            println!("    [OK] {} 变更一致", t.table);
        } else {
            all_ok = false;
            println!("    [DIFF] {} 发现 {} 处差异:", t.table, diffs.len());
            for d in &diffs {
                println!("      {d}");
            }
        }
    }

    // 步骤 8: 还原 Rust 变更，清理测试脏数据
    println!("  [8/8] 还原 Rust 变更...");
    for (i, t) in affected_tables.iter().enumerate() {
        db_restore(rb, t.table, t.pk_field, &rust_diffs_per_table[i]).await;
    }
    println!("    还原成功");

    assert!(all_ok, "{name}: Java 和 Rust 的 DB 变更不一致！");
    println!("  [PASS] {name}");
}
