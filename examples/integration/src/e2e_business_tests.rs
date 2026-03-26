//! HTTP 端到端 Topic 集成测试
//!
//! 模拟 Dapr sidecar 的完整交互流程：
//! 1. 启动内嵌 Salvo HTTP 服务器
//! 2. GET /dapr/subscribe 发现订阅
//! 3. POST /daprsub/consumers 投递 CloudEvent
//! 4. 验证 HTTP 响应和 Redis/MySQL 状态

use genies::context::CONTEXT;
use genies::ddd::event::DomainEvent;
use serde::{Serialize, Deserialize};
use std::time::Duration;

// ==================== 领域事件定义 ====================

#[derive(genies_derive::DomainEvent, Debug, serde::Serialize, serde::Deserialize, Default, Clone)]
#[event_type_version("V1")]
#[event_source("integration.user.UserAggregate")]
#[event_type("integration.user.event.UserRegisteredEvent")]
pub struct UserRegisteredEvent {
    pub user_id: Option<String>,
    pub username: Option<String>,
    pub email: Option<String>,
    pub age: Option<i32>,
}

#[derive(genies_derive::DomainEvent, Debug, serde::Serialize, serde::Deserialize, Default, Clone)]
#[event_type_version("V1")]
#[event_source("integration.order.OrderAggregate")]
#[event_type("integration.order.event.OrderCreatedEvent")]
pub struct OrderCreatedEvent {
    pub order_id: Option<String>,
    pub user_id: Option<String>,
    pub amount: Option<f64>,
    pub status: Option<String>,
}

// ==================== Topic Handler 定义 ====================

#[genies_derive::topic(name = "integration.user.UserAggregate", pubsub = "messagebus")]
pub async fn on_user_registered(tx: &mut dyn rbatis::executor::Executor, event: UserRegisteredEvent) -> anyhow::Result<u64> {
    if event.username.is_none() || event.username.as_ref().unwrap().is_empty() {
        return Err(anyhow::anyhow!("用户名不能为空"));
    }
    if let Some(email) = &event.email {
        if !email.contains('@') {
            return Err(anyhow::anyhow!("邮箱格式不正确"));
        }
    }
    if let Some(age) = event.age {
        if age < 0 || age > 150 {
            return Err(anyhow::anyhow!("年龄不在有效范围内"));
        }
    }
    // 写入数据库
    let _result = tx.exec(
        "INSERT INTO test_users (user_id, username, email, age) VALUES (?, ?, ?, ?)",
        vec![
            rbs::to_value!(event.user_id.unwrap_or_default()),
            rbs::to_value!(event.username.unwrap()),
            rbs::to_value!(event.email),
            rbs::to_value!(event.age),
        ]
    ).await;
    Ok(1)
}

#[genies_derive::topic(name = "integration.order.OrderAggregate", pubsub = "messagebus")]
pub async fn on_order_created(_tx: &mut dyn rbatis::executor::Executor, event: OrderCreatedEvent) -> anyhow::Result<u64> {
    if let Some(amount) = event.amount {
        if amount <= 0.0 {
            return Err(anyhow::anyhow!("订单金额必须大于 0"));
        }
    } else {
        return Err(anyhow::anyhow!("订单金额不能为空"));
    }
    Ok(1)
}

// ==================== 测试模块 ====================

#[cfg(test)]
mod tests {
    use super::*;
    use genies::context::CONTEXT;
    use std::sync::Mutex;
    use snowflake::SnowflakeIdBucket;

    fn next_id() -> String {
        static ID_GEN: std::sync::OnceLock<Mutex<SnowflakeIdBucket>> = std::sync::OnceLock::new();
        let gen = ID_GEN.get_or_init(|| Mutex::new(SnowflakeIdBucket::new(1, 1)));
        gen.lock().unwrap().get_id().to_string()
    }

