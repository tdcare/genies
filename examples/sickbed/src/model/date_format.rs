//! 日期时间序列化辅助模块
//!
//! 将 `rbdc::DateTime` 序列化为毫秒时间戳（JSON 数字），与 Java 端保持一致。
//! 反序列化时支持从毫秒时间戳数字或 ISO 8601 字符串读取。
//!
//! 注意: 此模块仅影响 serde JSON 序列化/反序列化，不影响 RBatis 的数据库读写。

use serde::{self, Deserialize, Deserializer, Serializer};

/// 序列化 `Option<rbdc::DateTime>` 为毫秒时间戳数字（与 Java Long 一致）。
pub fn serialize_option_datetime<S>(
    value: &Option<rbdc::DateTime>,
    serializer: S,
) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    match value {
        Some(dt) => {
            // rbdc::DateTime Deref 到 fastdate::DateTime，有 unix_timestamp_millis()
            let millis = dt.unix_timestamp_millis() as i64;
            serializer.serialize_i64(millis)
        }
        None => serializer.serialize_none(),
    }
}

/// 反序列化 `Option<rbdc::DateTime>`，支持：
/// - 毫秒时间戳数字（来自 JSON / Java 端）
/// - ISO 8601 字符串（来自某些场景）
/// - null
pub fn deserialize_option_datetime<'de, D>(
    deserializer: D,
) -> Result<Option<rbdc::DateTime>, D::Error>
where
    D: Deserializer<'de>,
{
    use serde::de;

    #[derive(Deserialize)]
    #[serde(untagged)]
    enum DateTimeRepr {
        Millis(i64),
        Str(String),
    }

    let repr = Option::<DateTimeRepr>::deserialize(deserializer)?;
    match repr {
        None => Ok(None),
        Some(DateTimeRepr::Millis(ms)) => {
            // 通过字符串中转构建 rbdc::DateTime，确保版本兼容
            // rbdc::DateTime 实现了 FromStr
            let secs = ms / 1000;
            let nanos = ((ms % 1000) * 1_000_000) as u32;
            let dt_utc = chrono::DateTime::from_timestamp(secs, nanos)
                .ok_or_else(|| de::Error::custom(format!("invalid timestamp millis: {}", ms)))?;
            let s = dt_utc.naive_utc().format("%Y-%m-%dT%H:%M:%S").to_string();
            let dt: rbdc::DateTime = s.parse().map_err(de::Error::custom)?;
            Ok(Some(dt))
        }
        Some(DateTimeRepr::Str(s)) => {
            if s.is_empty() {
                return Ok(None);
            }
            let dt: rbdc::DateTime = s.parse().map_err(de::Error::custom)?;
            Ok(Some(dt))
        }
    }
}
