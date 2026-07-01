//! 应用管理 — 应用服务层
//!
//! 封装应用 CRUD 用例，协调领域实体与领域服务。

use crate::domain::entity::application_entity::ApplicationEntity;
use crate::domain::service::ApplicationDomainService;
use crate::application::dto::PageResult;
use crate::interfaces::dto::application_dto::ApplicationVO;
use genies::context::CONTEXT;

pub struct ApplicationAppService;

impl ApplicationAppService {
    /// 分页列表
    pub async fn list_apps(
        page: u64,
        size: u64,
        keyword: &str,
    ) -> Result<PageResult<ApplicationVO>, String> {
        let rb = &CONTEXT.rbatis;
        let page = page.max(1);
        let size = size.min(100);

        let total = ApplicationEntity::count(rb, keyword)
            .await
            .map_err(|e| e.to_string())?;

        let all = ApplicationEntity::list(rb, keyword)
            .await
            .map_err(|e| e.to_string())?;

        // 内存分页
        let offset = ((page - 1) * size) as usize;
        let list: Vec<ApplicationVO> = all.into_iter()
            .skip(offset)
            .take(size as usize)
            .map(|e| e.into())
            .collect();

        Ok(PageResult {
            total,
            page,
            size,
            list,
        })
    }

    /// 获取详情
    pub async fn get_app(id: i64) -> Result<ApplicationEntity, String> {
        let rb = &CONTEXT.rbatis;
        ApplicationEntity::find_by_id(rb, &id)
            .await
            .map_err(|e| e.to_string())?
            .ok_or_else(|| "应用不存在".to_string())
    }

    /// 创建应用
    pub async fn create_app(
        app_name: &str,
        display_name: &str,
        description: &str,
        base_url: &str,
        status: i8,
    ) -> Result<serde_json::Value, String> {
        let rb = &CONTEXT.rbatis;

        if app_name.is_empty() {
            return Err("应用标识不能为空".to_string());
        }
        if base_url.is_empty() {
            return Err("微服务地址不能为空".to_string());
        }

        // 检查唯一性
        if let Ok(Some(_)) = ApplicationEntity::find_by_app_name(rb, app_name).await {
            return Err("应用标识已存在".to_string());
        }

        ApplicationDomainService::create(app_name, display_name, description, base_url, &status).await?;

        // 查回新建的记录获取 ID
        let app = ApplicationEntity::find_by_app_name(rb, app_name)
            .await
            .ok()
            .flatten();

        Ok(serde_json::json!({"id": app.and_then(|a| a.id)}))
    }

    /// 更新应用
    pub async fn update_app(
        id: i64,
        app_name: &str,
        display_name: &str,
        description: &str,
        base_url: &str,
        status: i8,
    ) -> Result<(), String> {
        // 检查存在性
        let _ = Self::get_app(id).await?;

        ApplicationDomainService::update(id, app_name, display_name, description, base_url, &status).await
    }

    /// 删除应用
    pub async fn delete_app(id: i64) -> Result<(), String> {
        let _ = Self::get_app(id).await?;
        ApplicationDomainService::delete(id).await
    }
}