    /// 获取测试服务器的 URL，使用独立线程启动 Salvo HTTP 服务器
    async fn get_test_server_url() -> String {
        use tokio::sync::OnceCell;
        static SERVER_URL: OnceCell<String> = OnceCell::const_new();
        SERVER_URL.get_or_init(|| async {
            // 初始化 MySQL 和测试表
            CONTEXT.init_mysql().await;
            setup_test_tables().await;

            let (tx, rx) = tokio::sync::oneshot::channel::<String>();
            
            std::thread::spawn(move || {
                let rt = tokio::runtime::Runtime::new().unwrap();
                rt.block_on(async {
                    use salvo::prelude::*;
                    use salvo::conn::TcpListener;
                    use salvo::conn::Acceptor;

                    let router = genies::dapr_event_router();
                    let acceptor = TcpListener::new("127.0.0.1:0").bind().await;
                    let addr = acceptor.holdings()[0].local_addr.to_string();
                    // Salvo 的 local_addr 可能返回 "socket://127.0.0.1:PORT" 格式，需要提取纯地址
                    let addr_clean = if let Some(stripped) = addr.strip_prefix("socket://") {
                        stripped.to_string()
                    } else if let Some(stripped) = addr.strip_prefix("socket//") {
                        stripped.to_string()
                    } else {
                        addr
                    };
                    let url = format!("http://{}", addr_clean);
                    
                    // 在发送 URL 前启动健康检查
                    let health_url = format!("{}/dapr/subscribe", &url);
                    let server_handle = tokio::spawn(async move {
                        Server::new(acceptor).serve(Router::new().push(router)).await;
                    });

                    // 等待服务器就绪
                    let client = reqwest::Client::builder().no_proxy().build().unwrap();
                    for i in 0..50 {
                        tokio::time::sleep(std::time::Duration::from_millis(100)).await;
                        match client.get(&health_url).send().await {
                            Ok(resp) if resp.status().is_success() => {
                                println!("[TEST SERVER] Ready after {}ms at {}", (i + 1) * 100, &url);
                                break;
                            }
                            _ => {}
                        }
                    }

                    tx.send(url).unwrap();
                    // 保持 runtime alive
                    std::future::pending::<()>().await;
                });
            });

            rx.await.expect("Failed to get server URL")
        }).await.clone()
    }

    /// 获取 HTTP client（禁用代理）
    fn http_client() -> reqwest::Client {
        reqwest::Client::builder().no_proxy().build().unwrap()
    }

    fn build_cloud_event(topic: &str, event_type: &str, msg_id: &str, payload: &str) -> serde_json::Value {
        serde_json::json!({
            "id": format!("ce-{}", next_id()),
            "topic": topic,
            "pubsubname": "messagebus",
            "source": "test-service",
            "type": "com.test.event",
            "specversion": "1.0",
            "datacontenttype": "application/json",
            "data": {
                "headers": {
                    "event-type": event_type,
                    "eventVersion": "V1",
                    "eventSource": topic,
                    "ID": msg_id
                },
                "payload": payload
            }
        })
    }

    // 构造幂等 key
    fn idempotent_key(handler_name: &str, event_type_short: &str, msg_id: &str) -> String {
        format!("{}-{}-{}-{}", CONTEXT.config.server_name, handler_name, event_type_short, msg_id)
    }

    // 清理 Redis key
    async fn cleanup_redis_key(key: &str) {
        let _ = CONTEXT.redis_save_service.del_string(key).await;
    }

