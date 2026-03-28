//! RBatis Adapter for Casbin
//!
//! 使用 RBatis ORM 读写 MySQL 中的 casbin_rules 表，实现 Casbin 权限策略的持久化存储。

use async_trait::async_trait;
use casbin::error::AdapterError;
use casbin::{Adapter, Filter, Model, Result};
use genies::context::CONTEXT;
use rbs::value;

use serde::Deserialize;

/// Casbin 规则行结构（用于从数据库反序列化）
#[derive(Debug, Deserialize, Default)]
struct CasbinRuleRow {
    #[serde(default)]
    ptype: String,
    #[serde(default)]
    v0: String,
    #[serde(default)]
    v1: String,
    #[serde(default)]
    v2: String,
    #[serde(default)]
    v3: String,
    #[serde(default)]
    v4: String,
    #[serde(default)]
    v5: String,
}

/// 将规则向量填充到 6 个元素，不足的补空字符串
fn normalize_rule(rule: &[String]) -> Vec<String> {
    let mut result = vec![String::new(); 6];
    for (i, v) in rule.iter().enumerate() {
        if i < 6 {
            result[i] = v.clone();
        }
    }
    result
}

/// 从 rbs::Value 行中提取 ptype 和规则值向量
/// 返回 (ptype, [v0, v1, v2, v3, v4, v5])，会自动过滤尾部空字符串
fn row_to_rule(row: &CasbinRuleRow) -> (String, Vec<String>) {
    let ptype = row.ptype.clone();

    let mut values = vec![
        row.v0.clone(),
        row.v1.clone(),
        row.v2.clone(),
        row.v3.clone(),
        row.v4.clone(),
        row.v5.clone(),
    ];

    // 过滤掉尾部空字符串
    while values.last().map(|s| s.is_empty()).unwrap_or(false) {
        values.pop();
    }

    (ptype, values)
}

/// 根据 ptype 确定 section
/// "p" 开头的返回 "p"，"g" 开头的返回 "g"
fn ptype_to_sec(ptype: &str) -> &str {
    if ptype.starts_with('p') {
        "p"
    } else {
        "g"
    }
}

/// RBatis Casbin Adapter
///
/// 使用 RBatis ORM 从 MySQL 的 casbin_rules 表加载和保存 Casbin 策略。
///
/// # 使用示例
/// ```ignore
/// use genies_auth::adapter::RBatisAdapter;
/// use casbin::prelude::*;
///
/// let adapter = RBatisAdapter::new();
/// let enforcer = Enforcer::new("model.conf", adapter).await?;
/// ```
pub struct RBatisAdapter {
    is_filtered: bool,
}

impl RBatisAdapter {
    /// 创建新的 RBatisAdapter 实例
    pub fn new() -> Self {
        Self { is_filtered: false }
    }
}

impl Default for RBatisAdapter {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Adapter for RBatisAdapter {
    /// 从数据库加载所有策略规则
    async fn load_policy(&mut self, m: &mut dyn Model) -> Result<()> {
        self.is_filtered = false;

        let sql = "SELECT ptype, v0, v1, v2, v3, v4, v5 FROM casbin_rules";
        let value: rbs::Value = CONTEXT
            .rbatis
            .query(sql, vec![])
            .await
            .map_err(|e| AdapterError(Box::new(e)))?;
        
        let rows: Vec<CasbinRuleRow> = rbs::from_value(value)
            .map_err(|e| AdapterError(Box::new(e)))?;

        for row in &rows {
            let (ptype, rule) = row_to_rule(row);
            if !ptype.is_empty() {
                let sec = ptype_to_sec(&ptype);
                m.add_policy(sec, &ptype, rule);
            }
        }

        Ok(())
    }

