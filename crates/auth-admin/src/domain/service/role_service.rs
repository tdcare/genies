//! 角色域服务
//!
//! 封装角色聚合根的"持久化 + 事件发布"原子操作，
//! 所有方法内部自行管理事务边界。

use crate::domain::aggregate::role::RoleAggregate;
use crate::domain::entity::role_entity::{AdminRole, RolePermission};
use genies::context::CONTEXT;
use genies::ddd::DomainEventPublisher::publish;
use genies_auth::event::*;

pub struct RoleDomainService;

impl RoleDomainService {
    /// 创建角色（insert + 发布 RoleCreatedEvent）
    pub async fn create(
        role: &AdminRole,
        event: RoleCreatedEvent,
    ) -> Result<(), String> {
        let rb = &CONTEXT.rbatis;
        let mut tx = rb.acquire_begin().await.map_err(|e| e.to_string())?;

        AdminRole::create_role(
            &tx,
            &role.name,
            &role.display_name,
            role.description.as_deref().unwrap_or(""),
            &role.parent_id.unwrap_or(0),
            &role.status,
        ).await.map_err(|e| e.to_string())?;

        let aggregate = RoleAggregate::new(event.id);
        publish(&mut tx, &aggregate, Box::new(event)).await;

        tx.commit().await.map_err(|e| e.to_string())?;
        Ok(())
    }

    /// 更新角色（update + 发布 RoleUpdatedEvent）
    pub async fn update(
        id: i64,
        name: &str,
        display_name: &str,
        description: &str,
        status: i8,
        event: RoleUpdatedEvent,
    ) -> Result<(), String> {
        let rb = &CONTEXT.rbatis;
        let mut tx = rb.acquire_begin().await.map_err(|e| e.to_string())?;

        AdminRole::update_by_id(&tx, &id, name, display_name, description, &status)
            .await.map_err(|e| e.to_string())?;

        let aggregate = RoleAggregate::new(id);
        publish(&mut tx, &aggregate, Box::new(event)).await;

        tx.commit().await.map_err(|e| e.to_string())?;
        Ok(())
    }

    /// 删除角色（delete + 发布 RoleDeletedEvent）
    pub async fn delete(
        id: i64,
        event: RoleDeletedEvent,
    ) -> Result<(), String> {
        let rb = &CONTEXT.rbatis;
        let mut tx = rb.acquire_begin().await.map_err(|e| e.to_string())?;

        AdminRole::delete_by_id(&tx, &id)
            .await.map_err(|e| e.to_string())?;

        let aggregate = RoleAggregate::new(id);
        publish(&mut tx, &aggregate, Box::new(event)).await;

        tx.commit().await.map_err(|e| e.to_string())?;
        Ok(())
    }

    /// 分配权限（insert 关联 + 发布 RolePermissionAssignedEvent）
    pub async fn assign_permission(
        role_id: i64,
        permission_id: i64,
        event: RolePermissionAssignedEvent,
    ) -> Result<(), String> {
        let rb = &CONTEXT.rbatis;
        let mut tx = rb.acquire_begin().await.map_err(|e| e.to_string())?;

        RolePermission::assign(&tx, &role_id, &permission_id)
            .await.map_err(|e| e.to_string())?;

        let aggregate = RoleAggregate::new(role_id);
        publish(&mut tx, &aggregate, Box::new(event)).await;

        tx.commit().await.map_err(|e| e.to_string())?;
        Ok(())
    }

    /// 撤销权限（delete 关联 + 发布 RolePermissionRevokedEvent）
    pub async fn revoke_permission(
        role_id: i64,
        permission_id: i64,
        event: RolePermissionRevokedEvent,
    ) -> Result<(), String> {
        let rb = &CONTEXT.rbatis;
        let mut tx = rb.acquire_begin().await.map_err(|e| e.to_string())?;

        RolePermission::revoke(&tx, &role_id, &permission_id)
            .await.map_err(|e| e.to_string())?;

        let aggregate = RoleAggregate::new(role_id);
        publish(&mut tx, &aggregate, Box::new(event)).await;

        tx.commit().await.map_err(|e| e.to_string())?;
        Ok(())
    }
}
