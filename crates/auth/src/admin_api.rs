//! Auth 模块的 Admin API 端点
//!
//! 提供权限管理的 REST API，包括：
//! - API Schema 查询
//! - Casbin 模型定义管理
//! - 策略规则管理（policy/role/group）
//! - Enforcer 热重载
//!
//! # 端点列表
//!
//! | 端点 | 方法 | 功能 |
//! |------|------|------|
//! | `/auth/schemas` | GET | 列出所有 API Schema 和字段 |
//! | `/auth/model` | GET | 获取当前 Casbin 模型定义 |
//! | `/auth/model` | PUT | 修改 Casbin 模型定义 |
//! | `/auth/policies` | GET | 列出所有策略规则 |
//! | `/auth/policies` | POST | 添加策略规则 |
//! | `/auth/policies/{id}` | DELETE | 删除策略规则 |
//! | `/auth/roles` | GET | 列出角色分配 (g 类型) |
//! | `/auth/roles` | POST | 添加角色分配 |
//! | `/auth/roles/{id}` | DELETE | 移除角色分配 |
//! | `/auth/groups` | GET | 列出对象分组 (g2 类型) |
//! | `/auth/groups` | POST | 添加对象分组 |
//! | `/auth/groups/{id}` | DELETE | 移除对象分组 |
//! | `/auth/reload` | POST | 手动触发 Enforcer 重载 |

use std::sync::Arc;

use salvo::oapi::extract::{JsonBody, PathParam, QueryParam};
use salvo::prelude::*;
use serde::{Deserialize, Serialize};

use genies::context::CONTEXT;
use rbs::value;

use crate::enforcer_manager::EnforcerManager;

// ============================================================================
// 数据结构定义
// ============================================================================

/// 策略规则 DTO（用于添加策略）
#[derive(Debug, Serialize, Deserialize, salvo::oapi::ToSchema)]
#[salvo(schema(example = json!({"ptype": "p", "v0": "admin", "v1": "/api/*", "v2": "GET", "v3": "", "v4": "", "v5": ""})))]
pub struct PolicyDto {
    /// 策略类型: "p" (策略), "g" (角色), "g2" (分组)
    pub ptype: String,
    /// 第一个参数（通常是 subject/用户/角色）
    pub v0: String,
    /// 第二个参数（通常是 object/资源/角色）
    pub v1: String,
    /// 第三个参数（通常是 action/操作）
    #[serde(default)]
    pub v2: String,
    /// 第四个参数（可选扩展字段）
    #[serde(default)]
    pub v3: String,
    /// 第五个参数（可选扩展字段）
    #[serde(default)]
    pub v4: String,
    /// 第六个参数（可选扩展字段）
    #[serde(default)]
    pub v5: String,
}

/// 策略规则查询结果（含 id）
#[derive(Debug, Serialize, Deserialize, salvo::oapi::ToSchema)]
pub struct PolicyRecord {
    /// 数据库记录 ID
    pub id: i64,
    /// 策略类型
    pub ptype: String,
    /// 第一个参数
    pub v0: String,
    /// 第二个参数
    pub v1: String,
    /// 第三个参数
    pub v2: String,
    /// 第四个参数
    pub v3: String,
    /// 第五个参数
    pub v4: String,
    /// 第六个参数
    pub v5: String,
}

/// Casbin 模型 DTO
#[derive(Debug, Serialize, Deserialize, salvo::oapi::ToSchema)]
#[salvo(schema(example = json!({"model_name": "default", "model_text": "[request_definition]\nr = sub, obj, act", "description": "默认模型"})))]
pub struct ModelDto {
    /// 模型名称
    pub model_name: String,
    /// 模型定义文本（Casbin 模型格式）
    pub model_text: String,
    /// 模型描述
    pub description: Option<String>,
}

/// 模型查询结果（含 id）
#[derive(Debug, Serialize, Deserialize, salvo::oapi::ToSchema)]
pub struct ModelRecord {
    /// 数据库记录 ID
    pub id: i64,
    /// 模型名称
    pub model_name: String,
    /// 模型定义文本
    pub model_text: String,
    /// 模型描述
    pub description: Option<String>,
}

