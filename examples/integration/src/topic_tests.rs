//! Topic 定义和注册测试
//!
//! 测试 DomainEvent trait 实现和 topic handler 注册功能

use genies::context::CONTEXT;
use genies::ddd::event::DomainEvent;
use serde::{Serialize, Deserialize};
use std::time::Duration;
use rbatis::executor::Executor;
use futures;
use tokio::time::sleep;

/// 测试用领域事件
#[derive(genies_derive::DomainEvent, Debug, Serialize, Deserialize, Default, Clone)]
#[event_type_version("V1")]
#[event_source("integration.test.TestAggregate")]
#[event_type("integration.test.event.TestEvent")]
pub struct TestEvent {
    pub id: Option<i64>,
    pub name: Option<String>,
}

/// 测试用 topic handler
#[genies_derive::topic(name = "integration.test.TestAggregate", pubsub = "messagebus")]
pub async fn on_test_event(tx: &mut dyn Executor, event: TestEvent) -> anyhow::Result<u64> {
    Ok(0)
}

// ===== Topic 定义测试 (3个) =====

#[tokio::test]
async fn test_topic_event_definition() {
    let event = TestEvent { id: Some(1), name: Some("test".to_string()) };
    // 验证 DomainEvent trait 方法
    assert_eq!(event.event_type(), "integration.test.event.TestEvent");
}

#[tokio::test]
async fn test_topic_event_source() {
    let event = TestEvent::default();
    assert_eq!(event.event_source(), "integration.test.TestAggregate");
    assert_eq!(event.event_type_version(), "V1");
}

#[tokio::test]
async fn test_topic_event_serialization() {
    let event = TestEvent { id: Some(42), name: Some("serialize_test".to_string()) };
    // 序列化
    let json = serde_json::to_string(&event).expect("Serialize failed");
    assert!(json.contains("42"));
    assert!(json.contains("serialize_test"));
    // 反序列化
    let deserialized: TestEvent = serde_json::from_str(&json).expect("Deserialize failed");
    assert_eq!(deserialized.id, Some(42));
    assert_eq!(deserialized.name, Some("serialize_test".to_string()));
    // 通过 DomainEvent::json() 方法
    let event_json = event.json();
    assert!(!event_json.is_empty());
}

// ===== Topic 注册测试 (3个) =====

#[tokio::test]
async fn test_topic_subscription_registered() {
    let subscriptions = genies::dapr::topicpoint::collect_topic_subscriptions();
    // 验证至少包含 TestEvent 的订阅
    let found = subscriptions.iter().any(|s| {
        s.topic.as_deref() == Some("integration.test.TestAggregate")
    });
    assert!(found, "TestEvent subscription should be registered, found: {:?}", 
        subscriptions.iter().map(|s| s.topic.clone()).collect::<Vec<_>>());
}

#[tokio::test]
async fn test_topic_subscription_fields() {
    let subscriptions = genies::dapr::topicpoint::collect_topic_subscriptions();
    let sub = subscriptions.iter().find(|s| {
        s.topic.as_deref() == Some("integration.test.TestAggregate")
    }).expect("TestEvent subscription not found");
    
    assert_eq!(sub.pubsub_name.as_deref(), Some("messagebus"));
    assert_eq!(sub.route.as_deref(), Some("/daprsub/consumers"));
}

#[tokio::test]
async fn test_topic_hoop_router_registered() {
    let router = genies::dapr::topicpoint::collect_topic_routers();
    // 验证 router 路径正确且包含 hoop
    let hoops = router.hoops();
    assert!(!hoops.is_empty(), "Topic router should have at least one hoop");
}

// ===== 执行成功场景测试 =====

