//! Auth Dapr 事件订阅处理
//!
//! 订阅来自 auth-admin 服务的领域事件，自动同步本地 Casbin 规则。
//! 每个事件 handler 使用 `#[topic]` 宏注册为 Dapr 订阅者，
//! 自动具备 Redis 幂等消费和事务管理能力。
//!
//! # 事件 Topic 命名规范
//! - `auth.user.{action}` — 用户事件（仅日志，用户管理由 auth-admin 负责）
//! - `auth.role.{action}` — 角色事件 → casbin_rules g 规则
//! - `auth.permission.{action}` — 权限事件 → casbin_rules p 规则
//! - `auth.user_role.{action}` — 用户-角色关联 → casbin_rules g 规则
//! - `auth.role_permission.{action}` — 角色-权限关联 → casbin_rules p 规则

use crate::event::*;

// ============================================================================
// 用户事件处理（仅日志记录）
// ============================================================================

#[genies_derive::topic(name = "auth.user.created", pubsub = "messagebus")]
pub async fn handle_user_created(tx: &mut dyn Executor, event: UserCreatedEvent) -> anyhow::Result<()> {
    log::info!("[Auth Event] 用户已创建: id={}, username={}", event.id, event.username);
    Ok(())
}

#[genies_derive::topic(name = "auth.user.updated", pubsub = "messagebus")]
pub async fn handle_user_updated(tx: &mut dyn Executor, event: UserUpdatedEvent) -> anyhow::Result<()> {
    log::info!("[Auth Event] 用户已更新: id={}", event.id);
    Ok(())
}

#[genies_derive::topic(name = "auth.user.deleted", pubsub = "messagebus")]
pub async fn handle_user_deleted(tx: &mut dyn Executor, event: UserDeletedEvent) -> anyhow::Result<()> {
    // 清理 casbin_rules 中该用户的所有角色关联（g 规则）
    genies::context::CONTEXT.rbatis.exec(
        "DELETE FROM casbin_rules WHERE ptype = 'g' AND v0 = ?",
        vec![rbs::value!(&event.username)],
    ).await?;
    log::info!("[Auth Event] 用户已删除: id={}, username={}", event.id, event.username);
    Ok(())
}

// ============================================================================
// 角色事件处理（操作 casbin_rules）
// ============================================================================

#[genies_derive::topic(name = "auth.role.created", pubsub = "messagebus")]
pub async fn handle_role_created(tx: &mut dyn Executor, event: RoleCreatedEvent) -> anyhow::Result<()> {
    log::info!("[Auth Event] 角色已创建: id={}, name={}", event.id, event.name);
    Ok(())
}

#[genies_derive::topic(name = "auth.role.updated", pubsub = "messagebus")]
pub async fn handle_role_updated(tx: &mut dyn Executor, event: RoleUpdatedEvent) -> anyhow::Result<()> {
    log::info!("[Auth Event] 角色已更新: id={}, name={}", event.id, event.name);
    Ok(())
}

/// 删除 casbin_rules 中所有关联该角色的 g 和 p 规则
#[genies_derive::topic(name = "auth.role.deleted", pubsub = "messagebus")]
pub async fn handle_role_deleted(tx: &mut dyn Executor, event: RoleDeletedEvent) -> anyhow::Result<()> {
    genies::context::CONTEXT.rbatis.exec(
        "DELETE FROM casbin_rules WHERE ptype = 'g' AND v1 = ?",
        vec![rbs::value!(&event.name)],
    ).await?;
    genies::context::CONTEXT.rbatis.exec(
        "DELETE FROM casbin_rules WHERE ptype = 'p' AND v0 = ?",
        vec![rbs::value!(&event.name)],
    ).await?;
    log::info!("[Auth Event] 角色已删除: id={}, name={}", event.id, event.name);
    Ok(())
}

// ============================================================================
// 权限事件处理（操作 casbin_rules p 规则）
// ============================================================================

#[genies_derive::topic(name = "auth.permission.created", pubsub = "messagebus")]
pub async fn handle_permission_created(tx: &mut dyn Executor, event: PermissionCreatedEvent) -> anyhow::Result<()> {
    log::info!("[Auth Event] 权限已创建: id={}, name={}, {} {}",
        event.id, event.name, event.action, event.resource);
    Ok(())
}