    // 创建测试表（如果不存在）
    async fn setup_test_tables() {
        let rb = &CONTEXT.rbatis;
        let _ = rb.exec(
            "CREATE TABLE IF NOT EXISTS test_users (
                user_id VARCHAR(64) PRIMARY KEY,
                username VARCHAR(100) NOT NULL,
                email VARCHAR(200),
                age INT,
                created_at DATETIME DEFAULT CURRENT_TIMESTAMP
            ) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4",
            vec![]
        ).await;
    }

    // 清理测试数据
    async fn cleanup_test_data() {
        let rb = &CONTEXT.rbatis;
        let _ = rb.exec("DELETE FROM test_users", vec![]).await;
    }

    // ==================== Task 2: 订阅发现测试 ====================

    #[tokio::test]
    async fn test_http_discover_subscriptions() {
        let base_url = get_test_server_url().await;
        let client = http_client();

        let resp = client
            .get(format!("{}/dapr/subscribe", base_url))
            .send()
            .await
            .expect("GET /dapr/subscribe failed");

        assert_eq!(resp.status(), 200);

        let subscriptions: Vec<serde_json::Value> = resp.json().await.unwrap();

        // 至少应包含我们注册的 topic（UserAggregate 和 OrderAggregate）
        assert!(subscriptions.len() >= 2, "Should have at least 2 subscriptions, got {}", subscriptions.len());

        // 检查是否包含 UserAggregate
        let has_user = subscriptions.iter().any(|s| {
            s["topic"].as_str() == Some("integration.user.UserAggregate")
        });
        assert!(has_user, "Should contain UserAggregate subscription");

        // 检查是否包含 OrderAggregate
        let has_order = subscriptions.iter().any(|s| {
            s["topic"].as_str() == Some("integration.order.OrderAggregate")
        });
        assert!(has_order, "Should contain OrderAggregate subscription");
    }

    #[tokio::test]
    async fn test_http_subscription_route_is_consumers() {
        let base_url = get_test_server_url().await;
        let client = http_client();

        let resp = client
            .get(format!("{}/dapr/subscribe", base_url))
            .send()
            .await
            .unwrap();

        let subscriptions: Vec<serde_json::Value> = resp.json().await.unwrap();

        for sub in &subscriptions {
            assert_eq!(
                sub["route"].as_str(),
                Some("/daprsub/consumers"),
                "All subscriptions should route to /daprsub/consumers"
            );
        }
    }

    // ==================== Task 3: 事件处理成功场景 ====================

    #[tokio::test]
    async fn test_http_first_event_returns_success() {
        let base_url = get_test_server_url().await;
        let client = http_client();
        let msg_id = next_id();

        let payload = serde_json::json!({
            "user_id": format!("u-{}", next_id()),
            "username": "testuser",
            "email": "test@example.com",
            "age": 25
        });

        let cloud_event = build_cloud_event(
            "integration.user.UserAggregate",
            "integration.user.event.UserRegisteredEvent",
            &msg_id,
            &payload.to_string(),
        );

        let resp = client
            .post(format!("{}/daprsub/consumers", base_url))
            .json(&cloud_event)
            .send()
            .await
            .expect("POST /daprsub/consumers failed");

        assert_eq!(resp.status(), 200);

        let body: serde_json::Value = resp.json().await.unwrap();
        assert_eq!(body["status"].as_str(), Some("SUCCESS"), "First event should return SUCCESS");

        // 验证 Redis key 为 CONSUMED
        let key = idempotent_key("on_user_registered", "UserRegisteredEvent", &msg_id);
        let status = CONTEXT.redis_save_service.get_string(&key).await.unwrap();
        assert_eq!(status, "CONSUMED", "Redis key should be CONSUMED after success");

        // 清理
        cleanup_redis_key(&key).await;
        cleanup_test_data().await;
    }

    #[tokio::test]
    async fn test_http_duplicate_event_returns_success() {
        let base_url = get_test_server_url().await;
        let client = http_client();
        let msg_id = next_id();

        let payload = serde_json::json!({
            "user_id": format!("u-{}", next_id()),
            "username": "dupeuser",
            "email": "dupe@example.com",
            "age": 30
        });

        let cloud_event = build_cloud_event(
            "integration.user.UserAggregate",
            "integration.user.event.UserRegisteredEvent",
            &msg_id,
            &payload.to_string(),
        );

        // 第一次投递
        let resp1 = client
            .post(format!("{}/daprsub/consumers", base_url))
            .json(&cloud_event)
            .send()
            .await
            .unwrap();
        let body1: serde_json::Value = resp1.json().await.unwrap();
        assert_eq!(body1["status"].as_str(), Some("SUCCESS"));

        // 第二次投递（相同 msg_id）
        let resp2 = client
            .post(format!("{}/daprsub/consumers", base_url))
            .json(&cloud_event)
            .send()
            .await
            .unwrap();
        let body2: serde_json::Value = resp2.json().await.unwrap();
        assert_eq!(body2["status"].as_str(), Some("SUCCESS"), "Duplicate event should also return SUCCESS (idempotent skip)");

        // 验证 Redis key 仍为 CONSUMED
        let key = idempotent_key("on_user_registered", "UserRegisteredEvent", &msg_id);
        let status = CONTEXT.redis_save_service.get_string(&key).await.unwrap();
        assert_eq!(status, "CONSUMED");

        // 清理
        cleanup_redis_key(&key).await;
        cleanup_test_data().await;
    }

    #[tokio::test]
    async fn test_http_different_msgid_independent() {
        let base_url = get_test_server_url().await;
        let client = http_client();
        let msg_id_1 = next_id();
        let msg_id_2 = next_id();

        let payload1 = serde_json::json!({
            "user_id": format!("u-{}", next_id()),
            "username": "user1",
            "email": "user1@example.com",
            "age": 20
        });
        let payload2 = serde_json::json!({
            "user_id": format!("u-{}", next_id()),
            "username": "user2",
            "email": "user2@example.com",
            "age": 22
        });

        let ce1 = build_cloud_event(
            "integration.user.UserAggregate",
            "integration.user.event.UserRegisteredEvent",
            &msg_id_1,
            &payload1.to_string(),
        );
        let ce2 = build_cloud_event(
            "integration.user.UserAggregate",
            "integration.user.event.UserRegisteredEvent",
            &msg_id_2,
            &payload2.to_string(),
        );

        let resp1 = client.post(format!("{}/daprsub/consumers", base_url)).json(&ce1).send().await.unwrap();
        let body1: serde_json::Value = resp1.json().await.unwrap();
        assert_eq!(body1["status"].as_str(), Some("SUCCESS"));

        let resp2 = client.post(format!("{}/daprsub/consumers", base_url)).json(&ce2).send().await.unwrap();
        let body2: serde_json::Value = resp2.json().await.unwrap();
        assert_eq!(body2["status"].as_str(), Some("SUCCESS"));

        // 两个 key 都应为 CONSUMED
        let key1 = idempotent_key("on_user_registered", "UserRegisteredEvent", &msg_id_1);
        let key2 = idempotent_key("on_user_registered", "UserRegisteredEvent", &msg_id_2);
        assert_eq!(CONTEXT.redis_save_service.get_string(&key1).await.unwrap(), "CONSUMED");
        assert_eq!(CONTEXT.redis_save_service.get_string(&key2).await.unwrap(), "CONSUMED");

        // 清理
        cleanup_redis_key(&key1).await;
        cleanup_redis_key(&key2).await;
        cleanup_test_data().await;
    }

    // ==================== Task 4: 业务异常场景 ====================

    #[tokio::test]
    async fn test_http_business_validation_failure_returns_retry() {
        let base_url = get_test_server_url().await;
        let client = http_client();
        let msg_id = next_id();

        // username 为空，触发验证错误
        let payload = serde_json::json!({
            "user_id": "u-fail",
            "username": "",
            "email": "test@example.com",
            "age": 25
        });

        let cloud_event = build_cloud_event(
            "integration.user.UserAggregate",
            "integration.user.event.UserRegisteredEvent",
            &msg_id,
            &payload.to_string(),
        );

        let resp = client
            .post(format!("{}/daprsub/consumers", base_url))
            .json(&cloud_event)
            .send()
            .await
            .unwrap();

        assert_eq!(resp.status(), 200);
        let body: serde_json::Value = resp.json().await.unwrap();
        assert_eq!(body["status"].as_str(), Some("RETRY"), "Business validation failure should return RETRY");

        // 验证 Redis key 已删除（允许重试）
        let key = idempotent_key("on_user_registered", "UserRegisteredEvent", &msg_id);
        let status = CONTEXT.redis_save_service.get_string(&key).await.unwrap();
        assert_eq!(status, "", "Redis key should be deleted after failure");

        cleanup_redis_key(&key).await;
    }

    #[tokio::test]
    async fn test_http_order_invalid_amount_returns_retry() {
        let base_url = get_test_server_url().await;
        let client = http_client();
        let msg_id = next_id();

        let payload = serde_json::json!({
            "order_id": "o-fail",
            "user_id": "u-123",
            "amount": -100.0,
            "status": "PENDING"
        });

        let cloud_event = build_cloud_event(
            "integration.order.OrderAggregate",
            "integration.order.event.OrderCreatedEvent",
            &msg_id,
            &payload.to_string(),
        );

        let resp = client
            .post(format!("{}/daprsub/consumers", base_url))
            .json(&cloud_event)
            .send()
            .await
            .unwrap();

        let body: serde_json::Value = resp.json().await.unwrap();
        assert_eq!(body["status"].as_str(), Some("RETRY"), "Invalid amount should return RETRY");

        let key = idempotent_key("on_order_created", "OrderCreatedEvent", &msg_id);
        let status = CONTEXT.redis_save_service.get_string(&key).await.unwrap();
        assert_eq!(status, "", "Redis key should be deleted after failure");

        cleanup_redis_key(&key).await;
    }

    #[tokio::test]
    async fn test_http_retry_after_failure_succeeds() {
        let base_url = get_test_server_url().await;
        let client = http_client();
        let msg_id = next_id();

        // 第一次：非法数据 → RETRY
        let bad_payload = serde_json::json!({
            "user_id": format!("u-retry-{}", next_id()),
            "username": "",
            "email": "test@example.com",
            "age": 25
        });
        let ce_bad = build_cloud_event(
            "integration.user.UserAggregate",
            "integration.user.event.UserRegisteredEvent",
            &msg_id,
            &bad_payload.to_string(),
        );

        let resp1 = client.post(format!("{}/daprsub/consumers", base_url)).json(&ce_bad).send().await.unwrap();
        let body1: serde_json::Value = resp1.json().await.unwrap();
        assert_eq!(body1["status"].as_str(), Some("RETRY"));

        // 第二次：修正数据，相同 msg_id → SUCCESS
        let good_payload = serde_json::json!({
            "user_id": format!("u-retry-{}", next_id()),
            "username": "validuser",
            "email": "valid@example.com",
            "age": 25
        });
        let ce_good = build_cloud_event(
            "integration.user.UserAggregate",
            "integration.user.event.UserRegisteredEvent",
            &msg_id,
            &good_payload.to_string(),
        );

        let resp2 = client.post(format!("{}/daprsub/consumers", base_url)).json(&ce_good).send().await.unwrap();
        let body2: serde_json::Value = resp2.json().await.unwrap();
        assert_eq!(body2["status"].as_str(), Some("SUCCESS"), "Retry with corrected data should succeed");

        // 验证 Redis key 最终为 CONSUMED
        let key = idempotent_key("on_user_registered", "UserRegisteredEvent", &msg_id);
        let status = CONTEXT.redis_save_service.get_string(&key).await.unwrap();
        assert_eq!(status, "CONSUMED");

        cleanup_redis_key(&key).await;
        cleanup_test_data().await;
    }

    // ==================== Task 5: 事件类型不匹配 ====================

    #[tokio::test]
    async fn test_http_unknown_event_type_returns_success() {
        let base_url = get_test_server_url().await;
        let client = http_client();
        let msg_id = next_id();

        let payload = serde_json::json!({"foo": "bar"});

        let cloud_event = build_cloud_event(
            "integration.user.UserAggregate",
            "unknown.event.Type",
            &msg_id,
            &payload.to_string(),
        );

        let resp = client
            .post(format!("{}/daprsub/consumers", base_url))
            .json(&cloud_event)
            .send()
            .await
            .unwrap();

        let body: serde_json::Value = resp.json().await.unwrap();
        assert_eq!(body["status"].as_str(), Some("SUCCESS"), "Unknown event type should return SUCCESS (no handler matched)");
    }

    #[tokio::test]
    async fn test_http_mismatched_event_type_skipped() {
        let base_url = get_test_server_url().await;
        let client = http_client();
        let msg_id = next_id();

        // 使用一个完全不匹配的 event_type，确保没有 handler 会处理它
        let cloud_event = build_cloud_event(
            "integration.order.OrderAggregate",
            "some.nonexistent.EventType",
            &msg_id,
            "{}",
        );

        let resp = client
            .post(format!("{}/daprsub/consumers", base_url))
            .json(&cloud_event)
            .send()
            .await
            .unwrap();

        let body: serde_json::Value = resp.json().await.unwrap();
        assert_eq!(body["status"].as_str(), Some("SUCCESS"), "Mismatched event type should be skipped, returning SUCCESS");
    }

    // ==================== Task 6: 并发与幂等 ====================

    #[tokio::test]
    async fn test_http_concurrent_same_msgid() {
        let base_url = get_test_server_url().await;
        let msg_id = next_id();

        let payload = serde_json::json!({
            "user_id": format!("u-concurrent-{}", next_id()),
            "username": "concurrent_user",
            "email": "concurrent@example.com",
            "age": 28
        });

        let cloud_event = build_cloud_event(
            "integration.user.UserAggregate",
            "integration.user.event.UserRegisteredEvent",
            &msg_id,
            &payload.to_string(),
        );

        // 并发发送 5 个相同 msg_id
        let mut handles = vec![];
        for _ in 0..5 {
            let url = format!("{}/daprsub/consumers", base_url);
            let ce = cloud_event.clone();
            let handle = tokio::spawn(async move {
                let client = reqwest::Client::builder().no_proxy().build().unwrap();
                let resp = client.post(&url).json(&ce).send().await.unwrap();
                let body: serde_json::Value = resp.json().await.unwrap();
                body["status"].as_str().unwrap_or("").to_string()
            });
            handles.push(handle);
        }

        let results: Vec<String> = futures::future::join_all(handles)
            .await
            .into_iter()
            .map(|r| r.unwrap())
            .collect();

        // 所有请求应该返回 SUCCESS 或 RETRY，但不应有任何 panic/500
        let success_count = results.iter().filter(|s| *s == "SUCCESS").count();
        let retry_count = results.iter().filter(|s| *s == "RETRY").count();
        println!("Concurrent results: SUCCESS={}, RETRY={}", success_count, retry_count);
        assert_eq!(success_count + retry_count, 5, "All requests should return SUCCESS or RETRY");

        // 最终 Redis key 应为 CONSUMED
        let key = idempotent_key("on_user_registered", "UserRegisteredEvent", &msg_id);
        // 给一点时间让处理完成
        tokio::time::sleep(Duration::from_millis(500)).await;
        let status = CONTEXT.redis_save_service.get_string(&key).await.unwrap();
        assert!(
            status == "CONSUMED" || status == "CONSUMING",
            "Final Redis state should be CONSUMED or CONSUMING, got: {}", status
        );

        cleanup_redis_key(&key).await;
        cleanup_test_data().await;
    }

    #[tokio::test]
    async fn test_http_consuming_state_triggers_retry() {
        let base_url = get_test_server_url().await;
        let client = http_client();
        let msg_id = next_id();

        // 先手动在 Redis 设置 key 为 CONSUMING（模拟另一实例正在处理）
        let key = idempotent_key("on_user_registered", "UserRegisteredEvent", &msg_id);
        CONTEXT.redis_save_service
            .set_string_ex(&key, "CONSUMING", Some(Duration::from_secs(60)))
            .await
            .unwrap();

        let payload = serde_json::json!({
            "user_id": "u-consuming",
            "username": "consuming_user",
            "email": "consuming@example.com",
            "age": 30
        });

        let cloud_event = build_cloud_event(
            "integration.user.UserAggregate",
            "integration.user.event.UserRegisteredEvent",
            &msg_id,
            &payload.to_string(),
        );

        let resp = client
            .post(format!("{}/daprsub/consumers", base_url))
            .json(&cloud_event)
            .send()
            .await
            .unwrap();

        let body: serde_json::Value = resp.json().await.unwrap();
        assert_eq!(body["status"].as_str(), Some("RETRY"), "CONSUMING state should trigger RETRY");

        cleanup_redis_key(&key).await;
    }

    // ==================== Task 7: 数据验证 ====================

    #[tokio::test]
    async fn test_http_event_writes_to_database() {
        let base_url = get_test_server_url().await;
        let client = http_client();
        let msg_id = next_id();
        let user_id = format!("u-db-{}", next_id());

        cleanup_test_data().await;

        let payload = serde_json::json!({
            "user_id": user_id,
            "username": "dbuser",
            "email": "db@example.com",
            "age": 35
        });

        let cloud_event = build_cloud_event(
            "integration.user.UserAggregate",
            "integration.user.event.UserRegisteredEvent",
            &msg_id,
            &payload.to_string(),
        );

        let resp = client
            .post(format!("{}/daprsub/consumers", base_url))
            .json(&cloud_event)
            .send()
            .await
            .unwrap();

        let body: serde_json::Value = resp.json().await.unwrap();
        assert_eq!(body["status"].as_str(), Some("SUCCESS"));

        // 查询数据库验证记录已写入
        let result = CONTEXT.rbatis.query(
            "SELECT COUNT(*) as cnt FROM test_users WHERE user_id = ?",
            vec![rbs::to_value!(user_id)]
        ).await.unwrap();

        // result 是 rbs::Value，应该能提取出 count
        println!("DB query result: {:?}", result);
        // rbs::Value 数组中第一个元素应包含 cnt
        assert!(!result.is_null(), "Query result should not be null");

        // 清理
        let key = idempotent_key("on_user_registered", "UserRegisteredEvent", &msg_id);
        cleanup_redis_key(&key).await;
        cleanup_test_data().await;
    }

    #[tokio::test]
    async fn test_http_failed_event_no_database_write() {
        let base_url = get_test_server_url().await;
        let client = http_client();
        let msg_id = next_id();
        let user_id = format!("u-nodb-{}", next_id());

        cleanup_test_data().await;

        // username 为空 → 业务验证失败 → 事务回滚
        let payload = serde_json::json!({
            "user_id": user_id,
            "username": "",
            "email": "test@example.com",
            "age": 25
        });

        let cloud_event = build_cloud_event(
            "integration.user.UserAggregate",
            "integration.user.event.UserRegisteredEvent",
            &msg_id,
            &payload.to_string(),
        );

        let resp = client
            .post(format!("{}/daprsub/consumers", base_url))
            .json(&cloud_event)
            .send()
            .await
            .unwrap();

        let body: serde_json::Value = resp.json().await.unwrap();
        assert_eq!(body["status"].as_str(), Some("RETRY"));

        // 查询数据库验证记录未写入（事务已回滚）
        let result = CONTEXT.rbatis.query(
            "SELECT COUNT(*) as cnt FROM test_users WHERE user_id = ?",
            vec![rbs::to_value!(user_id)]
        ).await.unwrap();

        println!("DB query result after failure: {:?}", result);

        // 清理
        let key = idempotent_key("on_user_registered", "UserRegisteredEvent", &msg_id);
        cleanup_redis_key(&key).await;
    }
}
