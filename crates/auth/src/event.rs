//! Auth 模块领域事件定义
//!
//! 定义所有需要通过 Dapr pub/sub 跨服务同步的领域事件。
//! 这些事件由外部的 Auth Admin 服务发布，各微服务的 auth crate 订阅并更新本地副本。
//!
//! # 事件分类
//! - 用户事件: UserCreated / UserUpdated / UserDeleted
//! - 角色事件: RoleCreated / RoleUpdated / RoleDeleted
//! - 权限事件: PermissionCreated / PermissionUpdated / PermissionDeleted
//! - 关联事件: UserRoleAssigned / UserRoleRevoked / RolePermissionAssigned / RolePermissionRevoked

use genies_derive::DomainEvent;
use serde::{Deserialize, Serialize};

// ============================================================================
// 用户事件
// ============================================================================

/// 用户创建事件
#[derive(Debug, Clone, Serialize, Deserialize, Default, DomainEvent)]
#[event_type("auth.user.created")]
#[event_type_version("V1")]
#[event_source("auth-admin")]
pub struct UserCreatedEvent {
    pub id: i64,
    pub username: String,
    pub password_hash: String,
    pub display_name: String,
    pub email: String,
    pub phone: String,
    pub department_id: String,
    pub department_name: String,
    pub status: i8,
}

/// 用户更新事件
#[derive(Debug, Clone, Serialize, Deserialize, Default, DomainEvent)]
#[event_type("auth.user.updated")]
#[event_type_version("V1")]
#[event_source("auth-admin")]
pub struct UserUpdatedEvent {
    pub id: i64,
    pub username: String,
    pub password_hash: String,
    pub display_name: String,
    pub email: String,
    pub phone: String,
    pub department_id: String,
    pub department_name: String,
    pub status: i8,
}

/// 用户删除事件
#[derive(Debug, Clone, Serialize, Deserialize, Default, DomainEvent)]
#[event_type("auth.user.deleted")]
#[event_type_version("V1")]
#[event_source("auth-admin")]
pub struct UserDeletedEvent {
    pub id: i64,
    pub username: String,
}

// ============================================================================
// 角色事件
// ============================================================================

/// 角色创建事件
#[derive(Debug, Clone, Serialize, Deserialize, Default, DomainEvent)]
#[event_type("auth.role.created")]
#[event_type_version("V1")]
#[event_source("auth-admin")]
pub struct RoleCreatedEvent {
    pub id: i64,
    pub name: String,
    pub display_name: String,
    pub description: String,
    pub status: i8,
}

/// 角色更新事件
#[derive(Debug, Clone, Serialize, Deserialize, Default, DomainEvent)]
#[event_type("auth.role.updated")]
#[event_type_version("V1")]
#[event_source("auth-admin")]
pub struct RoleUpdatedEvent {
    pub id: i64,
    pub name: String,
    pub display_name: String,
    pub description: String,
    pub status: i8,
}

/// 角色删除事件
#[derive(Debug, Clone, Serialize, Deserialize, Default, DomainEvent)]
#[event_type("auth.role.deleted")]
#[event_type_version("V1")]
#[event_source("auth-admin")]
pub struct RoleDeletedEvent {
    pub id: i64,
    pub name: String,
}

// ============================================================================
// 权限事件
// ============================================================================

/// 权限创建事件
#[derive(Debug, Clone, Serialize, Deserialize, Default, DomainEvent)]
#[event_type("auth.permission.created")]
#[event_type_version("V1")]
#[event_source("auth-admin")]
pub struct PermissionCreatedEvent {
    pub id: i64,
    pub name: String,
    pub resource: String,
    pub action: String,
    pub description: String,
    pub status: i8,
}

