//! 角色聚合根
//!
//! AdminRole 本身作为聚合根，此处提供验证辅助方法。

pub use crate::domain::entity::AdminRole;
pub use crate::domain::entity::RolePermission;

use genies::ddd::aggregate::{AggregateType, WithAggregateId};
use serde::{Deserialize, Serialize};

/// 角色聚合根标识（用于 Outbox 事件发布）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoleAggregate {
    pub id: String,
}

impl RoleAggregate {
    pub fn new(id: i64) -> Self {
        Self { id: id.to_string() }
    }
}

impl AggregateType for RoleAggregate {
    fn aggregate_type(&self) -> String {
        "auth-admin.Role".to_string()
    }
    fn atype() -> String {
        "auth-admin.Role".to_string()
    }
}

impl WithAggregateId for RoleAggregate {
    type Id = String;
    fn aggregate_id(&self) -> &Self::Id {
        &self.id
    }
}
