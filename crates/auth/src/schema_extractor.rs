//! OpenApi Schema 提取与同步模块
//!
//! 从 Salvo OpenApi 文档中自动提取 Schema 和字段信息，
//! 同步到 `auth_api_schemas` 数据库表。
//!
//! # 核心功能
//! - 解析 OpenApi 文档中的 `components.schemas`
//! - 解析 `paths` 中的 API 端点信息
//! - 建立 Schema 与 Endpoint 的关联关系
//! - 同步到数据库（INSERT ... ON DUPLICATE KEY UPDATE）

use genies::context::CONTEXT;
use genies_core::error::Error;
use genies_core::Result;
use salvo::oapi::OpenApi;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;

// ============================================================================
// 数据结构定义
// ============================================================================

/// Schema 字段记录
///
/// 表示 API Schema 中的单个字段，包含其元信息和关联的 Endpoint 信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SchemaFieldRecord {
    /// Schema 完整名称，如 "genies_auth.vo.User"
    pub schema_name: String,
    /// Schema 中文标签（可选，由管理员后续设置）
    pub schema_label: Option<String>,
    /// Schema 对象描述，来自 OpenAPI Schema.description
    pub schema_description: Option<String>,
    /// 字段名称，如 "email", "phone"
    pub field_name: String,
    /// 字段中文标签（可选，由管理员后续设置）
    pub field_label: Option<String>,
    /// 字段类型，如 "string", "integer", "boolean"
    pub field_type: Option<String>,
    /// 字段描述，来自 OpenAPI property.description
    pub field_description: Option<String>,
    /// 字段是否必需，来自 OpenAPI Schema.required 数组
    pub field_required: Option<bool>,
    /// 关联的 API 端点路径，如 "/api/users"
    pub endpoint_path: Option<String>,
    /// 端点中文标签（可选，由管理员后续设置）
    pub endpoint_label: Option<String>,
    /// API 操作详细描述，来自 OpenAPI Operation.description
    pub endpoint_description: Option<String>,
    /// API 标签，JSON 数组格式，来自 OpenAPI Operation.tags
    pub endpoint_tags: Option<String>,
    /// 操作 ID，来自 OpenAPI Operation.operationId
    pub endpoint_operation_id: Option<String>,
    /// HTTP 方法，如 "GET", "POST", "PUT", "DELETE"
    pub http_method: Option<String>,
}

/// Schema 信息（内部使用）
#[derive(Debug, Clone)]
struct SchemaInfo {
    /// Schema 名称
    name: String,
    /// Schema 对象描述
    description: Option<String>,
    /// 必需字段列表
    required_fields: Vec<String>,
    /// 字段列表：(字段名, 字段类型, 字段描述)
    fields: Vec<(String, Option<String>, Option<String>)>,
}

/// Endpoint 信息（内部使用）
#[derive(Debug, Clone)]
struct EndpointInfo {
    /// API 路径
    path: String,
    /// HTTP 方法
    method: String,
    /// 端点描述（summary）
    summary: Option<String>,
    /// 操作详细描述
    description: Option<String>,
    /// 标签列表
    tags: Vec<String>,
    /// 操作 ID
    operation_id: Option<String>,
    /// 关联的 Schema 名称列表
    schema_refs: Vec<String>,
}

// ============================================================================
// 主入口函数
// ============================================================================