/// 权限更新事件
#[derive(Debug, Clone, Serialize, Deserialize, Default, DomainEvent)]
#[event_type("auth.permission.updated")]
#[event_type_version("V1")]
#[event_source("auth-admin")]
pub struct PermissionUpdatedEvent {
    pub id: i64,
    pub name: String,
    pub resource: String,
    pub action: String,
    pub description: String,
    pub status: i8,
}

/// 权限删除事件
#[derive(Debug, Clone, Serialize, Deserialize, Default, DomainEvent)]
#[event_type("auth.permission.deleted")]
#[event_type_version("V1")]
#[event_source("auth-admin")]
pub struct PermissionDeletedEvent {
    pub id: i64,
    pub name: String,
    pub resource: String,
    pub action: String,
}

// ============================================================================
// 关联事件
// ============================================================================

/// 用户-角色分配事件
#[derive(Debug, Clone, Serialize, Deserialize, Default, DomainEvent)]
#[event_type("auth.user_role.assigned")]
#[event_type_version("V1")]
#[event_source("auth-admin")]
pub struct UserRoleAssignedEvent {
    pub user_id: i64,
    pub username: String,
    pub role_id: i64,
    pub role_name: String,
}

/// 用户-角色移除事件
#[derive(Debug, Clone, Serialize, Deserialize, Default, DomainEvent)]
#[event_type("auth.user_role.revoked")]
#[event_type_version("V1")]
#[event_source("auth-admin")]
pub struct UserRoleRevokedEvent {
    pub user_id: i64,
    pub username: String,
    pub role_id: i64,
    pub role_name: String,
}

/// 角色-权限分配事件
#[derive(Debug, Clone, Serialize, Deserialize, Default, DomainEvent)]
#[event_type("auth.role_permission.assigned")]
#[event_type_version("V1")]
#[event_source("auth-admin")]
pub struct RolePermissionAssignedEvent {
    pub role_id: i64,
    pub permission_id: i64,
    pub role_name: String,
    pub resource: String,
    pub action: String,
}

/// 角色-权限移除事件
#[derive(Debug, Clone, Serialize, Deserialize, Default, DomainEvent)]
#[event_type("auth.role_permission.revoked")]
#[event_type_version("V1")]
#[event_source("auth-admin")]
pub struct RolePermissionRevokedEvent {
    pub role_id: i64,
    pub permission_id: i64,
    pub role_name: String,
    pub resource: String,
    pub action: String,
}

#[cfg(test)]
mod tests {
    use super::*;
    use genies::ddd::event::DomainEvent;

    #[test]
    fn test_user_created_event_type() {
        let event = UserCreatedEvent::default();
        assert_eq!(event.event_type(), "auth.user.created");
        assert_eq!(event.event_type_version(), "V1");
        assert_eq!(event.event_source(), "auth-admin");
    }

    #[test]
    fn test_role_updated_event_type() {
        let event = RoleUpdatedEvent::default();
        assert_eq!(event.event_type(), "auth.role.updated");
    }

    #[test]
    fn test_permission_deleted_event_type() {
        let event = PermissionDeletedEvent::default();
        assert_eq!(event.event_type(), "auth.permission.deleted");
    }

    #[test]
    fn test_user_role_assigned_event_type() {
        let event = UserRoleAssignedEvent::default();
        assert_eq!(event.event_type(), "auth.user_role.assigned");
    }

    #[test]
    fn test_json_roundtrip() {
        let event = UserCreatedEvent {
            id: 1,
            username: "admin".to_string(),
            password_hash: "hash123".to_string(),
            display_name: "Admin".to_string(),
            email: "admin@test.com".to_string(),
            phone: "12345678901".to_string(),
            department_id: "dept1".to_string(),
            department_name: "管理部".to_string(),
            status: 1,
        };

        let json = serde_json::to_string(&event).unwrap();
        let restored: UserCreatedEvent = serde_json::from_str(&json).unwrap();
        assert_eq!(restored.id, 1);
        assert_eq!(restored.username, "admin");
        assert_eq!(restored.display_name, "Admin");
    }
}
