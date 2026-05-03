//! 角色与角色-权限关联 Repository
//!
//! 从 role_entity.rs 迁移的 SQL 方法，使用 RBatis `#[py_sql]` 宏重写。

use rbatis::executor::Executor;
use rbatis::py_sql;
use rbatis::rbdc::db::ExecResult;
use crate::domain::entity::role_entity::{AdminRole, RolePermission};
use crate::domain::entity::permission_entity::AdminPermission;

// ============================================================================
// AdminRole Repository
// ============================================================================

impl AdminRole {
    #[py_sql("INSERT INTO auth_admin_roles (name, display_name, description, parent_id, status) VALUES (#{name}, #{display_name}, #{description}, #{parent_id}, #{status})")]
    pub async fn create_role(
        rb: &dyn Executor,
        name: &str,
        display_name: &str,
        description: &str,
        parent_id: &i64,
        status: &i8,
    ) -> rbatis::Result<ExecResult> {
        impled!()
    }

    #[py_sql("SELECT * FROM auth_admin_roles ORDER BY parent_id, id")]
    pub async fn list_all(rb: &dyn Executor) -> rbatis::Result<Vec<AdminRole>> {
        impled!()
    }

    #[py_sql("SELECT * FROM auth_admin_roles WHERE id = #{id}")]
    pub async fn find_by_id(rb: &dyn Executor, id: &i64) -> rbatis::Result<Option<AdminRole>> {
        impled!()
    }

    #[py_sql("SELECT * FROM auth_admin_roles WHERE name = #{name}")]
    pub async fn find_by_name(rb: &dyn Executor, name: &str) -> rbatis::Result<Option<AdminRole>> {
        impled!()
    }

    #[py_sql("UPDATE auth_admin_roles SET name=#{name}, display_name=#{display_name}, description=#{description}, status=#{status} WHERE id=#{id}")]
    pub async fn update_by_id(rb: &dyn Executor, id: &i64, name: &str, display_name: &str, description: &str, status: &i8) -> rbatis::Result<ExecResult> {
        impled!()
    }

    #[py_sql("DELETE FROM auth_admin_roles WHERE id = #{id}")]
    pub async fn delete_by_id(rb: &dyn Executor, id: &i64) -> rbatis::Result<ExecResult> {
        impled!()
    }
}

// ============================================================================
// RolePermission Repository
// ============================================================================

impl RolePermission {
    #[py_sql("INSERT IGNORE INTO auth_admin_role_permissions (role_id, permission_id) VALUES (#{role_id}, #{permission_id})")]
    pub async fn assign(rb: &dyn Executor, role_id: &i64, permission_id: &i64) -> rbatis::Result<ExecResult> {
        impled!()
    }

    #[py_sql("DELETE FROM auth_admin_role_permissions WHERE role_id=#{role_id} AND permission_id=#{permission_id}")]
    pub async fn revoke(rb: &dyn Executor, role_id: &i64, permission_id: &i64) -> rbatis::Result<ExecResult> {
        impled!()
    }

    #[py_sql("SELECT p.* FROM auth_admin_permissions p INNER JOIN auth_admin_role_permissions rp ON p.id = rp.permission_id WHERE rp.role_id = #{role_id}")]
    pub async fn list_by_role(rb: &dyn Executor, role_id: &i64) -> rbatis::Result<Vec<AdminPermission>> {
        impled!()
    }

    #[py_sql("SELECT DISTINCT p.* FROM auth_admin_permissions p INNER JOIN auth_admin_role_permissions rp ON p.id = rp.permission_id INNER JOIN auth_admin_user_roles ur ON rp.role_id = ur.role_id WHERE ur.user_id = #{user_id}")]
    pub async fn list_by_user(rb: &dyn Executor, user_id: &i64) -> rbatis::Result<Vec<AdminPermission>> {
        impled!()
    }
}