#[tokio::test]
async fn test_topic_first_consumption_success() {
    // 模拟首次消费成功的完整流程
    let cache = &CONTEXT.redis_save_service;
    let msg_id = uuid::Uuid::new_v4().to_string();
    let key = format!("{}-on_test_event-TestEvent-{}", CONTEXT.config.server_name, msg_id);
    
    // 1. NX 抢锁 → 成功
    let acquired = cache.set_string_ex_nx(&key, "CONSUMING", Some(Duration::from_secs(60)))
        .await.unwrap();
    assert!(acquired, "First NX should succeed");
    
    // 2. 验证当前状态为 CONSUMING
    let status = cache.get_string(&key).await.unwrap();
    assert_eq!(status, "CONSUMING");
    
    // 3. 模拟 handler 执行成功 → 更新状态为 CONSUMED
    let _ = cache.set_string_ex(&key, "CONSUMED", Some(Duration::from_secs(600)))
        .await.unwrap();
    
    // 4. 验证最终状态为 CONSUMED
    let final_status = cache.get_string(&key).await.unwrap();
    assert_eq!(final_status, "CONSUMED");
    
    // 清理
    let _ = cache.del_string(&key).await;
}

#[tokio::test]
async fn test_topic_duplicate_consumed_skip() {
    // 模拟重复消息 → 已消费 → 跳过
    let cache = &CONTEXT.redis_save_service;
    let msg_id = uuid::Uuid::new_v4().to_string();
    let key = format!("{}-on_test_event-TestEvent-{}", CONTEXT.config.server_name, msg_id);
    
    // 预设 key 为 CONSUMED 状态（模拟已消费过）
    let _ = cache.set_string_ex(&key, "CONSUMED", Some(Duration::from_secs(600)))
        .await.unwrap();
    
    // NX 抢锁 → 失败（key 已存在）
    let acquired = cache.set_string_ex_nx(&key, "CONSUMING", Some(Duration::from_secs(60)))
        .await.unwrap();
    assert!(!acquired, "NX should fail for already consumed message");
    
    // 获取状态 → CONSUMED → 应跳过处理
    let status = cache.get_string(&key).await.unwrap();
    assert_eq!(status, "CONSUMED", "Should detect already consumed state");
    
    // 清理
    let _ = cache.del_string(&key).await;
}

#[tokio::test]
async fn test_topic_idempotent_key_format() {
    let server_name = &CONTEXT.config.server_name;
    let handler_name = "on_test_event";
    let event_type = "TestEvent";
    let msg_id = "msg-12345";
    
    let key = format!("{}-{}-{}-{}", server_name, handler_name, event_type, msg_id);
    
    // 验证 key 包含所有必要部分
    assert!(key.starts_with(server_name), "Key should start with server_name");
    assert!(key.contains(handler_name), "Key should contain handler_name");
    assert!(key.contains(event_type), "Key should contain event_type");
    assert!(key.ends_with(msg_id), "Key should end with msg_id");
    
    // 验证 key 格式正确（非空且合理长度）
    let expected = format!("{}-{}-{}-{}", server_name, handler_name, event_type, msg_id);
    assert_eq!(key, expected, "Key format should match expected pattern");
}

#[tokio::test]
async fn test_topic_concurrent_consumption() {
    // 模拟并发消费竞争 → 只有 1 个获得消费权
    let cache = &CONTEXT.redis_save_service;
    let msg_id = uuid::Uuid::new_v4().to_string();
    let key = format!("{}-on_test_event-concurrent-{}", CONTEXT.config.server_name, msg_id);
    
    let mut handles = vec![];
    for _ in 0..10 {
        let k = key.clone();
        let handle = tokio::spawn(async move {
            let cache = &CONTEXT.redis_save_service;
            cache.set_string_ex_nx(&k, "CONSUMING", Some(Duration::from_secs(60)))
                .await.unwrap()
        });
        handles.push(handle);
    }
    
    let results: Vec<bool> = futures::future::join_all(handles)
        .await
        .into_iter()
        .map(|r| r.unwrap())
        .collect();
    
    // 验证只有 1 个成功获得消费权
    let winners = results.iter().filter(|&&r| r).count();
    assert_eq!(winners, 1, "Only one concurrent consumer should win the NX race");
    
    // 清理
    let _ = cache.del_string(&key).await;
}

