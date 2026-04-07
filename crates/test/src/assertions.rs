//! 断言辅助 — 查询类严格比对、业务类过滤动态字段。

use crate::diff::filter_dynamic_diffs;

/// 对比断言辅助 - 查询类（严格比对）
pub fn assert_no_diffs(label: &str, diffs: &[String]) {
    assert!(
        diffs.is_empty(),
        "[{}] Differences found:\n{}",
        label,
        diffs.join("\n")
    );
}

/// 对比断言辅助 - 业务类（过滤动态字段）
pub fn assert_no_significant_diffs(label: &str, diffs: &[String]) {
    let significant = filter_dynamic_diffs(diffs);
    assert!(
        significant.is_empty(),
        "[{}] Significant differences:\n{}",
        label,
        significant
            .iter()
            .map(|s| s.as_str())
            .collect::<Vec<_>>()
            .join("\n")
    );
}