    /// 加载带过滤条件的策略
    async fn load_filtered_policy<'a>(
        &mut self,
        m: &mut dyn Model,
        f: Filter<'a>,
    ) -> Result<()> {
        self.is_filtered = true;

        let sql = "SELECT ptype, v0, v1, v2, v3, v4, v5 FROM casbin_rules";
        let value: rbs::Value = CONTEXT
            .rbatis
            .query(sql, vec![])
            .await
            .map_err(|e| AdapterError(Box::new(e)))?;
        
        let rows: Vec<CasbinRuleRow> = rbs::from_value(value)
            .map_err(|e| AdapterError(Box::new(e)))?;

        for row in &rows {
            let (ptype, rule) = row_to_rule(row);
            if ptype.is_empty() {
                continue;
            }

            let sec = ptype_to_sec(&ptype);

            // 根据 section 获取对应的过滤条件
            let filters = if sec == "p" { &f.p } else { &f.g };

            // 检查是否匹配过滤条件
            let matched = if filters.is_empty() {
                true
            } else {
                filters.iter().enumerate().all(|(i, filter)| {
                    filter.is_empty() || rule.get(i).map(|v| v == *filter).unwrap_or(true)
                })
            };

            if matched {
                m.add_policy(sec, &ptype, rule);
            }
        }

        Ok(())
    }

    /// 保存所有策略到数据库（先清空再批量插入）
    async fn save_policy(&mut self, m: &mut dyn Model) -> Result<()> {
        // 先清空表
        self.clear_policy().await?;

        let model = m.get_model();

        // 遍历 "p" 和 "g" sections
        for sec in ["p", "g"] {
            if let Some(assertions) = model.get(sec) {
                for (ptype, assertion) in assertions {
                    for rule in &assertion.policy {
                        let normalized = normalize_rule(rule);
                        let sql = "INSERT INTO casbin_rules (ptype, v0, v1, v2, v3, v4, v5) VALUES (?, ?, ?, ?, ?, ?, ?)";
                        CONTEXT
                            .rbatis
                            .exec(
                                sql,
                                vec![
                                    value!(ptype),
                                    value!(&normalized[0]),
                                    value!(&normalized[1]),
                                    value!(&normalized[2]),
                                    value!(&normalized[3]),
                                    value!(&normalized[4]),
                                    value!(&normalized[5]),
                                ],
                            )
                            .await
                            .map_err(|e| AdapterError(Box::new(e)))?;
                    }
                }
            }
        }

        Ok(())
    }

    /// 清空所有策略
    async fn clear_policy(&mut self) -> Result<()> {
        let sql = "DELETE FROM casbin_rules";
        CONTEXT
            .rbatis
            .exec(sql, vec![])
            .await
            .map_err(|e| AdapterError(Box::new(e)))?;
        Ok(())
    }

    /// 返回是否使用了过滤加载
    fn is_filtered(&self) -> bool {
        self.is_filtered
    }

    /// 添加单条策略规则
    async fn add_policy(&mut self, _sec: &str, ptype: &str, rule: Vec<String>) -> Result<bool> {
        let normalized = normalize_rule(&rule);
        let sql = "INSERT INTO casbin_rules (ptype, v0, v1, v2, v3, v4, v5) VALUES (?, ?, ?, ?, ?, ?, ?)";

        let result = CONTEXT
            .rbatis
            .exec(
                sql,
                vec![
                    value!(ptype),
                    value!(&normalized[0]),
                    value!(&normalized[1]),
                    value!(&normalized[2]),
                    value!(&normalized[3]),
                    value!(&normalized[4]),
                    value!(&normalized[5]),
                ],
            )
            .await
            .map_err(|e| AdapterError(Box::new(e)))?;

        Ok(result.rows_affected > 0)
    }

    /// 批量添加策略规则
    async fn add_policies(
        &mut self,
        sec: &str,
        ptype: &str,
        rules: Vec<Vec<String>>,
    ) -> Result<bool> {
        for rule in rules {
            self.add_policy(sec, ptype, rule).await?;
        }
        Ok(true)
    }

    /// 删除单条策略规则
    async fn remove_policy(&mut self, _sec: &str, ptype: &str, rule: Vec<String>) -> Result<bool> {
        let normalized = normalize_rule(&rule);
        let sql = "DELETE FROM casbin_rules WHERE ptype = ? AND v0 = ? AND v1 = ? AND v2 = ? AND v3 = ? AND v4 = ? AND v5 = ?";

        let result = CONTEXT
            .rbatis
            .exec(
                sql,
                vec![
                    value!(ptype),
                    value!(&normalized[0]),
                    value!(&normalized[1]),
                    value!(&normalized[2]),
                    value!(&normalized[3]),
                    value!(&normalized[4]),
                    value!(&normalized[5]),
                ],
            )
            .await
            .map_err(|e| AdapterError(Box::new(e)))?;

        Ok(result.rows_affected > 0)
    }

