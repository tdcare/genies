//! 应用实例 Repository
//!
//! 使用 RBatis `#[py_sql]` 宏实现应用实例表的自定义查询。

use rbatis::executor::Executor;
use rbatis::py_sql;
use rbatis::rbdc::db::ExecResult;
use crate::domain::entity::app_instance_entity::AppInstanceEntity;

impl AppInstanceEntity {
    #[py_sql("SELECT * FROM auth_app_instances WHERE app_name = #{app_name}")]
    pub async fn select_by_app_name(rb: &dyn Executor, app_name: &str) -> rbatis::Result<Vec<AppInstanceEntity>> {
        impled!()
    }

    #[py_sql("SELECT * FROM auth_app_instances WHERE instance_id = #{instance_id}")]
    pub async fn select_by_instance_id(rb: &dyn Executor, instance_id: &i64) -> rbatis::Result<Vec<AppInstanceEntity>> {
        impled!()
    }

    #[py_sql("
        SELECT * FROM auth_app_instances
        WHERE 1=1
        if keyword != null && keyword != '':
            ` AND app_name LIKE concat('%',#{keyword},'%')`
        ` ORDER BY id DESC`
        ` LIMIT #{size} OFFSET #{offset}`
    ")]
    pub async fn select_all_instances(rb: &dyn Executor, keyword: &str, offset: &u64, size: &u64) -> rbatis::Result<Vec<AppInstanceEntity>> {
        impled!()
    }

    #[py_sql("
        SELECT COUNT(*) FROM auth_app_instances
        WHERE app_name = #{app_name} AND status = 1
    ")]
    pub async fn count_online_by_app_name(rb: &dyn Executor, app_name: &str) -> rbatis::Result<u64> {
        impled!()
    }

    #[py_sql("
        UPDATE auth_app_instances
        SET last_heartbeat_at = NOW(), status = 1
        WHERE instance_id = #{instance_id}
    ")]
    pub async fn update_heartbeat(rb: &dyn Executor, instance_id: &i64) -> rbatis::Result<ExecResult> {
        impled!()
    }

    #[py_sql("
        UPDATE auth_app_instances
        SET status = 0
        WHERE status = 1 AND last_heartbeat_at < DATE_SUB(NOW(), INTERVAL #{threshold_seconds} SECOND)
    ")]
    pub async fn mark_stale_offline(rb: &dyn Executor, threshold_seconds: &i64) -> rbatis::Result<ExecResult> {
        impled!()
    }

    #[py_sql("
        SELECT COUNT(*) FROM auth_app_instances
        WHERE app_name = #{app_name}
    ")]
    pub async fn count_by_app_name(rb: &dyn Executor, app_name: &str) -> rbatis::Result<u64> {
        impled!()
    }
}
