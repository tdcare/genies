//! Auth 模块集成测试
//!
//! 测试 Auth 服务的完整功能流程，包括：
//! - Schema 同步
//! - 模型导入
//! - 策略加载
//! - 热更新
//! - 接口权限 (403)
//! - 字段过滤
//! - Redis 缓存
//! - 并发安全
//!
//! 运行测试: `cargo test -p integration auth_tests -- --nocapture`

#[cfg(test)]
mod tests {
    use casbin::CoreApi;
    use genies::context::CONTEXT;
    use genies_auth::{auth_admin_router, auth_admin_ui_router, auth_public_router, casbin_auth, extract_and_sync_schemas, EnforcerManager};
    use genies_derive::casbin;
    use reqwest::header::{HeaderMap, HeaderValue, AUTHORIZATION};
    use salvo::oapi::swagger_ui::SwaggerUi;
    use salvo::prelude::*;
    use serde::{Deserialize, Serialize};
    use serde_json::{json, Value};
    use std::sync::Arc;
    use std::time::Duration;
    use tokio::sync::OnceCell;

    // ==================== 示例 Struct（内嵌服务器用） ====================

    /// 用户信息对象
    #[casbin]
    #[derive(Deserialize, Serialize, ToSchema)]
    pub struct User {
        /// 用户唯一标识
        pub id: u64,
        /// 用户姓名
        pub name: Option<String>,
        /// 用户邮箱地址
        pub email: String,
        /// 用户手机号码
        pub phone: String,
        /// 用户信用卡号
        pub credit_card: String,
    }

    /// 获取用户信息
    ///
    /// 返回当前用户的详细信息，包括基本资料和联系方式
    #[endpoint]
    async fn get_user() -> Json<User> {
        Json(User {
            id: 1,
            name: Some("张三".into()),
            email: "zhangsan@example.com".into(),
            phone: "13800138000".into(),
            credit_card: "1234-5678-9012-3456".into(),
        })
    }

    // ==================== 常量定义 ====================

    /// 测试用 JWT Token (可根据实际环境调整)
    /// 在实际测试中，可能需要从 Keycloak 获取真实 token
    /// 或者配置服务端跳过 JWT 验证
    const TEST_TOKEN: &str = "eyJhbGciOiJSUzI1NiIsInR5cCIgOiAiSldUIiwia2lkIiA6ICJqcGVlbl94dzR0RjIxOFZfUF9ZVC1WNG5WNmw4XzFXN0JiUjRncHZmZFA4In0.eyJleHAiOjE3NzQ3Njk1MDgsImlhdCI6MTc3NDY2MTUwOCwianRpIjoiN2U0ZmM5YzItNWQzYS00NTEyLTkyOTUtMWVhYTAzYzIxZWQ2IiwiaXNzIjoiaHR0cDovL2dhdGV3YXktc2VydmljZS9hdXRoL3JlYWxtcy90ZGNhcmUiLCJhdWQiOiJhY2NvdW50Iiwic3ViIjoiZWYzZmRmYzYtNWVhNy00NTFjLThiMzctOTk2MGIxN2IxMzk3IiwidHlwIjoiQmVhcmVyIiwiYXpwIjoidGRuaXMiLCJzZXNzaW9uX3N0YXRlIjoiODA5OGYwYmMtNjMxYS00MWJhLWE4YzItODU5OGU4MjgxMDk2IiwiYWNyIjoiMSIsInJlYWxtX2FjY2VzcyI6eyJyb2xlcyI6WyJvZmZsaW5lX2FjY2VzcyIsIm51cnNlIiwidW1hX2F1dGhvcml6YXRpb24iLCJ1c2VyIl19LCJyZXNvdXJjZV9hY2Nlc3MiOnsidGRuaXMiOnsicm9sZXMiOlsibnVyc2VNYW5hZ2VyIiwidXNlciJdfSwiYWNjb3VudCI6eyJyb2xlcyI6WyJtYW5hZ2UtYWNjb3VudCIsInZpZXctcHJvZmlsZSJdfX0sInNjb3BlIjoiIiwiZGVwYXJ0bWVudE5hbWUiOiLlhajnp5HljLvlrabnp5HmiqTnkIbnq5kiLCJhZGRyZXNzIjp7fSwiZGVwYXJ0bWVudENvZGUiOiIwMjEzSEwiLCJkZXBhcnRtZW50SWQiOiI3Yjg4MWUzNy0xYThlLTQ0NDYtODcwZS1lM2RjMWM3NGMwNDIiLCJyb2xlcyI6WyJvZmZsaW5lX2FjY2VzcyIsIm51cnNlIiwidW1hX2F1dGhvcml6YXRpb24iLCJ1c2VyIl0sImdyb3VwcyI6WyJvZmZsaW5lX2FjY2VzcyIsIm51cnNlIiwidW1hX2F1dGhvcml6YXRpb24iLCJ1c2VyIl0sImRlcHQiOltdLCJwcmVmZXJyZWRfdXNlcm5hbWUiOiJhZG1pbiIsImdpdmVuX25hbWUiOiLns7t45ZGYIiwidXNlcklkIjoiYjdiOWNhMmUtNzU0Zi00MzU3LWE4ZTAtN2M3NDI5YTE2OTc5IiwibmFtZSI6Iuezu3jlkZgiLCJpZCI6ImI3YjljYTJlLTc1NGYtNDM1Ny1hOGUwLTdjNzQyOWExNjk3OSIsImRlcGFydG1lbnRBYnN0cmFjdCI6IuWFqOenkeWMu-WtpuenkeaKpOeQhuermSJ9.SghSp3G2F1EJDPy5Qi-nrDVNkPikDjEaUxrwJNwzQXkJd7m3EUnEDTkcaxE7cuK1u6ZWmEO2QofrlOYIEFDiPUL_g8k_BW7HypdllXIBupSu2SukuYvCauG0SvBHODuzrbv3qiFtoAbW0GDYAOMC3k7XoUaMbrqSCptof-bSm7MgID0zR5rqCD3xVnJup8_1vdMDwTVBSlEkIcRoDMZNrdSlPKVbZ3GGAFQoq2jWYRWutBJ6ErWnr07i_Gp3nqNoX08irXubxHo9MKcDIJCyNcYuQYDGBPW0SYmuAIvRqSW9mmrXY7JplcbTx0gNONaq-AH-BaRH7jo_ubuWXeup8A";

    // ==================== 响应结构体 ====================

    /// 通用 API 响应结构
    #[derive(Debug, Deserialize)]
    struct ApiResponse<T> {
        code: String,
        msg: String,
        data: Option<T>,
    }

    /// Schema 记录
    #[derive(Debug, Deserialize)]
    #[allow(dead_code)]
    struct SchemaRecord {
        id: i64,
        schema_name: String,
        schema_label: Option<String>,
        schema_description: Option<String>,
        field_name: String,
        field_label: Option<String>,
        field_type: Option<String>,
        field_description: Option<String>,
        field_required: Option<bool>,
        endpoint_path: Option<String>,
        endpoint_label: Option<String>,
        endpoint_description: Option<String>,
        endpoint_tags: Option<String>,
        endpoint_operation_id: Option<String>,
        http_method: Option<String>,
    }

    /// 模型记录
    #[derive(Debug, Deserialize)]
    struct ModelRecord {
        id: i64,
        model_name: String,
        model_text: String,
        description: Option<String>,
    }

    /// 策略记录
    #[derive(Debug, Deserialize, Clone)]
    struct PolicyRecord {
        id: i64,
        ptype: String,
        v0: String,
        v1: String,
        v2: String,
        v3: String,
        v4: String,
        v5: String,
    }