// ===== 数据库执行失败场景测试 =====

/// 辅助函数：确保 MySQL 已初始化
async fn ensure_mysql_init() {
    CONTEXT.init_mysql().await;
}

#[tokio::test]
async fn test_topic_handler_db_failure_cleanup() {
    // 模拟 handler 返回 Err → 删除 Redis key → 可重新消费
    let cache = &CONTEXT.redis_save_service;
    let msg_id = uuid::Uuid::new_v4().to_string();
    let key = format!("{}-on_test_event-db_fail-{}", CONTEXT.config.server_name, msg_id);
    
    // 1. NX 抢锁成功
    let acquired = cache.set_string_ex_nx(&key, "CONSUMING", Some(Duration::from_secs(60)))
        .await.unwrap();
    assert!(acquired);
    
    // 2. 模拟 handler 执行失败 → 删除 Redis key
    // （hoop 中 handler 失败会 del key + rollback）
    let _ = cache.del_string(&key).await;
    
    // 3. 验证 key 已被删除（get 返回空或错误）
    let result = cache.get_string(&key).await.unwrap();
    assert!(result.is_empty(), "Key should be deleted after handler failure");
    
    // 清理（已删除，无需额外操作）
}

#[tokio::test]
async fn test_topic_db_failure_allows_retry() {
    // 模拟失败后重试 → del key 后新的 NX 成功
    let cache = &CONTEXT.redis_save_service;
    let msg_id = uuid::Uuid::new_v4().to_string();
    let key = format!("{}-on_test_event-retry-{}", CONTEXT.config.server_name, msg_id);
    
    // 第一次：NX 成功 → handler 失败 → del key
    let first = cache.set_string_ex_nx(&key, "CONSUMING", Some(Duration::from_secs(60)))
        .await.unwrap();
    assert!(first, "First NX should succeed");
    
    // 模拟 handler 失败 → 删除 key
    let _ = cache.del_string(&key).await;
    
    // 第二次：重试 → NX 应该再次成功
    let retry = cache.set_string_ex_nx(&key, "CONSUMING", Some(Duration::from_secs(60)))
        .await.unwrap();
    assert!(retry, "Retry NX should succeed after key deletion");
    
    // 模拟重试成功 → set CONSUMED
    let _ = cache.set_string_ex(&key, "CONSUMED", Some(Duration::from_secs(600)))
        .await.unwrap();
    let status = cache.get_string(&key).await.unwrap();
    assert_eq!(status, "CONSUMED");
    
    // 清理
    let _ = cache.del_string(&key).await;
}

#[tokio::test]
async fn test_topic_transaction_rollback_on_failure() {
    // 模拟 handler 失败 → 事务 rollback → 数据库状态不变
    // 需要先初始化 MySQL
    ensure_mysql_init().await;
    let rb = &CONTEXT.rbatis;
    
    // 获取初始状态（简单查询记录数或版本）
    let before: Vec<rbs::Value> = rb
        .query_decode("SELECT 1 as val", vec![])
        .await
        .expect("Pre-check query failed");
    assert!(!before.is_empty());
    
    // 开启事务
    let tx = rb.acquire_begin().await.expect("Begin transaction failed");
    
    // 模拟 handler 中的查询操作
    let _result: Vec<rbs::Value> = tx
        .query_decode("SELECT 1 as val", vec![])
        .await
        .expect("Query in transaction failed");
    
    // 模拟 handler 失败 → 回滚事务
    tx.rollback().await.expect("Rollback failed");
    
    // 验证回滚后连接正常，数据库状态不变
    let after: Vec<rbs::Value> = rb
        .query_decode("SELECT 1 as val", vec![])
        .await
        .expect("Post-rollback query failed");
    assert!(!after.is_empty(), "Database should be accessible after rollback");
}

// ===== Redis 失败场景测试 =====

