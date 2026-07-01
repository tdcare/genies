//! 应用实例 DTO

use serde::{Deserialize, Serialize};
use salvo::oapi::ToSchema;

/// 注册实例请求
#[derive(Debug, Deserialize, ToSchema)]
pub struct RegisterInstanceRequest {
    /// 应用标识
    pub app_name: String,
    /// 实例 ID（雪花算法生成）
    pub instance_id: i64,
    /// 实例访问地址
    pub base_url: String,
    /// 显示名称
    pub display_name: Option<String>,
    /// 版本号
    pub version: Option<String>,
}

/// 心跳请求
#[derive(Debug, Deserialize, ToSchema)]
pub struct HeartbeatRequest {
    /// 实例 ID
    pub instance_id: i64,
}

/// 注销实例请求
#[derive(Debug, Deserialize, ToSchema)]
pub struct DeregisterRequest {
    /// 实例 ID
    pub instance_id: i64,
}

/// 实例视图对象
#[derive(Debug, Serialize, Deserialize, Clone, ToSchema)]
pub struct InstanceVO {
    pub id: Option<i64>,
    pub app_name: Option<String>,
    pub instance_id: Option<String>,
    pub base_url: Option<String>,
    pub version: Option<String>,
    /// 状态：1=在线 0=离线
    pub status: Option<i8>,
    pub last_heartbeat_at: Option<String>,
    pub registered_at: Option<String>,
}

impl From<crate::domain::entity::app_instance_entity::AppInstanceEntity> for InstanceVO {
    fn from(e: crate::domain::entity::app_instance_entity::AppInstanceEntity) -> Self {
        Self {
            id: e.id,
            app_name: e.app_name,
            instance_id: e.instance_id.map(|id| id.to_string()),
            base_url: e.base_url,
            version: e.version,
            status: e.status,
            last_heartbeat_at: e.last_heartbeat_at.map(|d| d.to_string()),
            registered_at: e.registered_at.map(|d| d.to_string()),
        }
    }
}