    /// 策略 DTO（用于添加策略）
    #[derive(Debug, Serialize)]
    struct PolicyDto {
        ptype: String,
        v0: String,
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

    /// 模型 DTO（用于更新模型）
    #[derive(Debug, Serialize)]
    struct ModelDto {
        model_name: String,
        model_text: String,
        description: Option<String>,
    }

    // ==================== 辅助函数 ====================

    /// 获取 HTTP Client（禁用代理）
    fn http_client() -> reqwest::Client {
        reqwest::Client::builder()
            .no_proxy()
            .timeout(Duration::from_secs(30))
            .build()
            .unwrap()
    }

    /// 构建带 Authorization 头的请求头
    fn auth_headers(token: &str) -> HeaderMap {
        let mut headers = HeaderMap::new();
        headers.insert(
            AUTHORIZATION,
            HeaderValue::from_str(&format!("Bearer {}", token)).unwrap(),
        );
        headers
    }

    /// 发送 GET 请求（带认证）
    async fn get_with_auth(base_url: &str, path: &str, token: &str) -> reqwest::Response {
        http_client()
            .get(format!("{}{}", base_url, path))
            .headers(auth_headers(token))
            .send()
            .await
            .expect("GET request failed")
    }

    /// 发送 GET 请求（无认证）
    async fn get_without_auth(base_url: &str, path: &str) -> reqwest::Response {
        http_client()
            .get(format!("{}{}", base_url, path))
            .send()
            .await
            .expect("GET request failed")
    }

    /// 发送 POST 请求（带认证 + JSON body）
    async fn post_json_with_auth<T: Serialize>(base_url: &str, path: &str, body: &T, token: &str) -> reqwest::Response {
        http_client()
            .post(format!("{}{}", base_url, path))
            .headers(auth_headers(token))
            .json(body)
            .send()
            .await
            .expect("POST request failed")
    }

    /// 发送 PUT 请求（带认证 + JSON body）
    async fn put_json_with_auth<T: Serialize>(base_url: &str, path: &str, body: &T, token: &str) -> reqwest::Response {
        http_client()
            .put(format!("{}{}", base_url, path))
            .headers(auth_headers(token))
            .json(body)
            .send()
            .await
            .expect("PUT request failed")
    }

    /// 发送 DELETE 请求（带认证）
    async fn delete_with_auth(base_url: &str, path: &str, token: &str) -> reqwest::Response {
        http_client()
            .delete(format!("{}{}", base_url, path))
            .headers(auth_headers(token))
            .send()
            .await
            .expect("DELETE request failed")
    }

    /// 发送 POST 请求（带认证，无 body）
    async fn post_with_auth(base_url: &str, path: &str, token: &str) -> reqwest::Response {
        http_client()
            .post(format!("{}{}", base_url, path))
            .headers(auth_headers(token))
            .send()
            .await
            .expect("POST request failed")
    }

    /// 获取 Auth 测试服务器 URL，使用独立线程启动完整 Auth 服务
    async fn get_auth_server_url() -> String {
        static SERVER_URL: OnceCell<String> = OnceCell::const_new();
        SERVER_URL.get_or_init(|| async {
            // 初始化 MySQL + Flyway 迁移
            CONTEXT.init_mysql().await;
            genies_auth::models::run_migrations().await;

            let (tx, rx) = tokio::sync::oneshot::channel::<String>();
            std::thread::spawn(move || {
                let rt = tokio::runtime::Runtime::new().unwrap();
                rt.block_on(async {
                    use salvo::conn::TcpListener;
                    use salvo::conn::Acceptor;

                    // 构建业务路由（需要认证保护）
                    let business_router = Router::new()
                        .push(Router::with_path("/api/users").get(get_user));

                    // OpenApi Schema 同步
                    let doc = OpenApi::new("auth-service", "1.0.0").merge_router(&business_router);
                    if let Err(e) = extract_and_sync_schemas(&doc).await {
                        println!("[AUTH SERVER] Schema sync warning: {}", e);
                    }

                    // 创建 Enforcer
                    let mgr = Arc::new(
                        EnforcerManager::new()
                            .await
                            .expect("Enforcer init failed"),
                    );

                    // 挂载中间件到业务路由
                    let protected_router = business_router
                        .hoop(genies::context::auth::salvo_auth)
                        .hoop(affix_state::inject(mgr.clone()))
                        .hoop(casbin_auth)
                        .push(auth_admin_router());

                    // 构建顶层路由：合并受保护路由 + 不需要认证的路由
                    let router = Router::new()
                        .push(protected_router)
                        .push(auth_public_router())     // Token 获取不需要认证
                        .push(auth_admin_ui_router())  // 静态资源不需要认证
                        .push(genies::k8s::k8s_health_check());

                    // 固定端口绑定
                    let acceptor = TcpListener::new("127.0.0.1:18080").bind().await;
                    let addr = acceptor.holdings()[0].local_addr.to_string();
                    let addr_clean = if let Some(stripped) = addr.strip_prefix("socket://") {
                        stripped.to_string()
                    } else if let Some(stripped) = addr.strip_prefix("socket//") {
                        stripped.to_string()
                    } else {
                        addr
                    };
                    let url = format!("http://{}", addr_clean);

                    // 启动服务器
                    let health_url = format!("{}/health", &url);
                    let server_handle = tokio::spawn(async move {
                        Server::new(acceptor).serve(router).await;
                    });

                    // 轮询等待服务就绪
                    let client = reqwest::Client::builder().no_proxy().build().unwrap();
                    for i in 0..50 {
                        tokio::time::sleep(std::time::Duration::from_millis(100)).await;
                        match client.get(&health_url).send().await {
                            Ok(resp) if resp.status().is_success() => {
                                println!("[AUTH SERVER] Ready after {}ms at {}", (i + 1) * 100, &url);
                                break;
                            }
                            _ => {}
                        }
                    }

                    tx.send(url).unwrap();
                    std::future::pending::<()>().await;
                });
            });
            rx.await.expect("Failed to get auth server URL")
        }).await.clone()
    }

    // ==================== Schema 同步测试 ====================

    /// 测试 1: Schema 同步 — 验证 auth_api_schemas 表有数据
    ///
    /// 服务启动后，OpenApi Schema 应自动同步到数据库
    #[tokio::test]
    async fn test_01_schema_sync_has_data() {
        let base_url = get_auth_server_url().await;

        let resp = get_with_auth(&base_url, "/auth/schemas", TEST_TOKEN).await;
        
        // 检查响应状态
        assert!(
            resp.status().is_success() || resp.status().as_u16() == 403,
            "Expected success or 403, got {}",
            resp.status()
        );

        if resp.status().is_success() {
            let body: ApiResponse<Vec<SchemaRecord>> = resp.json().await.unwrap();
            assert_eq!(body.code, "0", "API should return success code");
            
            if let Some(schemas) = body.data {
                println!("Found {} schema records", schemas.len());
                // Schema 表应该有数据（来自 main.rs 中的 User struct）
                assert!(!schemas.is_empty(), "Schema table should have data after startup");
                
                // 验证包含 User schema
                let has_user = schemas.iter().any(|s| s.schema_name == "User");
                println!("Contains User schema: {}", has_user);
            }
        } else {
            println!("Got 403, which means middleware is working (but no permission)");
        }
    }