/// API Schema 查询结果
#[derive(Debug, Serialize, Deserialize, salvo::oapi::ToSchema)]
pub struct SchemaRecord {
    /// 数据库记录 ID
    pub id: i64,
    /// Schema 名称
    pub schema_name: String,
    /// Schema 标签
    pub schema_label: Option<String>,
    /// Schema 描述
    pub schema_description: Option<String>,
    /// 字段名称
    pub field_name: String,
    /// 字段标签
    pub field_label: Option<String>,
    /// 字段类型
    pub field_type: Option<String>,
    /// 字段描述
    pub field_description: Option<String>,
    /// 字段是否必需
    pub field_required: Option<bool>,
    /// 端点路径
    pub endpoint_path: Option<String>,
    /// 端点标签
    pub endpoint_label: Option<String>,
    /// 端点描述
    pub endpoint_description: Option<String>,
    /// 端点标签列表（JSON 格式）
    pub endpoint_tags: Option<String>,
    /// 操作 ID
    pub endpoint_operation_id: Option<String>,
    /// HTTP 方法
    pub http_method: Option<String>,
}

/// Token 响应
#[derive(Debug, Serialize, Deserialize, salvo::oapi::ToSchema)]
pub struct TokenResponse {
    /// 访问令牌
    pub access_token: String,
    /// 过期时间（秒）
    pub expires_in: i64,
    /// 令牌类型
    pub token_type: String,
}

/// 通用 API 响应
#[derive(Debug, Serialize, Deserialize, salvo::oapi::ToSchema)]
pub struct ApiResponse<T: Serialize> {
    /// 响应码，"0" 表示成功，其他表示错误
    pub code: String,
    /// 响应消息
    pub msg: String,
    /// 响应数据
    pub data: Option<T>,
}

impl<T: Serialize> ApiResponse<T> {
    /// 创建成功响应
    pub fn ok(data: T) -> Self {
        Self {
            code: "0".to_string(),
            msg: "ok".to_string(),
            data: Some(data),
        }
    }

    /// 创建错误响应
    pub fn err(msg: impl Into<String>) -> Self {
        Self {
            code: "-1".to_string(),
            msg: msg.into(),
            data: None,
        }
    }
}

// ============================================================================
// 内部反序列化结构体
// ============================================================================

/// 用于从数据库反序列化的 PolicyRecord
#[derive(Debug, Deserialize)]
struct PolicyRecordDb {
    #[serde(default)]
    id: i64,
    #[serde(default)]
    ptype: String,
    #[serde(default)]
    v0: String,
    #[serde(default)]
    v1: String,
    #[serde(default)]
    v2: String,
    #[serde(default)]
    v3: String,
    #[serde(default)]
    v4: String,
    #[serde(default)]
    v5: String,
}

impl From<PolicyRecordDb> for PolicyRecord {
    fn from(db: PolicyRecordDb) -> Self {
        PolicyRecord {
            id: db.id,
            ptype: db.ptype,
            v0: db.v0,
            v1: db.v1,
            v2: db.v2,
            v3: db.v3,
            v4: db.v4,
            v5: db.v5,
        }
    }
}

/// 用于从数据库反序列化的 SchemaRecord
#[derive(Debug, Deserialize)]
struct SchemaRecordDb {
    #[serde(default)]
    id: i64,
    #[serde(default)]
    schema_name: String,
    schema_label: Option<String>,
    schema_description: Option<String>,
    #[serde(default)]
    field_name: String,
    field_label: Option<String>,
    field_type: Option<String>,
    field_description: Option<String>,
    #[serde(default, deserialize_with = "deserialize_bool_from_int")]
    field_required: Option<bool>,
    endpoint_path: Option<String>,
    endpoint_label: Option<String>,
    endpoint_description: Option<String>,
    endpoint_tags: Option<String>,
    endpoint_operation_id: Option<String>,
    http_method: Option<String>,
}

/// 自定义反序列化函数：将数据库整数（0/1）转换为 Option<bool>
fn deserialize_bool_from_int<'de, D>(deserializer: D) -> std::result::Result<Option<bool>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    use serde::Deserialize;
    
    // 先尝试作为 Option<i8> 反序列化
    let opt_int: Option<i8> = Option::deserialize(deserializer)?;
    Ok(opt_int.map(|v| v != 0))
}

