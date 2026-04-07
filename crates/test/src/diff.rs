//! Diff 工具 — deep_diff、动态字段过滤、字段过滤。

use serde_json::Value;

/// 递归比对两个 JSON Value，返回差异列表。
/// 忽略字段顺序，对数组按元素逐一比对。
pub fn deep_diff(path: &str, java: &Value, rust: &Value) -> Vec<String> {
    let mut diffs = Vec::new();
    match (java, rust) {
        (Value::Object(j), Value::Object(r)) => {
            for (key, jv) in j {
                match r.get(key) {
                    Some(rv) => diffs.extend(deep_diff(&format!("{}.{}", path, key), jv, rv)),
                    None => diffs.push(format!("{}.{}: missing in Rust", path, key)),
                }
            }
            for key in r.keys() {
                if !j.contains_key(key) {
                    diffs.push(format!("{}.{}: extra in Rust", path, key));
                }
            }
        }
        (Value::Array(j), Value::Array(r)) => {
            if j.len() != r.len() {
                diffs.push(format!(
                    "{}: array length Java={} Rust={}",
                    path,
                    j.len(),
                    r.len()
                ));
            }
            for (i, (jv, rv)) in j.iter().zip(r.iter()).enumerate() {
                diffs.extend(deep_diff(&format!("{}[{}]", path, i), jv, rv));
            }
        }
        _ => {
            if java != rust {
                diffs.push(format!("{}: Java={} Rust={}", path, java, rust));
            }
        }
    }
    diffs
}

/// 过滤掉动态字段差异（id、时间戳等）
pub fn filter_dynamic_diffs(diffs: &[String]) -> Vec<&String> {
    diffs
        .iter()
        .filter(|d| {
            let dl = d.to_lowercase();
            !dl.contains(".id:")
                && !dl.contains("time:")
                && !dl.contains("timestamp:")
                && !dl.contains("createtime:")
                && !dl.contains("updatetime:")
                && !dl.contains("created_at:")
                && !dl.contains("updated_at:")
        })
        .collect()
}

/// 从 JSON Object 中移除指定字段
pub fn filter_fields(value: &Value, ignore_fields: &[&str]) -> Value {
    if let Value::Object(map) = value {
        let filtered: serde_json::Map<String, Value> = map
            .iter()
            .filter(|(k, _)| !ignore_fields.contains(&k.as_str()))
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect();
        Value::Object(filtered)
    } else {
        value.clone()
    }
}
