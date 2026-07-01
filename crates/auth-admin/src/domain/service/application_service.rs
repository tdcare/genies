//! 应用域服务
//!
//! 封装应用聚合根的持久化操作，
//! 所有方法内部自行管理事务边界。

use crate::domain::entity::application_entity::ApplicationEntity;
use genies::context::CONTEXT;

pub struct ApplicationDomainService;

impl ApplicationDomainService {
    /// 创建应用
    pub async fn create(
        app_name: &str,
        display_name: &str,
        description: &str,
        base_url: &str,
        status: &i8,
    ) -> Result<(), String> {
        let rb = &CONTEXT.rbatis;
        let tx = rb.acquire_begin().await.map_err(|e| e.to_string())?;

        ApplicationEntity::insert_app(&tx, app_name, display_name, description, base_url, status)
            .await
            .map_err(|e| e.to_string())?;

        tx.commit().await.map_err(|e| e.to_string())?;
        Ok(())
    }

    /// 更新应用
    pub async fn update(
        id: i64,
        app_name: &str,
        display_name: &str,
        description: &str,
        base_url: &str,
        status: &i8,
    ) -> Result<(), String> {
        let rb = &CONTEXT.rbatis;
        let tx = rb.acquire_begin().await.map_err(|e| e.to_string())?;

        ApplicationEntity::update_by_id(&tx, &id, app_name, display_name, description, base_url, status)
            .await
            .map_err(|e| e.to_string())?;

        tx.commit().await.map_err(|e| e.to_string())?;
        Ok(())
    }

    /// 删除应用
    pub async fn delete(id: i64) -> Result<(), String> {
        let rb = &CONTEXT.rbatis;
        let tx = rb.acquire_begin().await.map_err(|e| e.to_string())?;

        ApplicationEntity::delete_by_id(&tx, &id)
            .await
            .map_err(|e| e.to_string())?;

        tx.commit().await.map_err(|e| e.to_string())?;
        Ok(())
    }
}