impl From<SchemaRecordDb> for SchemaRecord {
    fn from(db: SchemaRecordDb) -> Self {
        SchemaRecord {
            id: db.id,
            schema_name: db.schema_name,
            schema_label: db.schema_label,
            schema_description: db.schema_description,
            field_name: db.field_name,
            field_label: db.field_label,
            field_type: db.field_type,
            field_description: db.field_description,
            field_required: db.field_required,
            endpoint_path: db.endpoint_path,
            endpoint_label: db.endpoint_label,
            endpoint_description: db.endpoint_description,
            endpoint_tags: db.endpoint_tags,
            endpoint_operation_id: db.endpoint_operation_id,
            http_method: db.http_method,
        }
    }
}

/// 用于从数据库反序列化的 ModelRecord
#[derive(Debug, Deserialize)]
struct ModelRecordDb {
    #[serde(default)]
    id: i64,
    #[serde(default)]
    model_name: String,
    #[serde(default)]
    model_text: String,
    description: Option<String>,
}

impl From<ModelRecordDb> for ModelRecord {
    fn from(db: ModelRecordDb) -> Self {
        ModelRecord {
            id: db.id,
            model_name: db.model_name,
            model_text: db.model_text,
            description: db.description,
        }
    }
}

// ============================================================================
// Schema 端点
// ============================================================================

/// GET /auth/schemas — 列出所有 API Schema
///
/// 返回 auth_api_schemas 表中的所有 Schema 定义和字段信息
#[endpoint(tags("schemas"), summary = "列出所有 API Schema", description = "返回 auth_api_schemas 表中的所有 Schema 定义和字段信息")]
pub async fn list_schemas() -> Json<ApiResponse<Vec<SchemaRecord>>> {
    let sql = "SELECT * FROM auth_api_schemas ORDER BY schema_name, field_name";
    match CONTEXT.rbatis.query(sql, vec![]).await {
        Ok(value) => {
            match rbs::from_value::<Vec<SchemaRecordDb>>(value) {
                Ok(rows) => {
                    let records: Vec<SchemaRecord> = rows.into_iter().map(|r| r.into()).collect();
                    Json(ApiResponse::ok(records))
                }
                Err(e) => {
                    log::error!("反序列化 auth_api_schemas 失败: {}", e);
                    Json(ApiResponse::err(e.to_string()))
                }
            }
        }
        Err(e) => {
            log::error!("查询 auth_api_schemas 失败: {}", e);
            Json(ApiResponse::err(e.to_string()))
        }
    }
}

// ============================================================================
// Model 端点
// ============================================================================

/// GET /auth/model — 获取当前 Casbin 模型定义
///
/// 返回名为 "default" 的 Casbin 模型定义
#[endpoint(tags("model"), summary = "获取当前 Casbin 模型定义", description = "返回名为 default 的 Casbin 模型定义")]
pub async fn get_model() -> Json<ApiResponse<ModelRecord>> {
    let sql = "SELECT * FROM casbin_model WHERE model_name = ?";
    match CONTEXT.rbatis.query(sql, vec![value!("default")]).await {
        Ok(value) => {
            match rbs::from_value::<Vec<ModelRecordDb>>(value) {
                Ok(rows) => {
                    if let Some(row) = rows.into_iter().next() {
                        Json(ApiResponse::ok(row.into()))
                    } else {
                        Json(ApiResponse::err("未找到默认模型定义"))
                    }
                }
                Err(e) => {
                    log::error!("反序列化 casbin_model 失败: {}", e);
                    Json(ApiResponse::err(e.to_string()))
                }
            }
        }
        Err(e) => {
            log::error!("查询 casbin_model 失败: {}", e);
            Json(ApiResponse::err(e.to_string()))
        }
    }
}