/// 从 OpenApi 文档提取 Schema 和字段信息，同步到数据库
///
/// # 处理流程
/// 1. 将 OpenApi 对象序列化为 JSON
/// 2. 提取 `components.schemas` 中的所有 Schema 定义
/// 3. 提取 `paths` 中的所有 API 端点及其关联的 Schema
/// 4. 合并 Schema 与 Endpoint 信息
/// 5. 写入数据库（使用 UPSERT 语义）
///
/// # Arguments
/// * `doc` - Salvo OpenApi 文档对象
///
/// # Returns
/// * `Ok(())` - 成功同步
/// * `Err(e)` - 同步失败，返回错误信息
///
/// # Example
/// ```rust,ignore
/// use salvo::oapi::OpenApi;
/// use genies_auth::schema_extractor::extract_and_sync_schemas;
///
/// let router = Router::new().push(Router::with_path("/api/users").get(get_user));
/// let doc = OpenApi::new("service", "1.0").merge_router(&router);
/// extract_and_sync_schemas(&doc).await?;
/// ```
pub async fn extract_and_sync_schemas(doc: &OpenApi) -> Result<()> {
    // 1. 将 OpenApi 对象序列化为 JSON
    let json_value = serde_json::to_value(doc)
        .map_err(|e| Error::from(format!("序列化 OpenApi 文档失败: {}", e)))?;
    
    log::debug!("开始解析 OpenApi 文档...");
    
    // 2. 提取 schemas — 遍历 components.schemas
    let schemas = extract_schemas(&json_value);
    log::debug!("提取到 {} 个 Schema 定义", schemas.len());
    
    // 3. 提取 paths — 遍历 paths，关联 response schema
    let endpoints = extract_endpoints(&json_value);
    log::debug!("提取到 {} 个 API 端点", endpoints.len());
    
    // 4. 合并 schema + endpoint 信息
    let records = merge_schema_endpoints(&schemas, &endpoints);
    log::debug!("生成 {} 条 Schema 字段记录", records.len());
    
    if records.is_empty() {
        log::info!("未发现需要同步的 Schema 字段");
        return Ok(());
    }
    
    // 5. 写入数据库 — INSERT ... ON DUPLICATE KEY UPDATE
    for record in &records {
        sync_to_db(record).await
            .map_err(|e| Error::from(format!("同步记录到数据库失败: {}", e)))?;
    }

    log::info!("已同步 {} 条 Schema 字段记录到数据库", records.len());
    Ok(())
}

// ============================================================================
// Schema 提取
// ============================================================================

/// 从 JSON 中提取所有 Schema 定义
///
/// 遍历 `components.schemas` 节点，解析每个 Schema 的字段信息
fn extract_schemas(json: &Value) -> Vec<SchemaInfo> {
    let mut schemas = Vec::new();
    
    // 获取 components.schemas 节点
    let schemas_node = match json.get("components")
        .and_then(|c| c.get("schemas"))
        .and_then(|s| s.as_object())
    {
        Some(node) => node,
        None => {
            log::debug!("未找到 components.schemas 节点");
            return schemas;
        }
    };
    
    // 遍历每个 Schema
    for (schema_name, schema_def) in schemas_node {
        // 提取 Schema 级描述
        let description = schema_def.get("description")
            .and_then(|d| d.as_str())
            .map(|s| s.to_string());
        
        // 提取 required 字段列表
        let required_fields = schema_def.get("required")
            .and_then(|r| r.as_array())
            .map(|arr| arr.iter().filter_map(|v| v.as_str().map(String::from)).collect())
            .unwrap_or_default();
        
        let fields = extract_schema_fields(schema_def);
        
        if !fields.is_empty() {
            schemas.push(SchemaInfo {
                name: schema_name.clone(),
                description,
                required_fields,
                fields,
            });
        }
    }
    
    schemas
}

/// 提取单个 Schema 的字段列表
///
/// 解析 Schema 定义中的 `properties` 节点
fn extract_schema_fields(schema_def: &Value) -> Vec<(String, Option<String>, Option<String>)> {
    let mut fields = Vec::new();
    
    // 获取 properties 节点
    let properties = match schema_def.get("properties").and_then(|p| p.as_object()) {
        Some(props) => props,
        None => return fields,
    };
    
    // 遍历每个字段
    for (field_name, field_def) in properties {
        let field_type = extract_field_type(field_def);
        let field_description = field_def.get("description")
            .and_then(|d| d.as_str())
            .map(|s| s.to_string());
        fields.push((field_name.clone(), field_type, field_description));
    }
    
    fields
}

