//! 用户聚合根
//!
//! AdminUser 本身作为聚合根，此处提供验证辅助方法。

pub use crate::domain::entity::AdminUser;
pub use crate::domain::entity::UserRole;

use genies::ddd::aggregate::{AggregateType, WithAggregateId};
use serde::{Deserialize, Serialize};

/// 用户聚合根标识（用于 Outbox 事件发布）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserAggregate {
    pub id: String,
}

impl UserAggregate {
    pub fn new(id: i64) -> Self {
        Self { id: id.to_string() }
    }
}

impl AggregateType for UserAggregate {
    fn aggregate_type(&self) -> String {
        "auth-admin.User".to_string()
    }
    fn atype() -> String {
        "auth-admin.User".to_string()
    }
}

impl WithAggregateId for UserAggregate {
    type Id = String;
    fn aggregate_id(&self) -> &Self::Id {
        &self.id
    }
}