/// PUT /auth/model — 修改 Casbin 模型定义
///
/// 更新指定名称的模型定义，更新后自动重载 Enforcer
#[endpoint(tags("model"), summary = "修改 Casbin 模型定义", description = "更新指定名称的模型定义，更新后自动重载 Enforcer")]
pub async fn update_model(body: JsonBody<ModelDto>, depot: &mut Depot) -> Json<ApiResponse<String>> {
    let dto = body.into_inner();

    // 更新模型定义
    let sql = "UPDATE casbin_model SET model_text = ?, description = ? WHERE model_name = ?";
    let result = CONTEXT
        .rbatis
        .exec(
            sql,
            vec![
                value!(&dto.model_text),
                value!(&dto.description),
                value!(&dto.model_name),
            ],
        )
        .await;

    match result {
        Ok(exec_result) => {
            if exec_result.rows_affected == 0 {
                return Json(ApiResponse::err("未找到指定名称的模型"));
            }

            // 重载 Enforcer
            if let Ok(mgr) = depot.obtain::<Arc<EnforcerManager>>() {
                if let Err(e) = mgr.reload().await {
                    log::warn!("Enforcer 重载失败: {}", e);
                    return Json(ApiResponse::err(format!("模型已更新，但 Enforcer 重载失败: {}", e)));
                }
            }

            Json(ApiResponse::ok("模型更新成功".to_string()))
        }
        Err(e) => {
            log::error!("更新 casbin_model 失败: {}", e);
            Json(ApiResponse::err(e.to_string()))
        }
    }
}

// ============================================================================
// Policy 端点
// ============================================================================

/// GET /auth/policies — 列出所有策略规则
///
/// 返回 casbin_rules 表中的所有策略规则，支持可选过滤条件
/// - `object`: 按 v1 字段前缀匹配过滤
/// - `subject`: 按 v0 字段精确匹配过滤
#[endpoint(tags("policies"), summary = "列出所有策略规则", description = "返回 casbin_rules 表中的所有策略规则，支持按 object(v1) 前缀和 subject(v0) 精确过滤")]
pub async fn list_policies(
    object: QueryParam<String, false>,
    subject: QueryParam<String, false>,
) -> Json<ApiResponse<Vec<PolicyRecord>>> {
    let mut sql = "SELECT * FROM casbin_rules WHERE 1=1".to_string();
    let mut params: Vec<rbs::Value> = vec![];

    if let Some(obj) = object.into_inner() {
        if !obj.is_empty() {
            sql.push_str(" AND v1 LIKE ?");
            params.push(value!(format!("{}%", obj)));
        }
    }
    if let Some(sub) = subject.into_inner() {
        if !sub.is_empty() {
            sql.push_str(" AND v0 = ?");
            params.push(value!(sub));
        }
    }
    sql.push_str(" ORDER BY ptype, v0");

    match CONTEXT.rbatis.query(&sql, params).await {
        Ok(value) => {
            match rbs::from_value::<Vec<PolicyRecordDb>>(value) {
                Ok(rows) => {
                    let records: Vec<PolicyRecord> = rows.into_iter().map(|r| r.into()).collect();
                    Json(ApiResponse::ok(records))
                }
                Err(e) => {
                    log::error!("反序列化 casbin_rules 失败: {}", e);
                    Json(ApiResponse::err(e.to_string()))
                }
            }
        }
        Err(e) => {
            log::error!("查询 casbin_rules 失败: {}", e);
            Json(ApiResponse::err(e.to_string()))
        }
    }
}

/// POST /auth/policies — 添加策略规则
///
/// 添加新的策略规则（p/g/g2），添加后自动重载 Enforcer
#[endpoint(tags("policies"), summary = "添加策略规则", description = "添加新的策略规则（p/g/g2），添加后自动重载 Enforcer")]
pub async fn add_policy(body: JsonBody<PolicyDto>, depot: &mut Depot) -> Json<ApiResponse<String>> {
    let dto = body.into_inner();

    // 插入策略规则
    let sql = "INSERT INTO casbin_rules (ptype, v0, v1, v2, v3, v4, v5) VALUES (?, ?, ?, ?, ?, ?, ?)";
    let result = CONTEXT
        .rbatis
        .exec(
            sql,
            vec![
                value!(&dto.ptype),
                value!(&dto.v0),
                value!(&dto.v1),
                value!(&dto.v2),
                value!(&dto.v3),
                value!(&dto.v4),
                value!(&dto.v5),
            ],
        )
        .await;

    match result {
        Ok(_) => {
            // 重载 Enforcer
            if let Ok(mgr) = depot.obtain::<Arc<EnforcerManager>>() {
                if let Err(e) = mgr.reload().await {
                    log::warn!("Enforcer 重载失败: {}", e);
                }
            }
            Json(ApiResponse::ok("策略添加成功".to_string()))
        }
        Err(e) => {
            log::error!("插入 casbin_rules 失败: {}", e);
            Json(ApiResponse::err(e.to_string()))
        }
    }
}