/// 提取字段类型
///
/// 支持基本类型、数组类型和引用类型
fn extract_field_type(field_def: &Value) -> Option<String> {
    // 优先检查直接的 type 字段
    if let Some(type_val) = field_def.get("type").and_then(|t| t.as_str()) {
        // 如果是数组类型，尝试获取元素类型
        if type_val == "array" {
            if let Some(items) = field_def.get("items") {
                if let Some(item_type) = extract_field_type(items) {
                    return Some(format!("array<{}>", item_type));
                }
            }
            return Some("array".to_string());
        }
        
        // 检查是否有 format 补充信息
        if let Some(format) = field_def.get("format").and_then(|f| f.as_str()) {
            return Some(format!("{}({})", type_val, format));
        }
        
        return Some(type_val.to_string());
    }
    
    // 检查 $ref 引用
    if let Some(ref_val) = field_def.get("$ref").and_then(|r| r.as_str()) {
        return Some(extract_ref_name(ref_val));
    }
    
    // 检查 allOf/oneOf/anyOf 组合类型
    for combiner in ["allOf", "oneOf", "anyOf"] {
        if let Some(items) = field_def.get(combiner).and_then(|a| a.as_array()) {
            if let Some(first_item) = items.first() {
                if let Some(ref_val) = first_item.get("$ref").and_then(|r| r.as_str()) {
                    return Some(extract_ref_name(ref_val));
                }
            }
        }
    }
    
    None
}

/// 从 $ref 路径中提取 Schema 名称
///
/// 例如：`#/components/schemas/genies_auth.vo.User` -> `genies_auth.vo.User`
fn extract_ref_name(ref_path: &str) -> String {
    ref_path
        .rsplit('/')
        .next()
        .unwrap_or(ref_path)
        .to_string()
}

// ============================================================================
// Endpoint 提取
// ============================================================================

/// 从 JSON 中提取所有 API 端点信息
///
/// 遍历 `paths` 节点，解析每个端点的 HTTP 方法和关联的 Schema
fn extract_endpoints(json: &Value) -> Vec<EndpointInfo> {
    let mut endpoints = Vec::new();
    
    // 获取 paths 节点
    let paths_node = match json.get("paths").and_then(|p| p.as_object()) {
        Some(node) => node,
        None => {
            log::debug!("未找到 paths 节点");
            return endpoints;
        }
    };
    
    // 支持的 HTTP 方法
    let http_methods = ["get", "post", "put", "delete", "patch", "head", "options"];
    
    // 遍历每个路径
    for (path, path_def) in paths_node {
        let path_obj = match path_def.as_object() {
            Some(obj) => obj,
            None => continue,
        };
        
        // 遍历每个 HTTP 方法
        for method in &http_methods {
            if let Some(operation) = path_obj.get(*method) {
                let endpoint = parse_endpoint(path, method, operation);
                endpoints.push(endpoint);
            }
        }
    }
    
    endpoints
}

/// 解析单个 Endpoint 操作
///
/// 提取 summary 和关联的 Schema 引用
fn parse_endpoint(path: &str, method: &str, operation: &Value) -> EndpointInfo {
    // 提取 summary
    let summary = operation
        .get("summary")
        .and_then(|s| s.as_str())
        .map(|s| s.to_string());
    
    // 提取 description
    let description = operation.get("description")
        .and_then(|s| s.as_str())
        .map(|s| s.to_string());
    
    // 提取 tags
    let tags = operation.get("tags")
        .and_then(|t| t.as_array())
        .map(|arr| arr.iter().filter_map(|v| v.as_str().map(String::from)).collect())
        .unwrap_or_default();
    
    // 提取 operationId
    let operation_id = operation.get("operationId")
        .and_then(|s| s.as_str())
        .map(|s| s.to_string());
    
    // 提取关联的 Schema 引用
    let schema_refs = extract_operation_schema_refs(operation);
    
    EndpointInfo {
        path: path.to_string(),
        method: method.to_uppercase(),
        summary,
        description,
        tags,
        operation_id,
        schema_refs,
    }
}