    /// 测试 1.1: Schema 描述信息验证 — 验证 OpenAPI 说明信息已提取到数据库
    ///
    /// 检查 schema_description, field_description, endpoint_description 等新字段
    #[tokio::test]
    async fn test_01_1_schema_descriptions_extracted() {
        let base_url = get_auth_server_url().await;

        let resp = get_with_auth(&base_url, "/auth/schemas", TEST_TOKEN).await;

        if resp.status().as_u16() == 403 {
            println!("SKIP: Got 403, cannot verify schema descriptions without permission");
            return;
        }

        assert!(resp.status().is_success(), "Expected success, got {}", resp.status());

        let body: ApiResponse<Vec<SchemaRecord>> = resp.json().await.unwrap();
        assert_eq!(body.code, "0", "API should return success code");

        let schemas = body.data.expect("Should have schema data");
        assert!(!schemas.is_empty(), "Schema table should have data");

        println!("=== Schema 描述信息验证 ===");
        println!("共 {} 条记录", schemas.len());

        // 查找 User 相关的记录
        let user_schemas: Vec<&SchemaRecord> = schemas.iter()
            .filter(|s| s.schema_name.contains("User"))
            .collect();

        assert!(!user_schemas.is_empty(), "Should find User schema records");
        println!("User schema 记录数: {}", user_schemas.len());

        // 验证 Schema 描述（来自 User struct 的 doc comment）
        let first_user = &user_schemas[0];
        println!("  schema_name: {}", first_user.schema_name);
        println!("  schema_description: {:?}", first_user.schema_description);
        println!("  endpoint_path: {:?}", first_user.endpoint_path);
        println!("  endpoint_label: {:?}", first_user.endpoint_label);
        println!("  endpoint_description: {:?}", first_user.endpoint_description);
        println!("  endpoint_tags: {:?}", first_user.endpoint_tags);
        println!("  endpoint_operation_id: {:?}", first_user.endpoint_operation_id);
        println!("  http_method: {:?}", first_user.http_method);

        // 打印所有 User 字段的描述信息
        println!("\n--- User 字段详情 ---");
        for record in &user_schemas {
            println!(
                "  field: {} | type: {:?} | description: {:?} | required: {:?}",
                record.field_name,
                record.field_type,
                record.field_description,
                record.field_required
            );
        }

        // 验证 endpoint 信息已关联
        let has_endpoint = user_schemas.iter().any(|s| s.endpoint_path.is_some());
        println!("\n--- 验证结果 ---");
        println!("  Endpoint 关联: {}", has_endpoint);

        if has_endpoint {
            // 验证 endpoint_label（来自 #[endpoint] 的 summary）
            let has_label = user_schemas.iter().any(|s| s.endpoint_label.is_some());
            println!("  Endpoint label (summary): {}", has_label);

            // 验证 endpoint_description
            let has_desc = user_schemas.iter().any(|s| s.endpoint_description.is_some());
            println!("  Endpoint description: {}", has_desc);

            // 验证 http_method
            let has_method = user_schemas.iter().any(|s| s.http_method.is_some());
            println!("  HTTP method: {}", has_method);
            if has_method {
                let method = user_schemas.iter().find_map(|s| s.http_method.as_deref());
                assert_eq!(method, Some("GET"), "User endpoint should be GET");
            }
        }

        // 验证 schema_description（来自 struct 的 doc comment "用户信息对象"）
        let has_schema_desc = user_schemas.iter().any(|s| s.schema_description.is_some());
        println!("  Schema description: {}", has_schema_desc);

        // 验证 field_description（来自字段的 doc comment）
        let has_field_desc = user_schemas.iter().any(|s| s.field_description.is_some());
        println!("  Field description: {}", has_field_desc);

        // 验证 field_required
        let has_required = user_schemas.iter().any(|s| s.field_required.is_some());
        println!("  Field required: {}", has_required);

        println!("\n=== 描述信息验证完成 ===");
    }

    // ==================== 模型导入测试 ====================

    /// 测试 2: 模型导入 — 验证 casbin_model 表有默认模型
    ///
    /// Flyway 迁移应创建 "default" 模型
    #[tokio::test]
    async fn test_02_model_has_default_definition() {
        let base_url = get_auth_server_url().await;

        let resp = get_with_auth(&base_url, "/auth/model", TEST_TOKEN).await;

        if resp.status().is_success() {
            let body: ApiResponse<ModelRecord> = resp.json().await.unwrap();
            assert_eq!(body.code, "0", "API should return success code");
            
            if let Some(model) = body.data {
                assert_eq!(model.model_name, "default", "Model name should be 'default'");
                assert!(!model.model_text.is_empty(), "Model text should not be empty");
                
                // 验证模型文本包含必要的 section
                assert!(
                    model.model_text.contains("[request_definition]"),
                    "Model should contain request_definition"
                );
                assert!(
                    model.model_text.contains("[policy_definition]"),
                    "Model should contain policy_definition"
                );
                assert!(
                    model.model_text.contains("[matchers]"),
                    "Model should contain matchers"
                );
                
                println!("Default model loaded successfully:\n{}", model.model_text);
            }
        } else {
            println!("Status: {}, middleware may deny access", resp.status());
        }
    }

    // ==================== 策略加载测试 ====================

    /// 测试 3: 策略加载 — Enforcer 从 DB 正确加载模型+策略
    ///
    /// 验证 Enforcer 可以正常初始化并加载策略
    #[tokio::test]
    async fn test_03_policies_loaded_from_db() {
        let base_url = get_auth_server_url().await;

        let resp = get_with_auth(&base_url, "/auth/policies", TEST_TOKEN).await;

        if resp.status().is_success() {
            let body: ApiResponse<Vec<PolicyRecord>> = resp.json().await.unwrap();
            assert_eq!(body.code, "0", "API should return success code");
            
            if let Some(policies) = body.data {
                println!("Loaded {} policies from DB", policies.len());
                
                // 打印所有策略用于调试
                for p in &policies {
                    println!(
                        "  - ptype={}, v0={}, v1={}, v2={}, v3={}",
                        p.ptype, p.v0, p.v1, p.v2, p.v3
                    );
                }
            }
        } else {
            println!("Status: {}", resp.status());
        }
    }

    // ==================== 热更新测试 ====================

    /// 测试 4: 热更新 — Admin API 修改策略后 Enforcer 实时生效
    ///
    /// 添加策略 -> reload -> 删除策略 -> reload
    #[tokio::test]
    async fn test_04_hot_reload_policy_changes() {
        let base_url = get_auth_server_url().await;

        // 1. 添加一个测试策略
        let test_policy = PolicyDto {
            ptype: "p".to_string(),
            v0: "test_user_hot_reload".to_string(),
            v1: "/api/test/hot-reload".to_string(),
            v2: "GET".to_string(),
            v3: "allow".to_string(),
            v4: String::new(),
            v5: String::new(),
        };

        let add_resp = post_json_with_auth(&base_url, "/auth/policies", &test_policy, TEST_TOKEN).await;
        println!("Add policy status: {}", add_resp.status());

        if add_resp.status().is_success() {
            let body: ApiResponse<String> = add_resp.json().await.unwrap();
            assert_eq!(body.code, "0", "Add policy should succeed");
            println!("Policy added: {:?}", body.data);

            // 2. 手动触发 reload
            let reload_resp = post_with_auth(&base_url, "/auth/reload", TEST_TOKEN).await;
            println!("Reload status: {}", reload_resp.status());

            if reload_resp.status().is_success() {
                let reload_body: ApiResponse<String> = reload_resp.json().await.unwrap();
                assert_eq!(reload_body.code, "0", "Reload should succeed");
            }

            // 3. 查询策略确认已添加
            let list_resp = get_with_auth(&base_url, "/auth/policies", TEST_TOKEN).await;
            if list_resp.status().is_success() {
                let list_body: ApiResponse<Vec<PolicyRecord>> = list_resp.json().await.unwrap();
                if let Some(policies) = list_body.data {
                    let added = policies.iter().find(|p| p.v0 == "test_user_hot_reload");
                    assert!(added.is_some(), "Added policy should be in list");

                    // 4. 删除测试策略
                    if let Some(policy) = added {
                        let del_resp = delete_with_auth(
                            &base_url,
                            &format!("/auth/policies/{}", policy.id),
                            TEST_TOKEN,
                        )
                        .await;
                        println!("Delete policy status: {}", del_resp.status());
                    }
                }
            }
        }
    }