/// DELETE /auth/policies/{id} — 删除策略规则
///
/// 根据 ID 删除策略规则，删除后自动重载 Enforcer
#[endpoint(tags("policies"), summary = "删除策略规则", description = "根据 ID 删除策略规则，删除后自动重载 Enforcer")]
pub async fn remove_policy(id: PathParam<i64>, depot: &mut Depot) -> Json<ApiResponse<String>> {
    let id = id.into_inner();

    // 删除策略规则
    let sql = "DELETE FROM casbin_rules WHERE id = ?";
    let result = CONTEXT.rbatis.exec(sql, vec![value!(id)]).await;

    match result {
        Ok(exec_result) => {
            if exec_result.rows_affected == 0 {
                return Json(ApiResponse::err("未找到指定 ID 的策略规则"));
            }

            // 重载 Enforcer
            if let Ok(mgr) = depot.obtain::<Arc<EnforcerManager>>() {
                if let Err(e) = mgr.reload().await {
                    log::warn!("Enforcer 重载失败: {}", e);
                }
            }
            Json(ApiResponse::ok("策略删除成功".to_string()))
        }
        Err(e) => {
            log::error!("删除 casbin_rules 失败: {}", e);
            Json(ApiResponse::err(e.to_string()))
        }
    }
}

// ============================================================================
// Role 端点（ptype = 'g'）
// ============================================================================

/// GET /auth/roles — 列出角色分配
///
/// 返回所有 ptype='g' 的角色分配规则
#[endpoint(tags("roles"), summary = "列出角色分配", description = "返回所有 ptype=g 的角色分配规则")]
pub async fn list_roles() -> Json<ApiResponse<Vec<PolicyRecord>>> {
    let sql = "SELECT * FROM casbin_rules WHERE ptype = 'g' ORDER BY v0";
    match CONTEXT.rbatis.query(sql, vec![]).await {
        Ok(value) => {
            match rbs::from_value::<Vec<PolicyRecordDb>>(value) {
                Ok(rows) => {
                    let records: Vec<PolicyRecord> = rows.into_iter().map(|r| r.into()).collect();
                    Json(ApiResponse::ok(records))
                }
                Err(e) => {
                    log::error!("反序列化角色分配失败: {}", e);
                    Json(ApiResponse::err(e.to_string()))
                }
            }
        }
        Err(e) => {
            log::error!("查询角色分配失败: {}", e);
            Json(ApiResponse::err(e.to_string()))
        }
    }
}

/// POST /auth/roles — 添加角色分配
///
/// 添加用户到角色的映射（ptype='g'），添加后自动重载 Enforcer
#[endpoint(tags("roles"), summary = "添加角色分配", description = "添加用户到角色的映射（ptype=g），添加后自动重载 Enforcer")]
pub async fn add_role(body: JsonBody<PolicyDto>, depot: &mut Depot) -> Json<ApiResponse<String>> {
    let dto = body.into_inner();

    // 强制使用 ptype='g'
    let sql = "INSERT INTO casbin_rules (ptype, v0, v1, v2, v3, v4, v5) VALUES ('g', ?, ?, ?, ?, ?, ?)";
    let result = CONTEXT
        .rbatis
        .exec(
            sql,
            vec![
                value!(&dto.v0),
                value!(&dto.v1),
                value!(&dto.v2),
                value!(&dto.v3),
                value!(&dto.v4),
                value!(&dto.v5),
            ],
        )
        .await;

    match result {
        Ok(_) => {
            // 重载 Enforcer
            if let Ok(mgr) = depot.obtain::<Arc<EnforcerManager>>() {
                if let Err(e) = mgr.reload().await {
                    log::warn!("Enforcer 重载失败: {}", e);
                }
            }
            Json(ApiResponse::ok("角色分配添加成功".to_string()))
        }
        Err(e) => {
            log::error!("插入角色分配失败: {}", e);
            Json(ApiResponse::err(e.to_string()))
        }
    }
}