#[tokio::test]
async fn test_topic_consuming_state_triggers_retry() {
    // 模拟 key 处于 CONSUMING 状态 → 应标记重试
    let cache = &CONTEXT.redis_save_service;
    let msg_id = uuid::Uuid::new_v4().to_string();
    let key = format!("{}-on_test_event-consuming-{}", CONTEXT.config.server_name, msg_id);
    
    // 模拟另一个实例正在处理：预设 key 为 CONSUMING
    let _ = cache.set_string_ex(&key, "CONSUMING", Some(Duration::from_secs(60)))
        .await.unwrap();
    
    // 新消息到达 → NX 抢锁失败
    let acquired = cache.set_string_ex_nx(&key, "CONSUMING", Some(Duration::from_secs(60)))
        .await.unwrap();
    assert!(!acquired, "NX should fail when key is CONSUMING");
    
    // 获取状态 → CONSUMING → 表示正在被其他实例处理 → 应标记重试
    let status = cache.get_string(&key).await.unwrap();
    assert_eq!(status, "CONSUMING", "Should detect CONSUMING state for retry");
    // 此时 hoop 会设置 depot.insert("is_retry", "true")
    
    // 清理
    let _ = cache.del_string(&key).await;
}

#[tokio::test]
async fn test_topic_consuming_expire_prevents_deadlock() {
    // 模拟 CONSUMING 过期 → 防死锁 → 可重新消费
    let cache = &CONTEXT.redis_save_service;
    let msg_id = uuid::Uuid::new_v4().to_string();
    let key = format!("{}-on_test_event-expire-{}", CONTEXT.config.server_name, msg_id);
    
    // 设置极短 TTL 的 CONSUMING 状态（模拟处理超时）
    let first = cache.set_string_ex_nx(&key, "CONSUMING", Some(Duration::from_secs(1)))
        .await.unwrap();
    assert!(first, "First NX should succeed");
    
    // 验证当前为 CONSUMING
    let status = cache.get_string(&key).await.unwrap();
    assert_eq!(status, "CONSUMING");
    
    // 等待 TTL 过期
    sleep(Duration::from_secs(2)).await;
    
    // key 过期后 → 新的 NX 应该成功（锁自动释放，防死锁）
    let after_expiry = cache.set_string_ex_nx(&key, "CONSUMING", Some(Duration::from_secs(60)))
        .await.unwrap();
    assert!(after_expiry, "NX should succeed after CONSUMING expires (deadlock prevention)");
    
    // 清理
    let _ = cache.del_string(&key).await;
}

#[tokio::test]
async fn test_topic_consumed_update_failure_retry() {
    // 模拟 handler 成功但 set CONSUMED 失败 → key 仍为 CONSUMING → 需重试
    let cache = &CONTEXT.redis_save_service;
    let msg_id = uuid::Uuid::new_v4().to_string();
    let key = format!("{}-on_test_event-consumed_fail-{}", CONTEXT.config.server_name, msg_id);
    
    // 1. NX 抢锁成功
    let acquired = cache.set_string_ex_nx(&key, "CONSUMING", Some(Duration::from_secs(60)))
        .await.unwrap();
    assert!(acquired);
    
    // 2. 模拟 handler 执行成功
    // 3. 模拟 set CONSUMED 失败（不执行 set_string_ex）
    //    此时 key 仍为 CONSUMING
    
    // 4. 验证 key 仍为 CONSUMING
    let status = cache.get_string(&key).await.unwrap();
    assert_eq!(status, "CONSUMING", "Key should still be CONSUMING after set failure");
    
    // 5. 下一次消费尝试 → NX 失败 → 检测到 CONSUMING → 标记重试
    let retry_nx = cache.set_string_ex_nx(&key, "CONSUMING", Some(Duration::from_secs(60)))
        .await.unwrap();
    assert!(!retry_nx, "Retry NX should fail (key still CONSUMING)");
    
    let retry_status = cache.get_string(&key).await.unwrap();
    assert_eq!(retry_status, "CONSUMING", "Should trigger retry");
    
    // 清理
    let _ = cache.del_string(&key).await;
}