/// 提取 Operation 中关联的所有 Schema 引用
///
/// 搜索 responses 和 requestBody 中的 $ref
fn extract_operation_schema_refs(operation: &Value) -> Vec<String> {
    let mut refs = Vec::new();
    
    // 1. 从 responses 中提取（主要关注成功响应 200/201/2xx）
    if let Some(responses) = operation.get("responses").and_then(|r| r.as_object()) {
        for (status_code, response) in responses {
            // 只关注成功状态码
            if status_code.starts_with('2') || status_code == "default" {
                if let Some(schema_ref) = extract_content_schema_ref(response) {
                    if !refs.contains(&schema_ref) {
                        refs.push(schema_ref);
                    }
                }
            }
        }
    }
    
    // 2. 从 requestBody 中提取
    if let Some(request_body) = operation.get("requestBody") {
        if let Some(schema_ref) = extract_content_schema_ref(request_body) {
            if !refs.contains(&schema_ref) {
                refs.push(schema_ref);
            }
        }
    }
    
    refs
}

/// 从 content 节点中提取 Schema 引用
///
/// 搜索路径：content.application/json.schema.$ref
fn extract_content_schema_ref(node: &Value) -> Option<String> {
    // 尝试多种 content-type
    let content_types = [
        "application/json",
        "application/x-www-form-urlencoded",
        "multipart/form-data",
        "*/*",
    ];
    
    let content = node.get("content").and_then(|c| c.as_object())?;
    
    for content_type in &content_types {
        if let Some(media_type) = content.get(*content_type) {
            if let Some(schema) = media_type.get("schema") {
                // 直接 $ref
                if let Some(ref_val) = schema.get("$ref").and_then(|r| r.as_str()) {
                    return Some(extract_ref_name(ref_val));
                }
                
                // 数组类型中的 $ref
                if let Some(items) = schema.get("items") {
                    if let Some(ref_val) = items.get("$ref").and_then(|r| r.as_str()) {
                        return Some(extract_ref_name(ref_val));
                    }
                }
                
                // allOf/oneOf/anyOf 中的 $ref
                for combiner in ["allOf", "oneOf", "anyOf"] {
                    if let Some(items) = schema.get(combiner).and_then(|a| a.as_array()) {
                        for item in items {
                            if let Some(ref_val) = item.get("$ref").and_then(|r| r.as_str()) {
                                return Some(extract_ref_name(ref_val));
                            }
                        }
                    }
                }
            }
        }
    }
    
    None
}

// ============================================================================
// 合并 Schema 与 Endpoint
// ============================================================================

/// 合并 Schema 和 Endpoint 信息，生成最终记录
///
/// 每个 Schema 的每个字段生成一条记录，
/// 通过 $ref 将 Endpoint 信息关联到对应的 Schema
fn merge_schema_endpoints(
    schemas: &[SchemaInfo],
    endpoints: &[EndpointInfo],
) -> Vec<SchemaFieldRecord> {
    let mut records = Vec::new();
    
    // 构建 Schema -> Endpoints 映射表
    let schema_endpoint_map = build_schema_endpoint_map(endpoints);
    
    // 遍历每个 Schema
    for schema in schemas {
        // 查找关联的 Endpoint 信息
        let endpoint_info = schema_endpoint_map.get(&schema.name);
        
        // 为每个字段生成记录
        for (field_name, field_type, field_description) in &schema.fields {
            let field_required = Some(schema.required_fields.contains(field_name));
            
            let record = SchemaFieldRecord {
                schema_name: schema.name.clone(),
                schema_label: None, // 由管理员后续设置
                schema_description: schema.description.clone(),
                field_name: field_name.clone(),
                field_label: None,  // 由管理员后续设置
                field_type: field_type.clone(),
                field_description: field_description.clone(),
                field_required,
                endpoint_path: endpoint_info.map(|ep| ep.path.clone()),
                endpoint_label: endpoint_info.and_then(|ep| ep.summary.clone()),
                endpoint_description: endpoint_info.and_then(|ep| ep.description.clone()),
                endpoint_tags: endpoint_info.map(|ep| {
                    serde_json::to_string(&ep.tags).unwrap_or_default()
                }).filter(|s| s != "[]"),
                endpoint_operation_id: endpoint_info.and_then(|ep| ep.operation_id.clone()),
                http_method: endpoint_info.map(|ep| ep.method.clone()),
            };
            records.push(record);
        }
    }
    
    records
}