#[genies_derive::topic(name = "auth.permission.updated", pubsub = "messagebus")]
pub async fn handle_permission_updated(tx: &mut dyn Executor, event: PermissionUpdatedEvent) -> anyhow::Result<()> {
    log::info!("[Auth Event] 权限已更新: id={}, name={}", event.id, event.name);
    Ok(())
}

/// 删除 casbin_rules 中匹配 resource+action 的所有 p 规则
#[genies_derive::topic(name = "auth.permission.deleted", pubsub = "messagebus")]
pub async fn handle_permission_deleted(tx: &mut dyn Executor, event: PermissionDeletedEvent) -> anyhow::Result<()> {
    genies::context::CONTEXT.rbatis.exec(
        "DELETE FROM casbin_rules WHERE ptype = 'p' AND v1 = ? AND v2 = ?",
        vec![rbs::value!(&event.resource), rbs::value!(&event.action)],
    ).await?;
    log::info!("[Auth Event] 权限已删除: id={}, name={}, {} {}",
        event.id, event.name, event.action, event.resource);
    Ok(())
}

// ============================================================================
// 关联事件处理（操作 casbin_rules g/p 规则）
// ============================================================================

/// 插入 casbin_rules g 规则
#[genies_derive::topic(name = "auth.user_role.assigned", pubsub = "messagebus")]
pub async fn handle_user_role_assigned(tx: &mut dyn Executor, event: UserRoleAssignedEvent) -> anyhow::Result<()> {
    genies::context::CONTEXT.rbatis.exec(
        "INSERT INTO casbin_rules (ptype, v0, v1) VALUES ('g', ?, ?)",
        vec![rbs::value!(&event.username), rbs::value!(&event.role_name)],
    ).await?;
    log::info!("[Auth Event] 用户角色已分配: user_id={}, role={}", event.user_id, event.role_name);
    Ok(())
}

/// 删除 casbin_rules g 规则
#[genies_derive::topic(name = "auth.user_role.revoked", pubsub = "messagebus")]
pub async fn handle_user_role_revoked(tx: &mut dyn Executor, event: UserRoleRevokedEvent) -> anyhow::Result<()> {
    genies::context::CONTEXT.rbatis.exec(
        "DELETE FROM casbin_rules WHERE ptype = 'g' AND v0 = ? AND v1 = ?",
        vec![rbs::value!(&event.username), rbs::value!(&event.role_name)],
    ).await?;
    log::info!("[Auth Event] 用户角色已移除: user_id={}, role={}", event.user_id, event.role_name);
    Ok(())
}

/// 插入 casbin_rules p 规则
#[genies_derive::topic(name = "auth.role_permission.assigned", pubsub = "messagebus")]
pub async fn handle_role_permission_assigned(tx: &mut dyn Executor, event: RolePermissionAssignedEvent) -> anyhow::Result<()> {
    genies::context::CONTEXT.rbatis.exec(
        "INSERT INTO casbin_rules (ptype, v0, v1, v2, v3) VALUES ('p', ?, ?, ?, 'allow')",
        vec![rbs::value!(&event.role_name), rbs::value!(&event.resource), rbs::value!(&event.action)],
    ).await?;
    log::info!("[Auth Event] 角色权限已分配: role={}, {} {}",
        event.role_name, event.action, event.resource);
    Ok(())
}

/// 删除 casbin_rules p 规则
#[genies_derive::topic(name = "auth.role_permission.revoked", pubsub = "messagebus")]
pub async fn handle_role_permission_revoked(tx: &mut dyn Executor, event: RolePermissionRevokedEvent) -> anyhow::Result<()> {
    genies::context::CONTEXT.rbatis.exec(
        "DELETE FROM casbin_rules WHERE ptype = 'p' AND v0 = ? AND v1 = ? AND v2 = ?",
        vec![rbs::value!(&event.role_name), rbs::value!(&event.resource), rbs::value!(&event.action)],
    ).await?;
    log::info!("[Auth Event] 角色权限已移除: role={}, {} {}",
        event.role_name, event.action, event.resource);
    Ok(())
}

