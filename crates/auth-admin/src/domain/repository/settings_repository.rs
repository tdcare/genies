//! 设置 Repository

use rbatis::executor::Executor;
use rbatis::py_sql;
use rbatis::rbdc::db::ExecResult;
use crate::domain::entity::settings_entity::AdminSetting;

impl AdminSetting {
    /// 按 key 查询单条设置
    #[py_sql("SELECT * FROM auth_admin_settings WHERE setting_key = #{key}")]
    pub async fn find_by_key(
        rb: &dyn Executor,
        key: &str,
    ) -> rbatis::Result<Option<AdminSetting>> {
        impled!()
    }

    /// 列出全部设置
    #[py_sql("SELECT * FROM auth_admin_settings ORDER BY id")]
    pub async fn list_all(
        rb: &dyn Executor,
    ) -> rbatis::Result<Vec<AdminSetting>> {
        impled!()
    }

    /// 插入或更新设置（按 setting_key 唯一键）
    #[py_sql("
        INSERT INTO auth_admin_settings (setting_key, setting_value, description)
        VALUES (#{key}, #{value}, #{description})
        ON DUPLICATE KEY UPDATE
            setting_value = VALUES(setting_value),
            description = VALUES(description)
    ")]
    pub async fn upsert(
        rb: &dyn Executor,
        key: &str,
        value: &str,
        description: &str,
    ) -> rbatis::Result<ExecResult> {
        impled!()
    }
}