    /// 批量删除策略规则
    async fn remove_policies(
        &mut self,
        sec: &str,
        ptype: &str,
        rules: Vec<Vec<String>>,
    ) -> Result<bool> {
        for rule in rules {
            self.remove_policy(sec, ptype, rule).await?;
        }
        Ok(true)
    }

    /// 按字段条件删除策略
    ///
    /// # 参数
    /// - `field_index`: 起始字段索引（0=v0, 1=v1, ...）
    /// - `field_values`: 从 field_index 开始的字段匹配值
    async fn remove_filtered_policy(
        &mut self,
        _sec: &str,
        ptype: &str,
        field_index: usize,
        field_values: Vec<String>,
    ) -> Result<bool> {
        if field_values.is_empty() {
            return Ok(false);
        }

        // 构建动态 WHERE 条件
        let mut conditions = vec!["ptype = ?".to_string()];
        let mut params: Vec<rbs::Value> = vec![value!(ptype)];

        for (i, value) in field_values.iter().enumerate() {
            let field_idx = field_index + i;
            if field_idx >= 6 {
                break;
            }
            if !value.is_empty() {
                conditions.push(format!("v{} = ?", field_idx));
                params.push(value!(value));
            }
        }

        let sql = format!(
            "DELETE FROM casbin_rules WHERE {}",
            conditions.join(" AND ")
        );

        let result = CONTEXT
            .rbatis
            .exec(&sql, params)
            .await
            .map_err(|e| AdapterError(Box::new(e)))?;

        Ok(result.rows_affected > 0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_normalize_rule() {
        // 空规则
        let rule: Vec<String> = vec![];
        let result = normalize_rule(&rule);
        assert_eq!(result.len(), 6);
        assert!(result.iter().all(|s| s.is_empty()));

        // 3 个元素
        let rule = vec!["a".to_string(), "b".to_string(), "c".to_string()];
        let result = normalize_rule(&rule);
        assert_eq!(result, vec!["a", "b", "c", "", "", ""]);

        // 6 个元素
        let rule = vec![
            "a".to_string(),
            "b".to_string(),
            "c".to_string(),
            "d".to_string(),
            "e".to_string(),
            "f".to_string(),
        ];
        let result = normalize_rule(&rule);
        assert_eq!(result, vec!["a", "b", "c", "d", "e", "f"]);

        // 超过 6 个元素，只取前 6 个
        let rule = vec![
            "a".to_string(),
            "b".to_string(),
            "c".to_string(),
            "d".to_string(),
            "e".to_string(),
            "f".to_string(),
            "g".to_string(),
        ];
        let result = normalize_rule(&rule);
        assert_eq!(result, vec!["a", "b", "c", "d", "e", "f"]);
    }

    #[test]
    fn test_ptype_to_sec() {
        assert_eq!(ptype_to_sec("p"), "p");
        assert_eq!(ptype_to_sec("p2"), "p");
        assert_eq!(ptype_to_sec("g"), "g");
        assert_eq!(ptype_to_sec("g2"), "g");
        assert_eq!(ptype_to_sec("g3"), "g");
    }

    #[test]
    fn test_row_to_rule() {
        // 测试 CasbinRuleRow 转换
        let row = CasbinRuleRow {
            ptype: "p".to_string(),
            v0: "admin".to_string(),
            v1: "/api/*".to_string(),
            v2: "GET".to_string(),
            v3: "".to_string(),
            v4: "".to_string(),
            v5: "".to_string(),
        };

        let (ptype, rule) = row_to_rule(&row);
        assert_eq!(ptype, "p");
        // 尾部空字符串应该被过滤
        assert_eq!(rule, vec!["admin", "/api/*", "GET"]);
    }
}