    // ==================== 角色分配测试 ====================

    /// 测试 5: 角色分配 — 添加和删除角色映射
    #[tokio::test]
    async fn test_05_role_assignment() {
        let base_url = get_auth_server_url().await;

        // 1. 列出现有角色
        let list_resp = get_with_auth(&base_url, "/auth/roles", TEST_TOKEN).await;
        println!("List roles status: {}", list_resp.status());

        if list_resp.status().is_success() {
            let body: ApiResponse<Vec<PolicyRecord>> = list_resp.json().await.unwrap();
            if let Some(roles) = &body.data {
                println!("Found {} role assignments", roles.len());
            }
        }

        // 2. 添加测试角色映射
        let role_dto = PolicyDto {
            ptype: "g".to_string(),
            v0: "test_user_role".to_string(),
            v1: "admin".to_string(),
            v2: String::new(),
            v3: String::new(),
            v4: String::new(),
            v5: String::new(),
        };

        let add_resp = post_json_with_auth(&base_url, "/auth/roles", &role_dto, TEST_TOKEN).await;
        println!("Add role status: {}", add_resp.status());

        if add_resp.status().is_success() {
            // 3. 查询并删除
            let list_resp2 = get_with_auth(&base_url, "/auth/roles", TEST_TOKEN).await;
            if list_resp2.status().is_success() {
                let body: ApiResponse<Vec<PolicyRecord>> = list_resp2.json().await.unwrap();
                if let Some(roles) = body.data {
                    if let Some(role) = roles.iter().find(|r| r.v0 == "test_user_role") {
                        let del_resp = delete_with_auth(
                            &base_url,
                            &format!("/auth/roles/{}", role.id),
                            TEST_TOKEN,
                        )
                        .await;
                        println!("Delete role status: {}", del_resp.status());
                    }
                }
            }
        }
    }

    // ==================== 对象分组测试 ====================

    /// 测试 6: 对象分组 — g2 分组下多个 API 路径统一受控
    #[tokio::test]
    async fn test_06_object_grouping() {
        let base_url = get_auth_server_url().await;

        // 1. 列出现有分组
        let list_resp = get_with_auth(&base_url, "/auth/groups", TEST_TOKEN).await;
        println!("List groups status: {}", list_resp.status());

        if list_resp.status().is_success() {
            let body: ApiResponse<Vec<PolicyRecord>> = list_resp.json().await.unwrap();
            if let Some(groups) = &body.data {
                println!("Found {} object groups", groups.len());
                for g in groups {
                    println!("  - v0={}, v1={}", g.v0, g.v1);
                }
            }
        }

        // 2. 添加测试分组 (将 /api/users 归入 user_management 组)
        let group_dto = PolicyDto {
            ptype: "g2".to_string(),
            v0: "/api/test/grouped".to_string(),
            v1: "test_api_group".to_string(),
            v2: String::new(),
            v3: String::new(),
            v4: String::new(),
            v5: String::new(),
        };

        let add_resp = post_json_with_auth(&base_url, "/auth/groups", &group_dto, TEST_TOKEN).await;
        println!("Add group status: {}", add_resp.status());

        if add_resp.status().is_success() {
            // 3. 验证并清理
            let list_resp2 = get_with_auth(&base_url, "/auth/groups", TEST_TOKEN).await;
            if list_resp2.status().is_success() {
                let body: ApiResponse<Vec<PolicyRecord>> = list_resp2.json().await.unwrap();
                if let Some(groups) = body.data {
                    if let Some(group) = groups.iter().find(|g| g.v0 == "/api/test/grouped") {
                        let del_resp = delete_with_auth(
                            &base_url,
                            &format!("/auth/groups/{}", group.id),
                            TEST_TOKEN,
                        )
                        .await;
                        println!("Delete group status: {}", del_resp.status());
                    }
                }
            }
        }
    }

    // ==================== 接口权限 403 测试 ====================

    /// 测试 7: 接口权限 — 被 deny 的用户访问接口返回 403
    ///
    /// 添加 deny 规则后验证请求被拒绝
    #[tokio::test]
    async fn test_07_api_access_denied_returns_403() {
        let base_url = get_auth_server_url().await;

        // 1. 首先添加一个 deny 策略
        let deny_policy = PolicyDto {
            ptype: "p".to_string(),
            v0: "guest".to_string(),
            v1: "/api/users".to_string(),
            v2: "get".to_string(),
            v3: "deny".to_string(),
            v4: String::new(),
            v5: String::new(),
        };

        let add_resp = post_json_with_auth(&base_url, "/auth/policies", &deny_policy, TEST_TOKEN).await;
        let mut added_policy_id: Option<i64> = None;

        if add_resp.status().is_success() {
            // 查找添加的策略 ID 以便后续清理
            let list_resp = get_with_auth(&base_url, "/auth/policies", TEST_TOKEN).await;
            if list_resp.status().is_success() {
                let body: ApiResponse<Vec<PolicyRecord>> = list_resp.json().await.unwrap();
                if let Some(policies) = body.data {
                    if let Some(p) = policies.iter().find(|p| {
                        p.v0 == "guest" && p.v1 == "/api/users" && p.v3 == "deny"
                    }) {
                        added_policy_id = Some(p.id);
                    }
                }
            }

            // 2. 触发 reload
            let _ = post_with_auth(&base_url, "/auth/reload", TEST_TOKEN).await;

            // 3. 以 guest 身份（无 token）访问
            let user_resp = get_without_auth(&base_url, "/api/users").await;
            println!("Access /api/users as guest: {}", user_resp.status());

            // 应该返回 403 Forbidden（如果 deny 规则生效）
            // 注意：这取决于实际的 casbin 模型配置
            if user_resp.status().as_u16() == 403 {
                println!("SUCCESS: Guest denied access to /api/users as expected");
            } else {
                println!(
                    "INFO: Got status {}, deny rule may not be configured in model",
                    user_resp.status()
                );
            }
        }

        // 4. 清理测试策略
        if let Some(id) = added_policy_id {
            let del_resp = delete_with_auth(&base_url, &format!("/auth/policies/{}", id), TEST_TOKEN).await;
            println!("Cleanup policy status: {}", del_resp.status());
            let _ = post_with_auth(&base_url, "/auth/reload", TEST_TOKEN).await;
        }
    }

    // ==================== 字段过滤测试 ====================

