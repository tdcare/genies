//! 床位领域命令
//!
//! 对应 Java 命令类:
//! - ArrangementSickbedCommand: 安排床位
//! - ChangeSickbedCommand: 换床
//! - EmptySickbedCommand: 清空患者
//! - TestArrangementSickbedCommand: 测试安排床位（不回写HIS）

use serde::{Deserialize, Serialize};

/// 安排床位命令
///
/// 对应 Java: ArrangementSickbedCommand extends BaseCommandModel implements Command
#[derive(Clone, Debug, Serialize, Deserialize, Default, salvo::oapi::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct ArrangementSickbedCommand {
    /// 床位ID (Java 端字段名为 sickbedId)
    pub sickbed_id: Option<String>,
    /// 病人id
    pub patient_id: Option<String>,
}

/// 换床命令
///
/// 对应 Java: ChangeSickbedCommand implements Serializable
#[derive(Clone, Debug, Serialize, Deserialize, Default, salvo::oapi::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct ChangeSickbedCommand {
    /// 病人id
    pub patient_id: Option<String>,
    /// 原床位ID
    pub source_sickbed_id: Option<String>,
    /// 目标床位ID
    pub target_sickbed_id: Option<String>,
    /// 换床时间
    #[salvo(schema(value_type = Option<String>))]
    pub change_date: Option<rbdc::DateTime>,
}

/// 清空患者命令
///
/// 对应 Java: EmptySickbedCommand extends BaseCommandModel implements Command
/// Java 的 BaseCommandModel → IdModel 有一个 "id" 字段（JSON key = "id"）。
/// Java 的 emptySickbed 服务方法调用 cmd.getId()，即读取 JSON 的 "id" 字段，
/// 而不是 "sickbedId" 字段。
#[derive(Clone, Debug, Serialize, Deserialize, Default, salvo::oapi::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct EmptySickbedCommand {
    /// Java IdModel 继承字段 "id"：emptySickbed 用此字段查床位（不是 sickbedId）
    pub id: Option<String>,
    /// 床位ID (sickbedId)，Rust 内部保留但 Java 的 emptySickbed 不使用此字段
    pub sickbed_id: Option<String>,
    /// 病人id
    pub patient_id: Option<String>,
}

/// 测试安排床位命令（不回写HIS，用于临时安排床位）
///
/// 对应 Java: TestArrangementSickbedCommand extends BaseCommandModel implements Command
#[derive(Clone, Debug, Serialize, Deserialize, Default, salvo::oapi::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct TestArrangementSickbedCommand {
    /// 床位ID (Java 端字段名为 sickbedId)
    pub sickbed_id: Option<String>,
    /// 病人id
    pub patient_id: Option<String>,
}