/// DELETE /auth/roles/{id} — 移除角色分配
///
/// 根据 ID 删除角色分配规则，删除后自动重载 Enforcer
#[endpoint(tags("roles"), summary = "移除角色分配", description = "根据 ID 删除角色分配规则，删除后自动重载 Enforcer")]
pub async fn remove_role(id: PathParam<i64>, depot: &mut Depot) -> Json<ApiResponse<String>> {
    let id = id.into_inner();

    // 只删除 ptype='g' 的记录
    let sql = "DELETE FROM casbin_rules WHERE id = ? AND ptype = 'g'";
    let result = CONTEXT.rbatis.exec(sql, vec![value!(id)]).await;

    match result {
        Ok(exec_result) => {
            if exec_result.rows_affected == 0 {
                return Json(ApiResponse::err("未找到指定 ID 的角色分配"));
            }

            // 重载 Enforcer
            if let Ok(mgr) = depot.obtain::<Arc<EnforcerManager>>() {
                if let Err(e) = mgr.reload().await {
                    log::warn!("Enforcer 重载失败: {}", e);
                }
            }
            Json(ApiResponse::ok("角色分配删除成功".to_string()))
        }
        Err(e) => {
            log::error!("删除角色分配失败: {}", e);
            Json(ApiResponse::err(e.to_string()))
        }
    }
}

// ============================================================================
// Group 端点（ptype = 'g2'）
// ============================================================================

/// GET /auth/groups — 列出对象分组
///
/// 返回所有 ptype='g2' 的对象分组规则
#[endpoint(tags("groups"), summary = "列出对象分组", description = "返回所有 ptype=g2 的对象分组规则")]
pub async fn list_groups() -> Json<ApiResponse<Vec<PolicyRecord>>> {
    let sql = "SELECT * FROM casbin_rules WHERE ptype = 'g2' ORDER BY v0";
    match CONTEXT.rbatis.query(sql, vec![]).await {
        Ok(value) => {
            match rbs::from_value::<Vec<PolicyRecordDb>>(value) {
                Ok(rows) => {
                    let records: Vec<PolicyRecord> = rows.into_iter().map(|r| r.into()).collect();
                    Json(ApiResponse::ok(records))
                }
                Err(e) => {
                    log::error!("反序列化对象分组失败: {}", e);
                    Json(ApiResponse::err(e.to_string()))
                }
            }
        }
        Err(e) => {
            log::error!("查询对象分组失败: {}", e);
            Json(ApiResponse::err(e.to_string()))
        }
    }
}

/// POST /auth/groups — 添加对象分组
///
/// 添加资源到分组的映射（ptype='g2'），添加后自动重载 Enforcer
#[endpoint(tags("groups"), summary = "添加对象分组", description = "添加资源到分组的映射（ptype=g2），添加后自动重载 Enforcer")]
pub async fn add_group(body: JsonBody<PolicyDto>, depot: &mut Depot) -> Json<ApiResponse<String>> {
    let dto = body.into_inner();

    // 强制使用 ptype='g2'
    let sql = "INSERT INTO casbin_rules (ptype, v0, v1, v2, v3, v4, v5) VALUES ('g2', ?, ?, ?, ?, ?, ?)";
    let result = CONTEXT
        .rbatis
        .exec(
            sql,
            vec![
                value!(&dto.v0),
                value!(&dto.v1),
                value!(&dto.v2),
                value!(&dto.v3),
                value!(&dto.v4),
                value!(&dto.v5),
            ],
        )
        .await;

    match result {
        Ok(_) => {
            // 重载 Enforcer
            if let Ok(mgr) = depot.obtain::<Arc<EnforcerManager>>() {
                if let Err(e) = mgr.reload().await {
                    log::warn!("Enforcer 重载失败: {}", e);
                }
            }
            Json(ApiResponse::ok("对象分组添加成功".to_string()))
        }
        Err(e) => {
            log::error!("插入对象分组失败: {}", e);
            Json(ApiResponse::err(e.to_string()))
        }
    }
}