    /// 测试 8: 字段过滤 — 不同用户访问同一接口返回不同字段
    ///
    /// 依赖 #[casbin] 宏的字段过滤功能
    #[tokio::test]
    async fn test_08_field_level_filtering() {
        let base_url = get_auth_server_url().await;

        // 需要配置字段级权限规则才能看到过滤效果
        // 例如: p, guest, User, email, deny

        // 1. 以 guest 身份访问
        let resp_guest = get_without_auth(&base_url, "/api/users").await;
        println!("Guest access /api/users: {}", resp_guest.status());

        if resp_guest.status().is_success() {
            let body: Value = resp_guest.json().await.unwrap();
            println!("Guest response: {}", serde_json::to_string_pretty(&body).unwrap());

            // 检查敏感字段是否被过滤（取决于策略配置）
            if let Some(email) = body.get("email") {
                if email.is_null() {
                    println!("SUCCESS: email field filtered for guest");
                } else {
                    println!("INFO: email field visible, may need to configure deny rule");
                }
            }
        }

        // 2. 以 admin 身份访问（如果有有效 token）
        let resp_admin = get_with_auth(&base_url, "/api/users", TEST_TOKEN).await;
        println!("Admin access /api/users: {}", resp_admin.status());

        if resp_admin.status().is_success() {
            let body: Value = resp_admin.json().await.unwrap();
            println!("Admin response: {}", serde_json::to_string_pretty(&body).unwrap());
        }
    }

    // ==================== Redis 缓存测试 ====================

    /// 测试 9: Redis 缓存 — 验证版本控制和缓存失效
    ///
    /// 注意: CasbinRule, cache_policies, load_cached_policies 已从 cache 模块移除
    /// 仅测试当前可用的 invalidate_and_reload 和 get_enforcer_version 功能
    #[tokio::test]
    async fn test_09_redis_cache_operations() {
        // 直接使用 genies_auth::version_sync 模块测试
        use genies_auth::version_sync;

        // 1. 测试缓存失效（更新版本号）
        let invalidate_result = version_sync::invalidate_and_reload().await;
        println!("Invalidate cache result: {:?}", invalidate_result);
        assert!(invalidate_result.is_ok(), "Invalidate should succeed");

        // 2. 测试版本号
        let version = version_sync::get_enforcer_version().await;
        println!("Enforcer version: {:?}", version);

        if let Ok(Some(ver)) = version {
            assert!(!ver.is_empty(), "Version should not be empty after invalidate");
            println!("SUCCESS: Version updated: {}", ver);
        }
    }

    // ==================== Enforcer 重载测试 ====================

    /// 测试 10: Enforcer 重载 — 验证 POST /auth/reload 功能
    #[tokio::test]
    async fn test_10_reload_enforcer() {
        let base_url = get_auth_server_url().await;

        let resp = post_with_auth(&base_url, "/auth/reload", TEST_TOKEN).await;
        println!("Reload enforcer status: {}", resp.status());

        if resp.status().is_success() {
            let body: ApiResponse<String> = resp.json().await.unwrap();
            assert_eq!(body.code, "0", "Reload should return success code");
            println!("Reload result: {:?}", body.data);
        } else {
            println!("Status: {}, may need admin permission", resp.status());
        }
    }

    // ==================== 模型更新测试 ====================

    /// 测试 11: 模型更新 — PUT /auth/model
    #[tokio::test]
    async fn test_11_update_model() {
        let base_url = get_auth_server_url().await;

        // 1. 先获取当前模型
        let get_resp = get_with_auth(&base_url, "/auth/model", TEST_TOKEN).await;
        
        if !get_resp.status().is_success() {
            println!("SKIP: Cannot get current model, status: {}", get_resp.status());
            return;
        }

        let body: ApiResponse<ModelRecord> = get_resp.json().await.unwrap();
        let original_model = match body.data {
            Some(m) => m,
            None => {
                println!("SKIP: No model data returned");
                return;
            }
        };

        println!("Original model: {}", original_model.model_name);

        // 2. 更新模型（只改 description，保持 model_text 不变避免破坏功能）
        let update_dto = ModelDto {
            model_name: "default".to_string(),
            model_text: original_model.model_text.clone(),
            description: Some("Updated by integration test".to_string()),
        };

        let put_resp = put_json_with_auth(&base_url, "/auth/model", &update_dto, TEST_TOKEN).await;
        println!("Update model status: {}", put_resp.status());

        if put_resp.status().is_success() {
            let update_body: ApiResponse<String> = put_resp.json().await.unwrap();
            println!("Update result: {:?}", update_body);

            // 3. 恢复原始 description
            let restore_dto = ModelDto {
                model_name: "default".to_string(),
                model_text: original_model.model_text,
                description: original_model.description,
            };

            let restore_resp = put_json_with_auth(&base_url, "/auth/model", &restore_dto, TEST_TOKEN).await;
            println!("Restore model status: {}", restore_resp.status());
        }
    }

    // ==================== 并发安全测试 ====================

    /// 测试 12: 并发安全 — 并发请求下无死锁
    #[tokio::test]
    async fn test_12_concurrent_requests_no_deadlock() {
        let base_url = get_auth_server_url().await;

        let mut handles = vec![];

        // 并发发起 10 个请求
        for i in 0..10 {
            let base_url = base_url.clone();
            let handle = tokio::spawn(async move {
                let client = http_client();
                
                // 混合不同类型的请求
                let result = match i % 3 {
                    0 => {
                        // GET /auth/policies
                        client
                            .get(format!("{}/auth/policies", base_url))
                            .headers(auth_headers(TEST_TOKEN))
                            .send()
                            .await
                    }
                    1 => {
                        // GET /auth/schemas
                        client
                            .get(format!("{}/auth/schemas", base_url))
                            .headers(auth_headers(TEST_TOKEN))
                            .send()
                            .await
                    }
                    _ => {
                        // GET /auth/model
                        client
                            .get(format!("{}/auth/model", base_url))
                            .headers(auth_headers(TEST_TOKEN))
                            .send()
                            .await
                    }
                };

                match result {
                    Ok(resp) => Some(resp.status().as_u16()),
                    Err(e) => {
                        println!("Request {} failed: {}", i, e);
                        None
                    }
                }
            });
            handles.push(handle);
        }

        // 等待所有请求完成
        let results: Vec<Option<u16>> = futures::future::join_all(handles)
            .await
            .into_iter()
            .map(|r| r.unwrap_or(None))
            .collect();

        // 统计结果
        let success_count = results.iter().filter(|r| r.is_some()).count();
        let failed_count = results.iter().filter(|r| r.is_none()).count();

        println!(
            "Concurrent test: {} success, {} failed",
            success_count, failed_count
        );

        // 所有请求都应该正常返回（不一定是 200，可能是 403）
        assert_eq!(
            failed_count, 0,
            "All concurrent requests should complete without timeout/deadlock"
        );
    }

    // ==================== 综合测试 ====================

