//! 用户-部门关联 Repository
//!
//! 提供用户与部门多对多关联的增删查操作。

use rbatis::executor::Executor;
use rbatis::py_sql;
use rbatis::rbdc::db::ExecResult;
use serde::{Deserialize, Serialize};
use crate::domain::entity::user_department_entity::UserDepartment;

/// 部门成员数量（用于 GROUP BY 查询结果）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DepartmentMemberCount {
    pub department_id: i64,
    pub count: i64,
}

impl UserDepartment {
    /// 查询用户所属的所有部门关联
    #[py_sql("SELECT * FROM auth_admin_user_departments WHERE user_id = #{user_id}")]
    pub async fn list_by_user_id(rb: &dyn Executor, user_id: &i64) -> rbatis::Result<Vec<UserDepartment>> {
        impled!()
    }

    /// 查询部门下所有用户关联
    #[py_sql("SELECT * FROM auth_admin_user_departments WHERE department_id = #{department_id}")]
    pub async fn list_by_department_id(rb: &dyn Executor, department_id: &i64) -> rbatis::Result<Vec<UserDepartment>> {
        impled!()
    }

    /// 删除用户的所有部门关联
    #[py_sql("DELETE FROM auth_admin_user_departments WHERE user_id = #{user_id}")]
    pub async fn delete_by_user_id(rb: &dyn Executor, user_id: &i64) -> rbatis::Result<ExecResult> {
        impled!()
    }

    /// 批量插入用户-部门关联
    #[py_sql("
        INSERT INTO auth_admin_user_departments (user_id, department_id) VALUES
        trim ',':
            for _,dept_id in department_ids:
                (#{user_id}, #{dept_id}),
    ")]
    pub async fn batch_insert(rb: &dyn Executor, user_id: &i64, department_ids: &[i64]) -> rbatis::Result<ExecResult> {
        impled!()
    }

    /// 删除指定用户-部门关联
    #[py_sql("DELETE FROM auth_admin_user_departments WHERE user_id = #{user_id} AND department_id = #{department_id}")]
    pub async fn remove_user_from_department(rb: &dyn Executor, user_id: &i64, department_id: &i64) -> rbatis::Result<ExecResult> {
        impled!()
    }

    /// 查询所有部门的成员数量
    #[py_sql("SELECT department_id, COUNT(*) AS count FROM auth_admin_user_departments GROUP BY department_id")]
    pub async fn count_members_by_department(rb: &dyn Executor) -> rbatis::Result<Vec<DepartmentMemberCount>> {
        impled!()
    }
}