/// DELETE /auth/groups/{id} — 移除对象分组
///
/// 根据 ID 删除对象分组规则，删除后自动重载 Enforcer
#[endpoint(tags("groups"), summary = "移除对象分组", description = "根据 ID 删除对象分组规则，删除后自动重载 Enforcer")]
pub async fn remove_group(id: PathParam<i64>, depot: &mut Depot) -> Json<ApiResponse<String>> {
    let id = id.into_inner();

    // 只删除 ptype='g2' 的记录
    let sql = "DELETE FROM casbin_rules WHERE id = ? AND ptype = 'g2'";
    let result = CONTEXT.rbatis.exec(sql, vec![value!(id)]).await;

    match result {
        Ok(exec_result) => {
            if exec_result.rows_affected == 0 {
                return Json(ApiResponse::err("未找到指定 ID 的对象分组"));
            }

            // 重载 Enforcer
            if let Ok(mgr) = depot.obtain::<Arc<EnforcerManager>>() {
                if let Err(e) = mgr.reload().await {
                    log::warn!("Enforcer 重载失败: {}", e);
                }
            }
            Json(ApiResponse::ok("对象分组删除成功".to_string()))
        }
        Err(e) => {
            log::error!("删除对象分组失败: {}", e);
            Json(ApiResponse::err(e.to_string()))
        }
    }
}

// ============================================================================
// Reload 端点
// ============================================================================

/// POST /auth/reload — 手动触发 Enforcer 重载
///
/// 从数据库重新加载 Casbin 模型和策略规则
#[endpoint(tags("system"), summary = "手动触发 Enforcer 重载", description = "从数据库重新加载 Casbin 模型和策略规则")]
pub async fn reload_enforcer(depot: &mut Depot) -> Json<ApiResponse<String>> {
    match depot.obtain::<Arc<EnforcerManager>>() {
        Ok(mgr) => match mgr.reload().await {
            Ok(_) => Json(ApiResponse::ok("Enforcer 重载成功".to_string())),
            Err(e) => {
                log::error!("Enforcer 重载失败: {}", e);
                Json(ApiResponse::err(format!("重载失败: {}", e)))
            }
        },
        Err(_) => Json(ApiResponse::err("无法获取 EnforcerManager")),
    }
}

// ============================================================================
// 路由构建
// ============================================================================

/// 构建 Auth Admin 路由
///
/// 返回包含所有权限管理端点的路由器
///
/// # 端点
/// - `/auth/schemas` - GET: 列出所有 API Schema
/// - `/auth/model` - GET: 获取模型, PUT: 更新模型
/// - `/auth/policies` - GET: 列出策略, POST: 添加策略
/// - `/auth/policies/{id}` - DELETE: 删除策略
/// - `/auth/roles` - GET: 列出角色, POST: 添加角色
/// - `/auth/roles/{id}` - DELETE: 删除角色
/// - `/auth/groups` - GET: 列出分组, POST: 添加分组
/// - `/auth/groups/{id}` - DELETE: 删除分组
/// - `/auth/reload` - POST: 手动重载 Enforcer
pub fn auth_admin_router() -> Router {
    Router::with_path("/auth")
        .push(Router::with_path("/schemas").get(list_schemas))
        .push(Router::with_path("/model").get(get_model).put(update_model))
        .push(Router::with_path("/policies").get(list_policies).post(add_policy))
        .push(Router::with_path("/policies/{id}").delete(remove_policy))
        .push(Router::with_path("/roles").get(list_roles).post(add_role))
        .push(Router::with_path("/roles/{id}").delete(remove_role))
        .push(Router::with_path("/groups").get(list_groups).post(add_group))
        .push(Router::with_path("/groups/{id}").delete(remove_group))
        .push(Router::with_path("/reload").post(reload_enforcer))
}