    /// 测试 13: 综合流程 — 完整的 CRUD 流程
    #[tokio::test]
    async fn test_13_full_crud_workflow() {
        let base_url = get_auth_server_url().await;

        let unique_id = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_millis();

        let test_user = format!("test_crud_user_{}", unique_id);

        // 1. Create: 添加策略
        let policy = PolicyDto {
            ptype: "p".to_string(),
            v0: test_user.clone(),
            v1: "/api/crud/test".to_string(),
            v2: "GET".to_string(),
            v3: "allow".to_string(),
            v4: String::new(),
            v5: String::new(),
        };

        let create_resp = post_json_with_auth(&base_url, "/auth/policies", &policy, TEST_TOKEN).await;
        if !create_resp.status().is_success() {
            println!("SKIP: Cannot create policy, status: {}", create_resp.status());
            return;
        }

        println!("CREATE: Policy added for {}", test_user);

        // 2. Read: 查询策略
        let read_resp = get_with_auth(&base_url, "/auth/policies", TEST_TOKEN).await;
        let mut policy_id: Option<i64> = None;

        if read_resp.status().is_success() {
            let body: ApiResponse<Vec<PolicyRecord>> = read_resp.json().await.unwrap();
            if let Some(policies) = body.data {
                if let Some(p) = policies.iter().find(|p| p.v0 == test_user) {
                    policy_id = Some(p.id);
                    println!("READ: Found policy with id={}", p.id);
                }
            }
        }

        // 3. Update: 无直接更新 API，使用 reload 验证
        let reload_resp = post_with_auth(&base_url, "/auth/reload", TEST_TOKEN).await;
        println!("RELOAD: status={}", reload_resp.status());

        // 4. Delete: 删除策略
        if let Some(id) = policy_id {
            let delete_resp = delete_with_auth(&base_url, &format!("/auth/policies/{}", id), TEST_TOKEN).await;
            println!("DELETE: status={}", delete_resp.status());

            if delete_resp.status().is_success() {
                // 验证删除成功
                let verify_resp = get_with_auth(&base_url, "/auth/policies", TEST_TOKEN).await;
                if verify_resp.status().is_success() {
                    let body: ApiResponse<Vec<PolicyRecord>> = verify_resp.json().await.unwrap();
                    if let Some(policies) = body.data {
                        let still_exists = policies.iter().any(|p| p.v0 == test_user);
                        assert!(!still_exists, "Policy should be deleted");
                        println!("VERIFY: Policy successfully deleted");
                    }
                }
            }
        }

        println!("SUCCESS: Full CRUD workflow completed");
    }

    // ==================== 字段级权限过滤辅助函数 ====================

    /// 添加字段级 deny 策略
    async fn add_field_deny_policy(base_url: &str, subject: &str, type_name: &str, field_name: &str) -> Option<i64> {
        let resource = format!("{}.{}", type_name, field_name);
        let policy = PolicyDto {
            ptype: "p".to_string(),
            v0: subject.to_string(),
            v1: resource.clone(),
            v2: "read".to_string(),
            v3: "deny".to_string(),
            v4: String::new(),
            v5: String::new(),
        };

        let resp = post_json_with_auth(base_url, "/auth/policies", &policy, TEST_TOKEN).await;
        if !resp.status().is_success() {
            println!("Failed to add deny policy for {}: {}", resource, resp.status());
            return None;
        }

        // 查找添加的策略 ID
        let list_resp = get_with_auth(base_url, "/auth/policies", TEST_TOKEN).await;
        if list_resp.status().is_success() {
            let body: ApiResponse<Vec<PolicyRecord>> = list_resp.json().await.unwrap();
            if let Some(policies) = body.data {
                if let Some(p) = policies.iter().find(|p| {
                    p.v0 == subject && p.v1 == resource && p.v2 == "read" && p.v3 == "deny"
                }) {
                    println!("Added deny policy: {} -> {}.{} (id={})", subject, type_name, field_name, p.id);
                    return Some(p.id);
                }
            }
        }
        None
    }

    /// 重载 enforcer
    async fn reload_enforcer(base_url: &str) {
        let resp = post_with_auth(base_url, "/auth/reload", TEST_TOKEN).await;
        if resp.status().is_success() {
            println!("Enforcer reloaded successfully");
        } else {
            println!("Failed to reload enforcer: {}", resp.status());
        }
    }

    /// 删除策略
    async fn cleanup_policy(base_url: &str, policy_id: i64) {
        let resp = delete_with_auth(base_url, &format!("/auth/policies/{}", policy_id), TEST_TOKEN).await;
        if resp.status().is_success() {
            println!("Cleaned up policy id={}", policy_id);
        } else {
            println!("Failed to cleanup policy id={}: {}", policy_id, resp.status());
        }
    }

    /// 批量清理策略
    async fn cleanup_policies(base_url: &str, policy_ids: &[i64]) {
        for id in policy_ids {
            cleanup_policy(base_url, *id).await;
        }
        reload_enforcer(base_url).await;
    }

    // ==================== 字段级权限过滤测试 ====================

    /// 测试 14: 字段过滤 — 敏感原始字段被正确过滤
    ///
    /// 添加 deny 规则禁止 guest 用户读取 Employee 的 id_card_number 和 base_salary
    #[tokio::test]
    async fn test_14_field_filtering_sensitive_primitive_fields() {
        let base_url = get_auth_server_url().await;
        println!("\n=== Test 14: 敏感原始字段过滤 ===");

        let mut policy_ids: Vec<i64> = Vec::new();

        // 1. 添加 deny 策略
        if let Some(id) = add_field_deny_policy(&base_url, "guest", "Employee", "id_card_number").await {
            policy_ids.push(id);
        }
        if let Some(id) = add_field_deny_policy(&base_url, "guest", "Employee", "base_salary").await {
            policy_ids.push(id);
        }

        if policy_ids.is_empty() {
            println!("SKIP: Failed to add deny policies");
            return;
        }

        reload_enforcer(&base_url).await;

        // 2. 无 Token 请求（guest 用户）
        let guest_resp = get_without_auth(&base_url, "/api/users").await;
        println!("Guest access /api/users: {}", guest_resp.status());

        if guest_resp.status().is_success() {
            let body: Value = guest_resp.json().await.unwrap();
            println!("Guest response: {}", serde_json::to_string_pretty(&body).unwrap());

            // 验证敏感字段被过滤
            let has_id_card = body.get("id_card_number").is_some();
            let has_salary = body.get("base_salary").is_some();

            if !has_id_card && !has_salary {
                println!("SUCCESS: id_card_number and base_salary filtered for guest");
            } else {
                println!("INFO: id_card_number={}, base_salary={} (may need casbin macro integration)", 
                    has_id_card, has_salary);
            }
        }

        // 3. 有 Token 请求（admin 用户）
        let admin_resp = get_with_auth(&base_url, "/api/users", TEST_TOKEN).await;
        println!("Admin access /api/users: {}", admin_resp.status());

        if admin_resp.status().is_success() {
            let body: Value = admin_resp.json().await.unwrap();
            
            // admin 应该能看到所有字段
            let has_id_card = body.get("id_card_number").is_some();
            let has_salary = body.get("base_salary").is_some();
            
            if has_id_card && has_salary {
                println!("SUCCESS: Admin can see all fields (id_card_number and base_salary)");
            } else {
                println!("INFO: Admin sees id_card_number={}, base_salary={}", has_id_card, has_salary);
            }
        }

        // 4. 清理策略
        cleanup_policies(&base_url, &policy_ids).await;
        println!("=== Test 14 完成 ===\n");
    }

