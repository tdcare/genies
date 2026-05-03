//! 用户域服务
//!
//! 封装用户聚合根的"持久化 + 事件发布"原子操作，
//! 所有方法内部自行管理事务边界。

use crate::domain::aggregate::user::UserAggregate;
use crate::domain::entity::user_entity::{AdminUser, UserRole};
use genies::context::CONTEXT;
use genies::ddd::DomainEventPublisher::publish;
use genies_auth::event::*;

pub struct UserDomainService;

impl UserDomainService {
    /// 创建用户（insert + 发布 UserCreatedEvent）
    pub async fn create(
        user: &AdminUser,
        event: UserCreatedEvent,
    ) -> Result<(), String> {
        let rb = &CONTEXT.rbatis;
        let mut tx = rb.acquire_begin().await.map_err(|e| e.to_string())?;

        AdminUser::create_user(
            &tx,
            &user.username,
            &user.password_hash,
            &user.display_name,
            user.email.as_deref().unwrap_or(""),
            user.phone.as_deref().unwrap_or(""),
            user.avatar.as_deref().unwrap_or(""),
            &user.status,
        ).await.map_err(|e| e.to_string())?;

        let aggregate = UserAggregate::new(event.id);
        publish(&mut tx, &aggregate, Box::new(event)).await;

        tx.commit().await.map_err(|e| e.to_string())?;
        Ok(())
    }

    /// 更新用户（update + 发布 UserUpdatedEvent）
    pub async fn update(
        id: i64,
        username: &str,
        display_name: &str,
        email: &str,
        phone: &str,
        status: i8,
        event: UserUpdatedEvent,
    ) -> Result<(), String> {
        let rb = &CONTEXT.rbatis;
        let mut tx = rb.acquire_begin().await.map_err(|e| e.to_string())?;

        AdminUser::update_by_id(&tx, &id, username, display_name, email, phone, &status)
            .await.map_err(|e| e.to_string())?;

        let aggregate = UserAggregate::new(id);
        publish(&mut tx, &aggregate, Box::new(event)).await;

        tx.commit().await.map_err(|e| e.to_string())?;
        Ok(())
    }

    /// 删除用户（delete + 发布 UserDeletedEvent）
    pub async fn delete(
        id: i64,
        event: UserDeletedEvent,
    ) -> Result<(), String> {
        let rb = &CONTEXT.rbatis;
        let mut tx = rb.acquire_begin().await.map_err(|e| e.to_string())?;

        AdminUser::delete_by_id(&tx, &id)
            .await.map_err(|e| e.to_string())?;

        let aggregate = UserAggregate::new(id);
        publish(&mut tx, &aggregate, Box::new(event)).await;

        tx.commit().await.map_err(|e| e.to_string())?;
        Ok(())
    }

    /// 分配角色（insert 关联 + 发布 UserRoleAssignedEvent）
    pub async fn assign_role(
        user_id: i64,
        role_id: i64,
        event: UserRoleAssignedEvent,
    ) -> Result<(), String> {
        let rb = &CONTEXT.rbatis;
        let mut tx = rb.acquire_begin().await.map_err(|e| e.to_string())?;

        UserRole::assign(&tx, &user_id, &role_id)
            .await.map_err(|e| e.to_string())?;

        let aggregate = UserAggregate::new(user_id);
        publish(&mut tx, &aggregate, Box::new(event)).await;

        tx.commit().await.map_err(|e| e.to_string())?;
        Ok(())
    }

    /// 撤销角色（delete 关联 + 发布 UserRoleRevokedEvent）
    pub async fn revoke_role(
        user_id: i64,
        role_id: i64,
        event: UserRoleRevokedEvent,
    ) -> Result<(), String> {
        let rb = &CONTEXT.rbatis;
        let mut tx = rb.acquire_begin().await.map_err(|e| e.to_string())?;

        UserRole::revoke(&tx, &user_id, &role_id)
            .await.map_err(|e| e.to_string())?;

        let aggregate = UserAggregate::new(user_id);
        publish(&mut tx, &aggregate, Box::new(event)).await;

        tx.commit().await.map_err(|e| e.to_string())?;
        Ok(())
    }
}
