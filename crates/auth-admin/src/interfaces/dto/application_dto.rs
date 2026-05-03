//! 应用管理 DTO

use serde::{Deserialize, Serialize};
use salvo::oapi::ToSchema;
use genies_derive::casbin;

/// 创建应用请求
#[derive(Debug, Deserialize, ToSchema)]
pub struct CreateApplicationRequest {
    /// 应用标识（唯一）
    pub app_name: String,
    /// 显示名称
    pub display_name: Option<String>,
    /// 应用描述
    pub description: Option<String>,
    /// 微服务访问地址
    pub base_url: String,
    /// 状态：1=启用 0=禁用
    pub status: Option<i8>,
}

/// 更新应用请求
#[derive(Debug, Deserialize, ToSchema)]
pub struct UpdateApplicationRequest {
    /// 应用标识
    pub app_name: Option<String>,
    /// 显示名称
    pub display_name: Option<String>,
    /// 应用描述
    pub description: Option<String>,
    /// 微服务访问地址
    pub base_url: Option<String>,
    /// 状态：1=启用 0=禁用
    pub status: Option<i8>,
}

/// 应用视图对象
#[casbin]
#[derive(Debug, Serialize, Deserialize, Clone, ToSchema)]
pub struct ApplicationVO {
    pub id: Option<i64>,
    pub app_name: Option<String>,
    pub display_name: Option<String>,
    pub description: Option<String>,
    pub base_url: Option<String>,
    pub status: Option<i8>,
    pub created_at: Option<String>,
    pub updated_at: Option<String>,
}

impl From<crate::domain::entity::application_entity::ApplicationEntity> for ApplicationVO {
    fn from(e: crate::domain::entity::application_entity::ApplicationEntity) -> Self {
        Self {
            id: e.id,
            app_name: e.app_name,
            display_name: e.display_name,
            description: e.description,
            base_url: e.base_url,
            status: e.status,
            created_at: e.created_at.map(|d| d.to_string()),
            updated_at: e.updated_at.map(|d| d.to_string()),
        }
    }
}