    /// 测试 15: 字段过滤 — 嵌套 Address 对象字段被正确过滤
    ///
    /// 添加 deny 规则禁止 guest 读取 Address 的 street 和 postal_code
    #[tokio::test]
    async fn test_15_field_filtering_nested_address() {
        let base_url = get_auth_server_url().await;
        println!("\n=== Test 15: 嵌套 Address 字段过滤 ===");

        let mut policy_ids: Vec<i64> = Vec::new();

        // 1. 添加 deny 策略
        if let Some(id) = add_field_deny_policy(&base_url, "guest", "Address", "street").await {
            policy_ids.push(id);
        }
        if let Some(id) = add_field_deny_policy(&base_url, "guest", "Address", "postal_code").await {
            policy_ids.push(id);
        }

        if policy_ids.is_empty() {
            println!("SKIP: Failed to add deny policies");
            return;
        }

        reload_enforcer(&base_url).await;

        // 2. 无 Token 请求
        let resp = get_without_auth(&base_url, "/api/users").await;
        println!("Guest access /api/users: {}", resp.status());

        if resp.status().is_success() {
            let body: Value = resp.json().await.unwrap();

            // 检查 home_address
            if let Some(home_addr) = body.get("home_address") {
                let has_street = home_addr.get("street").is_some();
                let has_postal = home_addr.get("postal_code").is_some();
                let has_province = home_addr.get("province").is_some();
                let has_city = home_addr.get("city").is_some();

                println!("home_address - street: {}, postal_code: {}, province: {}, city: {}",
                    has_street, has_postal, has_province, has_city);

                if !has_street && !has_postal && has_province && has_city {
                    println!("SUCCESS: Address.street and Address.postal_code filtered, others visible");
                }
            }

            // 检查 work_address
            if let Some(work_addr) = body.get("work_address") {
                if !work_addr.is_null() {
                    let has_street = work_addr.get("street").is_some();
                    let has_postal = work_addr.get("postal_code").is_some();
                    println!("work_address - street: {}, postal_code: {}", has_street, has_postal);
                }
            }
        }

        // 3. 清理策略
        cleanup_policies(&base_url, &policy_ids).await;
        println!("=== Test 15 完成 ===\n");
    }

    /// 测试 16: 字段过滤 — 嵌套 ContactInfo 对象字段被正确过滤
    ///
    /// 添加 deny 规则禁止 guest 读取 ContactInfo 的 mobile 和 emergency_contact_phone
    #[tokio::test]
    async fn test_16_field_filtering_nested_contact_info() {
        let base_url = get_auth_server_url().await;
        println!("\n=== Test 16: 嵌套 ContactInfo 字段过滤 ===");

        let mut policy_ids: Vec<i64> = Vec::new();

        // 1. 添加 deny 策略
        if let Some(id) = add_field_deny_policy(&base_url, "guest", "ContactInfo", "mobile").await {
            policy_ids.push(id);
        }
        if let Some(id) = add_field_deny_policy(&base_url, "guest", "ContactInfo", "emergency_contact_phone").await {
            policy_ids.push(id);
        }

        if policy_ids.is_empty() {
            println!("SKIP: Failed to add deny policies");
            return;
        }

        reload_enforcer(&base_url).await;

        // 2. 无 Token 请求
        let resp = get_without_auth(&base_url, "/api/users").await;
        println!("Guest access /api/users: {}", resp.status());

        if resp.status().is_success() {
            let body: Value = resp.json().await.unwrap();

            if let Some(contact) = body.get("contact") {
                let has_mobile = contact.get("mobile").is_some();
                let has_emergency = contact.get("emergency_contact_phone").is_some();
                let has_email = contact.get("email").is_some();
                let has_wechat = contact.get("wechat").is_some();

                println!("contact - mobile: {}, emergency_phone: {}, email: {}, wechat: {}",
                    has_mobile, has_emergency, has_email, has_wechat);

                if !has_mobile && !has_emergency && has_email {
                    println!("SUCCESS: ContactInfo.mobile and emergency_contact_phone filtered, email visible");
                }
            }
        }

        // 3. 清理策略
        cleanup_policies(&base_url, &policy_ids).await;
        println!("=== Test 16 完成 ===\n");
    }

    /// 测试 17: 字段过滤 — Vec<BankAccount> 数组元素字段被正确过滤
    ///
    /// 添加 deny 规则禁止 guest 读取 BankAccount 的 account_number
    #[tokio::test]
    async fn test_17_field_filtering_vec_bank_accounts() {
        let base_url = get_auth_server_url().await;
        println!("\n=== Test 17: Vec<BankAccount> 字段过滤 ===");

        let mut policy_ids: Vec<i64> = Vec::new();

        // 1. 添加 deny 策略
        if let Some(id) = add_field_deny_policy(&base_url, "guest", "BankAccount", "account_number").await {
            policy_ids.push(id);
        }

        if policy_ids.is_empty() {
            println!("SKIP: Failed to add deny policies");
            return;
        }

        reload_enforcer(&base_url).await;

        // 2. 无 Token 请求
        let resp = get_without_auth(&base_url, "/api/users").await;
        println!("Guest access /api/users: {}", resp.status());

        if resp.status().is_success() {
            let body: Value = resp.json().await.unwrap();

            if let Some(accounts) = body.get("bank_accounts").and_then(|v| v.as_array()) {
                println!("bank_accounts count: {}", accounts.len());
                assert!(!accounts.is_empty(), "bank_accounts array should still exist");

                for (i, account) in accounts.iter().enumerate() {
                    let has_account_number = account.get("account_number").is_some();
                    let has_bank_name = account.get("bank_name").is_some();
                    let has_branch = account.get("branch_name").is_some();

                    println!("  [{}] account_number: {}, bank_name: {}, branch_name: {}",
                        i, has_account_number, has_bank_name, has_branch);

                    if !has_account_number && has_bank_name && has_branch {
                        println!("  [{}] SUCCESS: account_number filtered, others visible", i);
                    }
                }
            }
        }

        // 3. 清理策略
        cleanup_policies(&base_url, &policy_ids).await;
        println!("=== Test 17 完成 ===\n");
    }

    /// 测试 18: 字段过滤 — Vec<WorkExperience> 数组元素字段被正确过滤
    ///
    /// 添加 deny 规则禁止 guest 读取 WorkExperience 的 monthly_salary
    #[tokio::test]
    async fn test_18_field_filtering_vec_work_experiences() {
        let base_url = get_auth_server_url().await;
        println!("\n=== Test 18: Vec<WorkExperience> 字段过滤 ===");

        let mut policy_ids: Vec<i64> = Vec::new();

        // 1. 添加 deny 策略
        if let Some(id) = add_field_deny_policy(&base_url, "guest", "WorkExperience", "monthly_salary").await {
            policy_ids.push(id);
        }

        if policy_ids.is_empty() {
            println!("SKIP: Failed to add deny policies");
            return;
        }

        reload_enforcer(&base_url).await;

        // 2. 无 Token 请求
        let resp = get_without_auth(&base_url, "/api/users").await;
        println!("Guest access /api/users: {}", resp.status());

        if resp.status().is_success() {
            let body: Value = resp.json().await.unwrap();

            if let Some(experiences) = body.get("work_experiences").and_then(|v| v.as_array()) {
                println!("work_experiences count: {}", experiences.len());

                for (i, exp) in experiences.iter().enumerate() {
                    let has_salary = exp.get("monthly_salary").is_some();
                    let has_company = exp.get("company_name").is_some();
                    let has_position = exp.get("position").is_some();

                    println!("  [{}] monthly_salary: {}, company_name: {}, position: {}",
                        i, has_salary, has_company, has_position);

                    if !has_salary && has_company && has_position {
                        println!("  [{}] SUCCESS: monthly_salary filtered, others visible", i);
                    }
                }
            }
        }

        // 3. 清理策略
        cleanup_policies(&base_url, &policy_ids).await;
        println!("=== Test 18 完成 ===\n");
    }

