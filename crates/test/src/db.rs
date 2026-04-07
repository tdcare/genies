//! 数据库工具 — 快照、差异、还原。

use rbatis::RBatis;
use serde_json::Value;
use std::collections::BTreeMap;

// ==================== DB Diff 基础设施 ====================

/// 数据库变更类型
#[derive(Debug, Clone, PartialEq)]
pub enum ChangeType {
    Insert(Value),
    Delete(Value),
    Update { before: Value, after: Value },
}

/// 数据库变更记录
#[derive(Debug, Clone)]
pub struct DbChange {
    pub change_type: ChangeType,
    pub pk_value: Value,
}

impl std::fmt::Display for DbChange {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match &self.change_type {
            ChangeType::Insert(row) => write!(f, "INSERT pk={}: {}", self.pk_value, row),
            ChangeType::Delete(row) => write!(f, "DELETE pk={}: {}", self.pk_value, row),
            ChangeType::Update { before, after } => {
                write!(f, "UPDATE pk={}: {} -> {}", self.pk_value, before, after)
            }
        }
    }
}

/// 受影响的表描述
pub struct AffectedTable {
    pub table: &'static str,
    pub pk_field: &'static str,
    pub order_by: &'static str,
    pub where_clause: String,
}

/// 对指定表执行带 WHERE 条件的 SELECT，返回匹配的行。
pub async fn db_snapshot(rb: &RBatis, table: &str, where_clause: &str, order_by: &str) -> Vec<Value> {
    let sql = if where_clause.is_empty() {
        format!("SELECT * FROM {} ORDER BY {}", table, order_by)
    } else {
        format!("SELECT * FROM {} WHERE {} ORDER BY {}", table, where_clause, order_by)
    };
    let rbs_value: rbs::Value = rb.query(&sql, vec![])
        .await
        .unwrap_or_else(|e| panic!("db_snapshot 查询 {} 失败: {}", table, e));
    // rbs::Value → serde_json::Value → Vec<serde_json::Value>
    let json_value = serde_json::to_value(&rbs_value)
        .unwrap_or_else(|e| panic!("db_snapshot {} rbs→json 转换失败: {}", table, e));
    match json_value {
        Value::Array(arr) => arr,
        Value::Null => vec![],
        other => vec![other],
    }
}

/// 对比两个快照，找出增删改变更。
pub fn db_diff(before: &[Value], after: &[Value], pk_field: &str) -> Vec<DbChange> {
    let mut changes = Vec::new();

    let before_map: BTreeMap<String, &Value> = before
        .iter()
        .filter_map(|row| row.get(pk_field).map(|pk| (pk.to_string(), row)))
        .collect();

    let after_map: BTreeMap<String, &Value> = after
        .iter()
        .filter_map(|row| row.get(pk_field).map(|pk| (pk.to_string(), row)))
        .collect();

    // after 中有但 before 中没有 → INSERT
    for (pk_str, row) in &after_map {
        if !before_map.contains_key(pk_str) {
            changes.push(DbChange {
                change_type: ChangeType::Insert((*row).clone()),
                pk_value: row.get(pk_field).cloned().unwrap_or(Value::Null),
            });
        }
    }

    // before 中有但 after 中没有 → DELETE
    for (pk_str, row) in &before_map {
        if !after_map.contains_key(pk_str) {
            changes.push(DbChange {
                change_type: ChangeType::Delete((*row).clone()),
                pk_value: row.get(pk_field).cloned().unwrap_or(Value::Null),
            });
        }
    }

    // 两边都有但内容不同 → UPDATE
    for (pk_str, before_row) in &before_map {
        if let Some(after_row) = after_map.get(pk_str) {
            if before_row != after_row {
                changes.push(DbChange {
                    change_type: ChangeType::Update {
                        before: (*before_row).clone(),
                        after: (*after_row).clone(),
                    },
                    pk_value: before_row.get(pk_field).cloned().unwrap_or(Value::Null),
                });
            }
        }
    }

    changes
}

/// 带重试的 exec 执行，用于应对远程数据库偶发的网络抖动超时。
async fn exec_with_retry(rb: &RBatis, sql: &str, values: Vec<rbs::Value>, op_name: &str) {
    let max_retries = 3;
    for attempt in 1..=max_retries {
        match rb.exec(sql, values.clone()).await {
            Ok(_) => return,
            Err(e) if attempt < max_retries => {
                println!(
                    "  ⚠ db_restore {} 第 {} 次尝试失败: {}，2秒后重试...",
                    op_name, attempt, e
                );
                tokio::time::sleep(std::time::Duration::from_secs(2)).await;
            }
            Err(e) => {
                panic!(
                    "db_restore {} 失败（已重试 {} 次）: {}",
                    op_name, max_retries, e
                );
            }
        }
    }
}

/// 根据 db_diff 的结果，将数据库还原到提交前的状态。
pub async fn db_restore(rb: &RBatis, table: &str, pk_field: &str, changes: &[DbChange]) {
    for change in changes {
        match &change.change_type {
            ChangeType::Insert(_) => {
                let sql = format!("DELETE FROM {} WHERE {} = ?", table, pk_field);
                let pk = rbs::to_value(&change.pk_value).unwrap();
                exec_with_retry(rb, &sql, vec![pk], "DELETE").await;
            }
            ChangeType::Delete(row) => {
                if let Value::Object(map) = row {
                    let columns: Vec<&str> = map.keys().map(|k| k.as_str()).collect();
                    let placeholders: Vec<&str> = columns.iter().map(|_| "?").collect();
                    let sql = format!(
                        "INSERT INTO {} ({}) VALUES ({})",
                        table,
                        columns.join(", "),
                        placeholders.join(", ")
                    );
                    let values: Vec<rbs::Value> = columns
                        .iter()
                        .map(|col| rbs::to_value(&map[*col]).unwrap())
                        .collect();
                    exec_with_retry(rb, &sql, values, "INSERT").await;
                }
            }
            ChangeType::Update { before, .. } => {
                if let Value::Object(map) = before {
                    let set_clauses: Vec<String> = map
                        .keys()
                        .filter(|k| k.as_str() != pk_field)
                        .map(|k| format!("{} = ?", k))
                        .collect();
                    let sql = format!(
                        "UPDATE {} SET {} WHERE {} = ?",
                        table,
                        set_clauses.join(", "),
                        pk_field
                    );
                    let mut values: Vec<rbs::Value> = map
                        .keys()
                        .filter(|k| k.as_str() != pk_field)
                        .map(|k| rbs::to_value(&map[k]).unwrap())
                        .collect();
                    values.push(rbs::to_value(&change.pk_value).unwrap());
                    exec_with_retry(rb, &sql, values, "UPDATE").await;
                }
            }
        }
    }
}
