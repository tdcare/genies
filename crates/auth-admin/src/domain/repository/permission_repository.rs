//! 权限 Repository — 使用 #[py_sql] 宏

use rbatis::executor::Executor;
use rbatis::py_sql;
use rbatis::rbdc::db::ExecResult;
use crate::domain::entity::permission_entity::AdminPermission;

// CRUD 宏已在 AdminPermission 定义处调用: insert, select_by_column, update_by_column, delete_by_column

impl AdminPermission {
    /// 查询所有权限，按 resource, action 排序
    #[py_sql("SELECT * FROM auth_admin_permissions ORDER BY resource, action")]
    pub async fn list_all(rb: &dyn Executor) -> rbatis::Result<Vec<AdminPermission>> {
        impled!()
    }

    /// 按 ID 查询权限
    #[py_sql("SELECT * FROM auth_admin_permissions WHERE id = #{id}")]
    pub async fn find_by_id(rb: &dyn Executor, id: &i64) -> rbatis::Result<Option<AdminPermission>> {
        impled!()
    }

    /// 插入权限
    #[py_sql("INSERT INTO auth_admin_permissions (name, resource, action, description, status) VALUES (#{name}, #{resource}, #{action}, #{description}, #{status})")]
    pub async fn insert_permission(
        rb: &dyn Executor,
        name: &str,
        resource: &str,
        action: &str,
        description: &str,
        status: &i8,
    ) -> rbatis::Result<ExecResult> {
        impled!()
    }

    /// 按 ID 更新权限
    #[py_sql("UPDATE auth_admin_permissions SET name=#{name}, resource=#{resource}, action=#{action}, description=#{description}, status=#{status} WHERE id=#{id}")]
    pub async fn update_by_id(
        rb: &dyn Executor,
        id: &i64,
        name: &str,
        resource: &str,
        action: &str,
        description: &str,
        status: &i8,
    ) -> rbatis::Result<ExecResult> {
        impled!()
    }

    /// 按 ID 删除权限
    #[py_sql("DELETE FROM auth_admin_permissions WHERE id = #{id}")]
    pub async fn delete_by_id(rb: &dyn Executor, id: &i64) -> rbatis::Result<ExecResult> {
        impled!()
    }
}