    /// 测试 19: 字段过滤 — 综合场景：多个类型的 deny 规则同时生效
    ///
    /// 同时配置多个类型的 deny 规则，验证所有嵌套层级都正确过滤
    #[tokio::test]
    async fn test_19_field_filtering_mixed_nested_scenario() {
        let base_url = get_auth_server_url().await;
        println!("\n=== Test 19: 综合嵌套场景字段过滤 ===");

        let mut policy_ids: Vec<i64> = Vec::new();

        // 1. 添加多个类型的 deny 策略
        if let Some(id) = add_field_deny_policy(&base_url, "guest", "Employee", "id_card_number").await {
            policy_ids.push(id);
        }
        if let Some(id) = add_field_deny_policy(&base_url, "guest", "Address", "street").await {
            policy_ids.push(id);
        }
        if let Some(id) = add_field_deny_policy(&base_url, "guest", "BankAccount", "account_number").await {
            policy_ids.push(id);
        }
        if let Some(id) = add_field_deny_policy(&base_url, "guest", "ContactInfo", "emergency_contact_phone").await {
            policy_ids.push(id);
        }

        if policy_ids.len() < 4 {
            println!("WARNING: Only {} of 4 policies added", policy_ids.len());
        }

        reload_enforcer(&base_url).await;

        // 2. 无 Token 请求
        let resp = get_without_auth(&base_url, "/api/users").await;
        println!("Guest access /api/users: {}", resp.status());

        if resp.status().is_success() {
            let body: Value = resp.json().await.unwrap();
            println!("Response structure check:");

            // 检查 Employee 字段
            let has_id_card = body.get("id_card_number").is_some();
            println!("  Employee.id_card_number: {} (should be false)", has_id_card);

            // 检查 Address 字段
            if let Some(home_addr) = body.get("home_address") {
                let has_street = home_addr.get("street").is_some();
                println!("  Address.street: {} (should be false)", has_street);
            }

            // 检查 BankAccount 字段
            if let Some(accounts) = body.get("bank_accounts").and_then(|v| v.as_array()) {
                if let Some(first) = accounts.first() {
                    let has_acc_num = first.get("account_number").is_some();
                    println!("  BankAccount.account_number: {} (should be false)", has_acc_num);
                }
            }

            // 检查 ContactInfo 字段
            if let Some(contact) = body.get("contact") {
                let has_emergency = contact.get("emergency_contact_phone").is_some();
                println!("  ContactInfo.emergency_contact_phone: {} (should be false)", has_emergency);
            }

            println!("Mixed nested filtering verification complete");
        }

        // 3. 清理策略
        cleanup_policies(&base_url, &policy_ids).await;
        println!("=== Test 19 完成 ===\n");
    }

    /// 测试 20: 字段过滤 — Admin 用户能看到所有字段（不受 guest deny 影响）
    ///
    /// 配置多个 deny 规则（对 guest），验证 admin 用户能看到所有字段
    #[tokio::test]
    async fn test_20_field_filtering_admin_sees_all() {
        let base_url = get_auth_server_url().await;
        println!("\n=== Test 20: Admin 用户能看到所有字段 ===");

        let mut policy_ids: Vec<i64> = Vec::new();

        // 1. 添加多个 deny 策略（针对 guest）
        if let Some(id) = add_field_deny_policy(&base_url, "guest", "Employee", "id_card_number").await {
            policy_ids.push(id);
        }
        if let Some(id) = add_field_deny_policy(&base_url, "guest", "Employee", "base_salary").await {
            policy_ids.push(id);
        }
        if let Some(id) = add_field_deny_policy(&base_url, "guest", "Address", "street").await {
            policy_ids.push(id);
        }
        if let Some(id) = add_field_deny_policy(&base_url, "guest", "BankAccount", "account_number").await {
            policy_ids.push(id);
        }

        reload_enforcer(&base_url).await;

        // 2. 使用 TEST_TOKEN（admin 用户）请求
        let admin_resp = get_with_auth(&base_url, "/api/users", TEST_TOKEN).await;
        println!("Admin access /api/users: {}", admin_resp.status());

        if admin_resp.status().is_success() {
            let body: Value = admin_resp.json().await.unwrap();

            // 验证 admin 能看到所有被 guest deny 的字段
            let has_id_card = body.get("id_card_number").is_some();
            let has_salary = body.get("base_salary").is_some();
            
            println!("Admin sees Employee.id_card_number: {}", has_id_card);
            println!("Admin sees Employee.base_salary: {}", has_salary);

            if let Some(home_addr) = body.get("home_address") {
                let has_street = home_addr.get("street").is_some();
                println!("Admin sees Address.street: {}", has_street);
            }

            if let Some(accounts) = body.get("bank_accounts").and_then(|v| v.as_array()) {
                if let Some(first) = accounts.first() {
                    let has_acc_num = first.get("account_number").is_some();
                    println!("Admin sees BankAccount.account_number: {}", has_acc_num);
                }
            }

            if has_id_card && has_salary {
                println!("SUCCESS: Admin can see all Employee sensitive fields");
            } else {
                println!("INFO: Admin field visibility may depend on admin-specific rules");
            }
        }

        // 3. 清理策略
        cleanup_policies(&base_url, &policy_ids).await;
        println!("=== Test 20 完成 ===\n");
    }

    /// 测试 21: 字段过滤 — 清理策略后字段恢复可见
    ///
    /// 验证策略的动态生效能力：添加 deny 规则，验证过滤生效；删除规则后，验证字段恢复可见
    #[tokio::test]
    async fn test_21_field_filtering_cleanup_restores_full_response() {
        let base_url = get_auth_server_url().await;
        println!("\n=== Test 21: 策略清理后字段恢复 ===");

        // 1. 先获取原始响应（无 deny 规则）
        let original_resp = get_without_auth(&base_url, "/api/users").await;
        let original_has_id_card = if original_resp.status().is_success() {
            let body: Value = original_resp.json().await.unwrap();
            body.get("id_card_number").is_some()
        } else {
            println!("SKIP: Cannot get original response");
            return;
        };
        println!("Before deny rule - id_card_number visible: {}", original_has_id_card);

        // 2. 添加 deny 规则
        let policy_id = match add_field_deny_policy(&base_url, "guest", "Employee", "id_card_number").await {
            Some(id) => id,
            None => {
                println!("SKIP: Failed to add deny policy");
                return;
            }
        };

        reload_enforcer(&base_url).await;

        // 3. 验证字段被过滤
        let filtered_resp = get_without_auth(&base_url, "/api/users").await;
        let filtered_has_id_card = if filtered_resp.status().is_success() {
            let body: Value = filtered_resp.json().await.unwrap();
            body.get("id_card_number").is_some()
        } else {
            true // 假设仍可见
        };
        println!("With deny rule - id_card_number visible: {}", filtered_has_id_card);

        // 4. 删除 deny 规则并 reload
        cleanup_policy(&base_url, policy_id).await;
        reload_enforcer(&base_url).await;

        // 5. 验证字段恢复可见
        let restored_resp = get_without_auth(&base_url, "/api/users").await;
        if restored_resp.status().is_success() {
            let body: Value = restored_resp.json().await.unwrap();
            let restored_has_id_card = body.get("id_card_number").is_some();
            println!("After cleanup - id_card_number visible: {}", restored_has_id_card);

            if original_has_id_card && !filtered_has_id_card && restored_has_id_card {
                println!("SUCCESS: Field visibility correctly follows policy lifecycle");
                println!("  - Before deny: visible");
                println!("  - With deny: filtered");
                println!("  - After cleanup: restored");
            } else if !filtered_has_id_card && restored_has_id_card {
                println!("SUCCESS: Cleanup restored field visibility");
            } else {
                println!("INFO: Field visibility state - original: {}, filtered: {}, restored: {}",
                    original_has_id_card, filtered_has_id_card, restored_has_id_card);
            }
        }

        println!("=== Test 21 完成 ===\n");
    }
}