// ============================================================================
// Token 端点（公开，无需认证）
// ============================================================================

/// GET /auth/token — 获取临时访问令牌
///
/// 使用 client_credentials 方式从 Keycloak 获取临时 JWT Token
/// 此端点不需要认证即可访问
#[endpoint(tags("auth"), summary = "获取临时访问令牌", description = "使用 client_credentials 方式从 Keycloak 获取临时 JWT Token")]
pub async fn get_access_token() -> Json<ApiResponse<TokenResponse>> {
    use genies_core::jwt::get_temp_access_token;

    let config = &CONTEXT.config;

    match get_temp_access_token(
        &config.keycloak_auth_server_url,
        &config.keycloak_realm,
        &config.keycloak_resource,
        &config.keycloak_credentials_secret,
    )
    .await {
        Ok(token) => {
            let token_resp = TokenResponse {
                access_token: token,
                expires_in: 900,
                token_type: "Bearer".to_string(),
            };
            Json(ApiResponse::ok(token_resp))
        }
        Err(e) => {
            log::error!("获取 Token 失败: {}", e);
            Json(ApiResponse::err("获取 Token 失败"))
        }
    }
}

/// 不需要认证的 auth 公共路由
pub fn auth_public_router() -> Router {
    Router::with_path("auth")
        .push(Router::with_path("token").get(get_access_token))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_api_response_ok() {
        let resp: ApiResponse<String> = ApiResponse::ok("test".to_string());
        assert_eq!(resp.code, "0");
        assert_eq!(resp.msg, "ok");
        assert_eq!(resp.data, Some("test".to_string()));
    }

    #[test]
    fn test_api_response_err() {
        let resp: ApiResponse<String> = ApiResponse::err("error message");
        assert_eq!(resp.code, "-1");
        assert_eq!(resp.msg, "error message");
        assert!(resp.data.is_none());
    }

    #[test]
    fn test_policy_record_db_conversion() {
        let db = PolicyRecordDb {
            id: 1,
            ptype: "p".to_string(),
            v0: "admin".to_string(),
            v1: "/api/*".to_string(),
            v2: "GET".to_string(),
            v3: "".to_string(),
            v4: "".to_string(),
            v5: "".to_string(),
        };

        let record: PolicyRecord = db.into();
        assert_eq!(record.id, 1);
        assert_eq!(record.ptype, "p");
        assert_eq!(record.v0, "admin");
        assert_eq!(record.v1, "/api/*");
        assert_eq!(record.v2, "GET");
    }

    #[test]
    fn test_schema_record_db_conversion() {
        let db = SchemaRecordDb {
            id: 1,
            schema_name: "User".to_string(),
            schema_label: Some("用户".to_string()),
            schema_description: Some("用户信息对象".to_string()),
            field_name: "email".to_string(),
            field_label: Some("邮箱".to_string()),
            field_type: Some("string".to_string()),
            field_description: Some("用户邮箱地址".to_string()),
            field_required: Some(true),
            endpoint_path: Some("/api/users".to_string()),
            endpoint_label: Some("获取用户".to_string()),
            endpoint_description: Some("获取用户信息".to_string()),
            endpoint_tags: Some("[\"users\"]".to_string()),
            endpoint_operation_id: Some("get_user".to_string()),
            http_method: Some("GET".to_string()),
        };

        let record: SchemaRecord = db.into();
        assert_eq!(record.id, 1);
        assert_eq!(record.schema_name, "User");
        assert_eq!(record.field_name, "email");
        assert_eq!(record.endpoint_path, Some("/api/users".to_string()));
    }

    #[test]
    fn test_model_record_db_conversion() {
        let db = ModelRecordDb {
            id: 1,
            model_name: "default".to_string(),
            model_text: "[request_definition]\nr = sub, obj, act".to_string(),
            description: Some("默认模型".to_string()),
        };

        let record: ModelRecord = db.into();
        assert_eq!(record.id, 1);
        assert_eq!(record.model_name, "default");
        assert!(record.model_text.contains("request_definition"));
        assert_eq!(record.description, Some("默认模型".to_string()));
    }
}
