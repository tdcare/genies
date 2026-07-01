//! 应用 Repository
//!
//! 使用 RBatis `#[py_sql]` 宏实现应用表的自定义查询。

use rbatis::executor::Executor;
use rbatis::py_sql;
use rbatis::rbdc::db::ExecResult;
use crate::domain::entity::application_entity::ApplicationEntity;

impl ApplicationEntity {
    #[py_sql("SELECT * FROM auth_applications WHERE id = #{id}")]
    pub async fn find_by_id(rb: &dyn Executor, id: &i64) -> rbatis::Result<Option<ApplicationEntity>> {
        impled!()
    }

    #[py_sql("SELECT * FROM auth_applications WHERE app_name = #{app_name}")]
    pub async fn find_by_app_name(rb: &dyn Executor, app_name: &str) -> rbatis::Result<Option<ApplicationEntity>> {
        impled!()
    }

    #[py_sql("
        SELECT * FROM auth_applications
        WHERE 1=1
        if keyword != null && keyword != '':
            ` AND (app_name LIKE concat('%',#{keyword},'%') OR display_name LIKE concat('%',#{keyword},'%'))`
        ` ORDER BY id DESC`
    ")]
    pub async fn list(rb: &dyn Executor, keyword: &str) -> rbatis::Result<Vec<ApplicationEntity>> {
        impled!()
    }

    #[py_sql("
        SELECT COUNT(*) AS cnt FROM auth_applications
        WHERE 1=1
        if keyword != null && keyword != '':
            ` AND (app_name LIKE concat('%',#{keyword},'%') OR display_name LIKE concat('%',#{keyword},'%'))`
    ")]
    pub async fn count(rb: &dyn Executor, keyword: &str) -> rbatis::Result<u64> {
        impled!()
    }

    #[py_sql("INSERT INTO auth_applications (app_name, display_name, description, base_url, status) VALUES (#{app_name}, #{display_name}, #{description}, #{base_url}, #{status})")]
    pub async fn insert_app(
        rb: &dyn Executor,
        app_name: &str,
        display_name: &str,
        description: &str,
        base_url: &str,
        status: &i8,
    ) -> rbatis::Result<ExecResult> {
        impled!()
    }

    #[py_sql("UPDATE auth_applications SET app_name=#{app_name}, display_name=#{display_name}, description=#{description}, base_url=#{base_url}, status=#{status} WHERE id=#{id}")]
    pub async fn update_by_id(
        rb: &dyn Executor,
        id: &i64,
        app_name: &str,
        display_name: &str,
        description: &str,
        base_url: &str,
        status: &i8,
    ) -> rbatis::Result<ExecResult> {
        impled!()
    }

    #[py_sql("DELETE FROM auth_applications WHERE id = #{id}")]
    pub async fn delete_by_id(rb: &dyn Executor, id: &i64) -> rbatis::Result<ExecResult> {
        impled!()
    }
}
