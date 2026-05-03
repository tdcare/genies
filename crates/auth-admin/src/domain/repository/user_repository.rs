//! 用户与用户-角色关联 Repository
//!
//! 从 user_entity.rs 迁移的 SQL 方法，使用 RBatis `#[py_sql]` 宏重写。

use rbatis::executor::Executor;
use rbatis::py_sql;
use rbatis::rbdc::db::ExecResult;
use crate::domain::entity::user_entity::{AdminUser, UserRole, UserRoleMapping};
use crate::domain::entity::role_entity::AdminRole;

// ============================================================================
// AdminUser Repository
// ============================================================================

impl AdminUser {
    #[py_sql("INSERT INTO auth_admin_users (username, password_hash, display_name, email, phone, avatar, status) VALUES (#{username}, #{password_hash}, #{display_name}, #{email}, #{phone}, #{avatar}, #{status})")]
    pub async fn create_user(
        rb: &dyn Executor,
        username: &str,
        password_hash: &str,
        display_name: &str,
        email: &str,
        phone: &str,
        avatar: &str,
        status: &i8,
    ) -> rbatis::Result<ExecResult> {
        impled!()
    }

    #[py_sql("SELECT * FROM auth_admin_users WHERE username = #{username}")]
    pub async fn find_by_username(rb: &dyn Executor, username: &str) -> rbatis::Result<Option<AdminUser>> {
        impled!()
    }

    #[py_sql("SELECT * FROM auth_admin_users WHERE id = #{id}")]
    pub async fn find_by_id(rb: &dyn Executor, id: &i64) -> rbatis::Result<Option<AdminUser>> {
        impled!()
    }

    #[py_sql("
        SELECT * FROM auth_admin_users
        WHERE 1=1
        if keyword != null && keyword != '':
            ` AND (username LIKE concat('%',#{keyword},'%') OR display_name LIKE concat('%',#{keyword},'%') OR email LIKE concat('%',#{keyword},'%'))`
        if status != null:
            ` AND status = #{status}`
        ` ORDER BY id DESC`
    ")]
    pub async fn list(rb: &dyn Executor, keyword: &str, status: Option<i8>) -> rbatis::Result<Vec<AdminUser>> {
        impled!()
    }

    #[py_sql("
        SELECT COUNT(*) AS cnt FROM auth_admin_users
        WHERE 1=1
        if keyword != null && keyword != '':
            ` AND (username LIKE concat('%',#{keyword},'%') OR display_name LIKE concat('%',#{keyword},'%') OR email LIKE concat('%',#{keyword},'%'))`
        if status != null:
            ` AND status = #{status}`
    ")]
    pub async fn count(rb: &dyn Executor, keyword: &str, status: Option<i8>) -> rbatis::Result<u64> {
        impled!()
    }

    #[py_sql("UPDATE auth_admin_users SET username=#{username}, display_name=#{display_name}, email=#{email}, phone=#{phone}, status=#{status} WHERE id=#{id}")]
    pub async fn update_by_id(rb: &dyn Executor, id: &i64, username: &str, display_name: &str, email: &str, phone: &str, status: &i8) -> rbatis::Result<ExecResult> {
        impled!()
    }

    #[py_sql("DELETE FROM auth_admin_users WHERE id = #{id}")]
    pub async fn delete_by_id(rb: &dyn Executor, id: &i64) -> rbatis::Result<ExecResult> {
        impled!()
    }

    #[py_sql("
        DELETE FROM auth_admin_users WHERE id IN (
        trim ',':
            for _,item in ids:
                #{item},
        )
    ")]
    pub async fn batch_delete(rb: &dyn Executor, ids: &[i64]) -> rbatis::Result<ExecResult> {
        impled!()
    }

    #[py_sql("UPDATE auth_admin_users SET password_hash=#{password_hash} WHERE id=#{id}")]
    pub async fn update_password(rb: &dyn Executor, id: &i64, password_hash: &str) -> rbatis::Result<ExecResult> {
        impled!()
    }

    #[py_sql("UPDATE auth_admin_users SET status=#{status} WHERE id=#{id}")]
    pub async fn update_status(rb: &dyn Executor, id: &i64, status: &i8) -> rbatis::Result<ExecResult> {
        impled!()
    }

    #[py_sql("UPDATE auth_admin_users SET last_login_at=NOW() WHERE id=#{id}")]
    pub async fn update_last_login(rb: &dyn Executor, id: &i64) -> rbatis::Result<ExecResult> {
        impled!()
    }
}

// ============================================================================
// UserRole Repository
// ============================================================================

/// 用于 list_user_ids_by_role 查询结果映射
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct UserIdRow {
    pub user_id: i64,
}

impl UserRole {
    #[py_sql("INSERT IGNORE INTO auth_admin_user_roles (user_id, role_id) VALUES (#{user_id}, #{role_id})")]
    pub async fn assign(rb: &dyn Executor, user_id: &i64, role_id: &i64) -> rbatis::Result<ExecResult> {
        impled!()
    }

    #[py_sql("DELETE FROM auth_admin_user_roles WHERE user_id=#{user_id} AND role_id=#{role_id}")]
    pub async fn revoke(rb: &dyn Executor, user_id: &i64, role_id: &i64) -> rbatis::Result<ExecResult> {
        impled!()
    }

    #[py_sql("SELECT r.* FROM auth_admin_roles r INNER JOIN auth_admin_user_roles ur ON r.id = ur.role_id WHERE ur.user_id = #{user_id}")]
    pub async fn list_by_user(rb: &dyn Executor, user_id: &i64) -> rbatis::Result<Vec<AdminRole>> {
        impled!()
    }

    #[py_sql("SELECT u.* FROM auth_admin_users u INNER JOIN auth_admin_user_roles ur ON u.id = ur.user_id WHERE ur.role_id = #{role_id}")]
    pub async fn list_by_role(rb: &dyn Executor, role_id: &i64) -> rbatis::Result<Vec<AdminUser>> {
        impled!()
    }

    #[py_sql("SELECT user_id FROM auth_admin_user_roles WHERE role_id = #{role_id}")]
    pub async fn list_user_ids_by_role(rb: &dyn Executor, role_id: &i64) -> rbatis::Result<Vec<UserIdRow>> {
        impled!()
    }

    /// 查询所有启用状态的用户-角色映射，返回 casbin 'g' 规则格式
    #[py_sql("
        SELECT 'g' AS ptype, u.username AS v0, r.name AS v1
        FROM auth_admin_user_roles ur
        INNER JOIN auth_admin_users u ON u.id = ur.user_id
        INNER JOIN auth_admin_roles r ON r.id = ur.role_id
        WHERE u.status = 1 AND r.status = 1
    ")]
    pub async fn list_active_user_roles(rb: &dyn Executor) -> rbatis::Result<Vec<UserRoleMapping>> {
        impled!()
    }
}
