//! 部门 Repository — 使用 #[py_sql] 宏

use rbatis::executor::Executor;
use rbatis::py_sql;
use rbatis::rbdc::db::ExecResult;
use crate::domain::entity::department_entity::AdminDepartment;

// CRUD 宏已在 AdminDepartment 定义处调用: insert, select_by_column, update_by_column, delete_by_column

impl AdminDepartment {
    /// 查询所有部门，按 parent_id, sort_order 排序
    #[py_sql("SELECT * FROM auth_admin_departments ORDER BY parent_id, sort_order")]
    pub async fn list_all(rb: &dyn Executor) -> rbatis::Result<Vec<AdminDepartment>> {
        impled!()
    }

    /// 按 ID 查询部门
    #[py_sql("SELECT * FROM auth_admin_departments WHERE id = #{id}")]
    pub async fn find_by_id(rb: &dyn Executor, id: &i64) -> rbatis::Result<Option<AdminDepartment>> {
        impled!()
    }

    /// 插入部门
    #[py_sql("INSERT INTO auth_admin_departments (name, parent_id, sort_order, description, status) VALUES (#{name}, #{parent_id}, #{sort_order}, #{description}, #{status})")]
    pub async fn insert_department(
        rb: &dyn Executor,
        name: &str,
        parent_id: &i64,
        sort_order: &i32,
        description: &str,
        status: &i8,
    ) -> rbatis::Result<ExecResult> {
        impled!()
    }

    /// 按 ID 更新部门
    #[py_sql("UPDATE auth_admin_departments SET name=#{name}, parent_id=#{parent_id}, sort_order=#{sort_order}, description=#{description}, status=#{status} WHERE id=#{id}")]
    pub async fn update_by_id(
        rb: &dyn Executor,
        id: &i64,
        name: &str,
        parent_id: &i64,
        sort_order: &i32,
        description: &str,
        status: &i8,
    ) -> rbatis::Result<ExecResult> {
        impled!()
    }

    /// 按 ID 删除部门
    #[py_sql("DELETE FROM auth_admin_departments WHERE id = #{id}")]
    pub async fn delete_by_id(rb: &dyn Executor, id: &i64) -> rbatis::Result<ExecResult> {
        impled!()
    }

    /// 移动部门（修改 parent_id）
    #[py_sql("UPDATE auth_admin_departments SET parent_id=#{new_parent_id} WHERE id=#{id}")]
    pub async fn move_dept(rb: &dyn Executor, id: &i64, new_parent_id: &i64) -> rbatis::Result<ExecResult> {
        impled!()
    }
}