/// 构建 Schema 名称到 Endpoint 信息的映射表
///
/// 返回：Schema名称 -> &EndpointInfo
fn build_schema_endpoint_map<'a>(
    endpoints: &'a [EndpointInfo],
) -> HashMap<String, &'a EndpointInfo> {
    let mut map = HashMap::new();
    
    for endpoint in endpoints {
        for schema_ref in &endpoint.schema_refs {
            // 如果一个 Schema 关联多个 Endpoint，保留第一个
            // （后续可以扩展为多对多关系）
            if !map.contains_key(schema_ref) {
                map.insert(schema_ref.clone(), endpoint);
            }
        }
    }
    
    map
}

// ============================================================================
// 数据库同步
// ============================================================================

/// 将记录同步到数据库
///
/// 使用 INSERT ... ON DUPLICATE KEY UPDATE 实现 UPSERT 语义
/// - 新记录：插入所有字段
/// - 已存在：更新 field_type, endpoint_path, http_method, updated_at
/// - 保留 schema_label, field_label, endpoint_label 的手动设置值
async fn sync_to_db(record: &SchemaFieldRecord) -> Result<()> {
    let sql = r#"
        INSERT INTO auth_api_schemas 
            (schema_name, schema_label, schema_description, field_name, field_label, 
             field_type, field_description, field_required,
             endpoint_path, endpoint_label, endpoint_description, 
             endpoint_tags, endpoint_operation_id, http_method)
        VALUES 
            (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
        ON DUPLICATE KEY UPDATE 
            schema_description = COALESCE(VALUES(schema_description), auth_api_schemas.schema_description),
            field_type = VALUES(field_type),
            field_description = COALESCE(VALUES(field_description), auth_api_schemas.field_description),
            field_required = VALUES(field_required),
            endpoint_path = VALUES(endpoint_path),
            endpoint_label = COALESCE(auth_api_schemas.endpoint_label, VALUES(endpoint_label)),
            endpoint_description = COALESCE(VALUES(endpoint_description), auth_api_schemas.endpoint_description),
            endpoint_tags = VALUES(endpoint_tags),
            endpoint_operation_id = VALUES(endpoint_operation_id),
            http_method = VALUES(http_method),
            updated_at = NOW()
    "#;
    
    // 使用 RBatis 执行原生 SQL
    CONTEXT.rbatis.exec(
        sql,
        vec![
            rbs::to_value(&record.schema_name).map_err(|e| Error::from(e.to_string()))?,
            rbs::to_value(&record.schema_label).map_err(|e| Error::from(e.to_string()))?,
            rbs::to_value(&record.schema_description).map_err(|e| Error::from(e.to_string()))?,
            rbs::to_value(&record.field_name).map_err(|e| Error::from(e.to_string()))?,
            rbs::to_value(&record.field_label).map_err(|e| Error::from(e.to_string()))?,
            rbs::to_value(&record.field_type).map_err(|e| Error::from(e.to_string()))?,
            rbs::to_value(&record.field_description).map_err(|e| Error::from(e.to_string()))?,
            rbs::to_value(&record.field_required.unwrap_or(false)).map_err(|e| Error::from(e.to_string()))?,
            rbs::to_value(&record.endpoint_path).map_err(|e| Error::from(e.to_string()))?,
            rbs::to_value(&record.endpoint_label).map_err(|e| Error::from(e.to_string()))?,
            rbs::to_value(&record.endpoint_description).map_err(|e| Error::from(e.to_string()))?,
            rbs::to_value(&record.endpoint_tags).map_err(|e| Error::from(e.to_string()))?,
            rbs::to_value(&record.endpoint_operation_id).map_err(|e| Error::from(e.to_string()))?,
            rbs::to_value(&record.http_method).map_err(|e| Error::from(e.to_string()))?,
        ],
    ).await.map_err(|e| Error::from(e.to_string()))?;
    
    Ok(())
}

// ============================================================================
// 辅助查询函数
// ============================================================================

/// 从数据库查询所有 Schema 记录
///
/// 可用于管理界面展示和权限配置
pub async fn query_all_schemas() -> Result<Vec<SchemaFieldRecord>> {
    let sql = r#"
        SELECT 
            schema_name, schema_label, schema_description,
            field_name, field_label, field_type, field_description, field_required,
            endpoint_path, endpoint_label, endpoint_description, 
            endpoint_tags, endpoint_operation_id, http_method
        FROM auth_api_schemas
        ORDER BY schema_name, field_name
    "#;
    
    let value: rbs::Value = CONTEXT.rbatis.query(sql, vec![]).await
        .map_err(|e| Error::from(e.to_string()))?;
    let result: Vec<SchemaFieldRecord> = rbs::from_value(value)
        .map_err(|e| Error::from(e.to_string()))?;
    Ok(result)
}

/// 根据 Schema 名称查询字段记录
///
/// # Arguments
/// * `schema_name` - Schema 完整名称
pub async fn query_schema_fields(schema_name: &str) -> Result<Vec<SchemaFieldRecord>> {
    let sql = r#"
        SELECT 
            schema_name, schema_label, schema_description,
            field_name, field_label, field_type, field_description, field_required,
            endpoint_path, endpoint_label, endpoint_description, 
            endpoint_tags, endpoint_operation_id, http_method
        FROM auth_api_schemas
        WHERE schema_name = ?
        ORDER BY field_name
    "#;
    
    let value: rbs::Value = CONTEXT.rbatis
        .query(sql, vec![rbs::to_value(schema_name).map_err(|e| Error::from(e.to_string()))?])
        .await
        .map_err(|e| Error::from(e.to_string()))?;
    let result: Vec<SchemaFieldRecord> = rbs::from_value(value)
        .map_err(|e| Error::from(e.to_string()))?;
    Ok(result)
}

/// 根据 Endpoint 路径查询关联的 Schema 记录
///
/// # Arguments
/// * `endpoint_path` - API 端点路径
pub async fn query_endpoint_schemas(endpoint_path: &str) -> Result<Vec<SchemaFieldRecord>> {
    let sql = r#"
        SELECT 
            schema_name, schema_label, schema_description,
            field_name, field_label, field_type, field_description, field_required,
            endpoint_path, endpoint_label, endpoint_description, 
            endpoint_tags, endpoint_operation_id, http_method
        FROM auth_api_schemas
        WHERE endpoint_path = ?
        ORDER BY schema_name, field_name
    "#;
    
    let value: rbs::Value = CONTEXT.rbatis
        .query(sql, vec![rbs::to_value(endpoint_path).map_err(|e| Error::from(e.to_string()))?])
        .await
        .map_err(|e| Error::from(e.to_string()))?;
    let result: Vec<SchemaFieldRecord> = rbs::from_value(value)
        .map_err(|e| Error::from(e.to_string()))?;
    Ok(result)
}

// ============================================================================
// 测试模块
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_extract_ref_name() {
        assert_eq!(
            extract_ref_name("#/components/schemas/genies_auth.vo.User"),
            "genies_auth.vo.User"
        );
        assert_eq!(
            extract_ref_name("#/components/schemas/Simple"),
            "Simple"
        );
    }

    #[test]
    fn test_extract_field_type() {
        // 基本类型
        let field_def = json!({"type": "string"});
        assert_eq!(extract_field_type(&field_def), Some("string".to_string()));

        // 带 format 的类型
        let field_def = json!({"type": "integer", "format": "int64"});
        assert_eq!(extract_field_type(&field_def), Some("integer(int64)".to_string()));

        // 数组类型
        let field_def = json!({"type": "array", "items": {"type": "string"}});
        assert_eq!(extract_field_type(&field_def), Some("array<string>".to_string()));

        // $ref 类型
        let field_def = json!({"$ref": "#/components/schemas/User"});
        assert_eq!(extract_field_type(&field_def), Some("User".to_string()));
    }

    #[test]
    fn test_extract_schemas() {
        let doc = json!({
            "components": {
                "schemas": {
                    "User": {
                        "type": "object",
                        "description": "用户对象",
                        "required": ["id", "email"],
                        "properties": {
                            "id": {"type": "integer", "format": "int64", "description": "用户ID"},
                            "name": {"type": "string", "description": "用户姓名"},
                            "email": {"type": "string", "description": "用户邮箱"}
                        }
                    }
                }
            }
        });

        let schemas = extract_schemas(&doc);
        assert_eq!(schemas.len(), 1);
        assert_eq!(schemas[0].name, "User");
        assert_eq!(schemas[0].description, Some("用户对象".to_string()));
        assert_eq!(schemas[0].required_fields, vec!["id", "email"]);
        assert_eq!(schemas[0].fields.len(), 3);
        // 验证字段描述
        let id_field = schemas[0].fields.iter().find(|(name, _, _)| name == "id").unwrap();
        assert_eq!(id_field.2, Some("用户ID".to_string()));
    }

    #[test]
    fn test_extract_endpoints() {
        let doc = json!({
            "paths": {
                "/api/users": {
                    "get": {
                        "summary": "获取用户列表",
                        "description": "获取所有用户的详细信息",
                        "tags": ["users"],
                        "operationId": "list_users",
                        "responses": {
                            "200": {
                                "content": {
                                    "application/json": {
                                        "schema": {
                                            "$ref": "#/components/schemas/User"
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        });

        let endpoints = extract_endpoints(&doc);
        assert_eq!(endpoints.len(), 1);
        assert_eq!(endpoints[0].path, "/api/users");
        assert_eq!(endpoints[0].method, "GET");
        assert_eq!(endpoints[0].summary, Some("获取用户列表".to_string()));
        assert_eq!(endpoints[0].description, Some("获取所有用户的详细信息".to_string()));
        assert_eq!(endpoints[0].tags, vec!["users".to_string()]);
        assert_eq!(endpoints[0].operation_id, Some("list_users".to_string()));
        assert!(endpoints[0].schema_refs.contains(&"User".to_string()));
    }

    #[test]
    fn test_merge_schema_endpoints() {
        let schemas = vec![SchemaInfo {
            name: "User".to_string(),
            description: Some("用户对象".to_string()),
            required_fields: vec!["id".to_string()],
            fields: vec![
                ("id".to_string(), Some("integer(int64)".to_string()), Some("用户ID".to_string())),
                ("name".to_string(), Some("string".to_string()), None),
            ],
        }];

        let endpoints = vec![EndpointInfo {
            path: "/api/users".to_string(),
            method: "GET".to_string(),
            summary: Some("获取用户".to_string()),
            description: Some("获取用户详情".to_string()),
            tags: vec!["users".to_string()],
            operation_id: Some("get_users".to_string()),
            schema_refs: vec!["User".to_string()],
        }];

        let records = merge_schema_endpoints(&schemas, &endpoints);
        
        assert_eq!(records.len(), 2);
        // 验证第一条记录
        assert_eq!(records[0].schema_name, "User");
        assert_eq!(records[0].schema_description, Some("用户对象".to_string()));
        assert_eq!(records[0].field_name, "id");
        assert_eq!(records[0].field_description, Some("用户ID".to_string()));
        assert_eq!(records[0].field_required, Some(true));
        assert_eq!(records[0].endpoint_path, Some("/api/users".to_string()));
        assert_eq!(records[0].endpoint_description, Some("获取用户详情".to_string()));
        assert_eq!(records[0].endpoint_operation_id, Some("get_users".to_string()));
        assert_eq!(records[0].http_method, Some("GET".to_string()));
        // 验证第二条记录的 required 为 false
        assert_eq!(records[1].field_name, "name");
        assert_eq!(records[1].field_required, Some(false));
        assert_eq!(records[1].field_description, None);
    }
}
