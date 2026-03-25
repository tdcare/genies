use salvo::prelude::*;
use salvo::Router;

use crate::dapr_sub::dapr_sub;
use crate::pubsub::DaprTopicSubscription;

pub struct Topicpoint {
    pub subscribe: fn() -> DaprTopicSubscription,
    pub hoop: fn()->Router,
}

impl Topicpoint {
    pub const fn new(subscribe: fn() -> DaprTopicSubscription, hoop: fn()->Router) -> Self {
        Self {
            subscribe,
            hoop,
        }
    }
}

inventory::collect!(Topicpoint);

/// 自动收集所有通过 #[topic] 注册的 topic handler，构建统一的路由
/// 返回一个挂载了所有 topic handler 的 Router（路径为 /daprsub/consumers）
/// 
/// 使用方式：
/// ```rust
/// let router = collect_topic_routers().post(dapr_sub);
/// ```
pub fn collect_topic_routers() -> Router {
    let mut router = Router::with_path("/daprsub/consumers");
    
    for record in inventory::iter::<Topicpoint> {
        let hoop_router = (record.hoop)();
        for hoop in hoop_router.hoops() {
            router.hoops_mut().push(hoop.clone());
        }
    }
    
    router
}

/// 自动收集所有通过 #[topic] 注册的 Dapr 订阅配置
/// 返回所有 DaprTopicSubscription 的列表
/// 
/// 使用方式：
/// ```rust
/// let subscriptions = collect_topic_subscriptions();
/// ```
pub fn collect_topic_subscriptions() -> Vec<DaprTopicSubscription> {
    let mut subscriptions = Vec::new();
    for record in inventory::iter::<Topicpoint> {
        subscriptions.push((record.subscribe)());
    }
    subscriptions
}

/// Dapr 订阅发现端点 handler
/// 自动收集所有通过 #[topic] 注册的订阅配置并返回 JSON
/// 
/// Dapr sidecar 启动时会调用 GET /dapr/subscribe 端点，了解服务订阅了哪些 topic
#[handler]
pub async fn dapr_subscribe_handler(res: &mut Response) {
    let subscriptions = collect_topic_subscriptions();
    res.render(Json(&subscriptions));
}

/// 构建完整的 Dapr 事件消费路由，包含：
/// - GET /dapr/subscribe — 订阅发现端点
/// - POST /daprsub/consumers — 事件消费端点（自动挂载所有 topic handler）
/// 
/// 使用方式：
/// ```rust
/// // 一行代码完成所有 Dapr 订阅路由配置
/// let router = dapr_event_router();
/// ```
pub fn dapr_event_router() -> Router {
    Router::new()
        .push(Router::with_path("/dapr/subscribe").get(dapr_subscribe_handler))
        .push(collect_topic_routers().post(dapr_sub))
}

#[cfg(test)]
mod tests {
    use genies_cache::mem_service::MemService;
    use genies_cache::cache_service::ICacheService;
    use std::time::Duration;
    use std::sync::Arc;

    const CONSUME_STATUS_CONSUMING: &str = "CONSUMING";
    const CONSUME_STATUS_CONSUMED: &str = "CONSUMED";

    /// 测试首次消费成功完整流程
    /// NX("CONSUMING", 60s) -> true -> handler 成功 -> set("CONSUMED", 长TTL) -> get -> "CONSUMED"
    #[tokio::test]
    async fn test_idempotent_first_consumption_success() {
        let cache = MemService::default();
        let key = "server1-handler1-EventType-msg-001";
        let processing_ttl = Some(Duration::from_secs(60));
        let record_ttl = Some(Duration::from_secs(10080 * 60)); // 7天

        // Step 1: NX 抢锁
        let acquired = cache
            .set_string_ex_nx(key, CONSUME_STATUS_CONSUMING, processing_ttl)
            .await
            .unwrap();
        assert!(acquired, "首次 NX 应返回 true");

        // Step 2: 模拟 handler 成功（此处省略实际业务逻辑）

        // Step 3: 设置 CONSUMED 状态
        let _ = cache
            .set_string_ex(key, CONSUME_STATUS_CONSUMED, record_ttl)
            .await
            .unwrap();

        // Step 4: 验证状态为 CONSUMED
        let status = cache.get_string(key).await.unwrap();
        assert_eq!(status, CONSUME_STATUS_CONSUMED, "状态应为 CONSUMED");
        println!("首次消费成功完整流程测试通过，最终状态: {}", status);
    }

    /// 测试已消费跳过
    /// 先 set("CONSUMED") -> NX -> false -> get -> "CONSUMED"（应跳过不重试）
    #[tokio::test]
    async fn test_idempotent_duplicate_consumed_skip() {
        let cache = MemService::default();
        let key = "server1-handler1-EventType-msg-002";

        // 预先设置 CONSUMED 状态
        let _ = cache
            .set_string(key, CONSUME_STATUS_CONSUMED)
            .await
            .unwrap();

        // 尝试 NX 抢锁，应返回 false
        let acquired = cache
            .set_string_ex_nx(key, CONSUME_STATUS_CONSUMING, Some(Duration::from_secs(60)))
            .await
            .unwrap();
        assert!(!acquired, "key 已存在，NX 应返回 false");

        // 验证状态仍为 CONSUMED
        let status = cache.get_string(key).await.unwrap();
        assert_eq!(status, CONSUME_STATUS_CONSUMED, "状态应为 CONSUMED");
        println!("已消费跳过测试通过，状态为: {} - 应跳过不重试", status);
    }

    /// 测试处理中重试
    /// NX(key, "CONSUMING") -> true -> 第二次 NX(同 key) -> false -> get -> "CONSUMING"
    #[tokio::test]
    async fn test_idempotent_duplicate_consuming_retry() {
        let cache = MemService::default();
        let key = "server1-handler1-EventType-msg-003";
        let ttl = Some(Duration::from_secs(60));

        // 首次 NX 抢锁成功
        let first = cache
            .set_string_ex_nx(key, CONSUME_STATUS_CONSUMING, ttl)
            .await
            .unwrap();
        assert!(first, "首次 NX 应成功");

        // 第二次 NX 抢锁失败
        let second = cache
            .set_string_ex_nx(key, CONSUME_STATUS_CONSUMING, ttl)
            .await
            .unwrap();
        assert!(!second, "第二次 NX 应返回 false");

        // 验证状态为 CONSUMING
        let status = cache.get_string(key).await.unwrap();
        assert_eq!(status, CONSUME_STATUS_CONSUMING, "状态应为 CONSUMING");
        println!("处理中重试测试通过，状态为: {} - 应标记重试", status);
    }

    /// 测试失败后可重新消费
    /// NX -> true -> 模拟失败 -> del(key) -> 再次 NX -> true
    #[tokio::test]
    async fn test_idempotent_handler_failure_allows_reprocess() {
        let cache = MemService::default();
        let key = "server1-handler1-EventType-msg-004";
        let ttl = Some(Duration::from_secs(60));

        // Step 1: NX 抢锁成功
        let first = cache
            .set_string_ex_nx(key, CONSUME_STATUS_CONSUMING, ttl)
            .await
            .unwrap();
        assert!(first, "首次 NX 应成功");

        // Step 2: 模拟 handler 失败，删除 key
        let _ = cache.del_string(key).await.unwrap();

        // Step 3: 再次 NX 应成功（可重新消费）
        let retry = cache
            .set_string_ex_nx(key, CONSUME_STATUS_CONSUMING, ttl)
            .await
            .unwrap();
        assert!(retry, "删除 key 后，再次 NX 应成功");
        println!("失败后可重新消费测试通过");
    }

    /// 测试 CONSUMED 更新失败场景
    /// NX -> true, handler 成功 -> key 仍为 "CONSUMING"（模拟 set CONSUMED 未执行）
    #[tokio::test]
    async fn test_idempotent_consumed_update_failure() {
        let cache = MemService::default();
        let key = "server1-handler1-EventType-msg-005";
        let ttl = Some(Duration::from_secs(60));

        // Step 1: NX 抢锁成功
        let acquired = cache
            .set_string_ex_nx(key, CONSUME_STATUS_CONSUMING, ttl)
            .await
            .unwrap();
        assert!(acquired, "NX 应成功");

        // Step 2: 模拟 handler 成功，但 set CONSUMED 未执行（网络故障等）
        // key 仍为 CONSUMING

        // Step 3: 验证状态仍为 CONSUMING
        let status = cache.get_string(key).await.unwrap();
        assert_eq!(status, CONSUME_STATUS_CONSUMING, "状态应仍为 CONSUMING");

        // 此时如果另一个实例尝试消费，应该看到 CONSUMING 状态并标记重试
        let retry_acquired = cache
            .set_string_ex_nx(key, CONSUME_STATUS_CONSUMING, ttl)
            .await
            .unwrap();
        assert!(!retry_acquired, "key 存在，NX 应返回 false");

        let current_status = cache.get_string(key).await.unwrap();
        assert_eq!(current_status, CONSUME_STATUS_CONSUMING, "状态为 CONSUMING 应触发重试");
        println!("CONSUMED 更新失败场景测试通过 - 状态: {}", current_status);
    }

    /// 测试幂等 key 格式验证
    #[test]
    fn test_idempotent_key_format() {
        let server_name = "server1";
        let handler_name = "handler1";
        let event_type_name = "EventType";
        let msg_id = "msg-001";

        let key = format!("{}-{}-{}-{}", server_name, handler_name, event_type_name, msg_id);

        assert!(key.contains("server1"), "key 应包含 server_name");
        assert!(key.contains("handler1"), "key 应包含 handler_name");
        assert!(key.contains("EventType"), "key 应包含 event_type_name");
        assert!(key.contains("msg-001"), "key 应包含 msg_id");
        assert_eq!(key, "server1-handler1-EventType-msg-001", "key 格式应正确");
        println!("幂等 key 格式验证通过: {}", key);
    }

    /// 测试 CONSUMING 过期防死锁
    /// NX(key, "CONSUMING", TTL=1ms) -> sleep 等待过期 -> 再次 NX -> true
    #[tokio::test]
    async fn test_processing_expire_prevents_deadlock() {
        let cache = MemService::default();
        let key = "server1-handler1-EventType-msg-006";

        // 设置极短 TTL 的 CONSUMING 状态
        let first = cache
            .set_string_ex_nx(key, CONSUME_STATUS_CONSUMING, Some(Duration::from_millis(50)))
            .await
            .unwrap();
        assert!(first, "首次 NX 应成功");

        // 等待 TTL 过期
        tokio::time::sleep(Duration::from_millis(100)).await;

        // 再次 NX 应成功（锁自动释放）
        let after_expiry = cache
            .set_string_ex_nx(key, CONSUME_STATUS_CONSUMING, Some(Duration::from_secs(60)))
            .await
            .unwrap();
        assert!(after_expiry, "TTL 过期后，NX 应再次成功（锁自动释放）");
        println!("CONSUMING 过期防死锁测试通过");
    }

    /// 测试并发场景只有一个处理
    #[tokio::test]
    async fn test_concurrent_idempotent_only_one_processes() {
        let cache = Arc::new(MemService::default());
        let key = "server1-handler1-EventType-concurrent-msg";
        let num_tasks = 10;

        let mut handles = vec![];
        for i in 0..num_tasks {
            let svc = Arc::clone(&cache);
            let k = key.to_string();
            let task_id = i;
            let handle = tokio::spawn(async move {
                // 尝试 NX 抢锁
                let acquired = svc
                    .set_string_ex_nx(&k, CONSUME_STATUS_CONSUMING, Some(Duration::from_secs(60)))
                    .await
                    .unwrap();

                if acquired {
                    // 模拟 handler 处理
                    tokio::time::sleep(Duration::from_millis(10)).await;
                    // 设置 CONSUMED 状态
                    let _ = svc
                        .set_string_ex(&k, CONSUME_STATUS_CONSUMED, Some(Duration::from_secs(10080 * 60)))
                        .await
                        .unwrap();
                    println!("Task {} 成功处理", task_id);
                } else {
                    println!("Task {} 未获取到处理权", task_id);
                }
                acquired
            });
            handles.push(handle);
        }

        let mut results = vec![];
        for handle in handles {
            results.push(handle.await.unwrap());
        }

        // 验证只有一个获取到处理权
        let winners: Vec<_> = results.iter().filter(|&&r| r).collect();
        assert_eq!(
            winners.len(),
            1,
            "只有一个 task 应该获取到处理权，实际有 {} 个",
            winners.len()
        );

        // 验证最终状态为 CONSUMED
        let final_status = cache.get_string(key).await.unwrap();
        assert_eq!(final_status, CONSUME_STATUS_CONSUMED, "最终状态应为 CONSUMED");
        println!("并发场景测试通过: {} 个任务中只有 1 个处理，最终状态: {}", num_tasks, final_status);
    }

    /// 测试 CONSUMED TTL 远大于 CONSUMING TTL
    #[test]
    fn test_consumed_ttl_longer_than_processing_ttl() {
        let processing_expire_seconds: u64 = 60;
        let record_reserve_minutes: u64 = 10080; // 7天

        // CONSUMED 的 TTL 应该远大于 CONSUMING 的 TTL（至少 10 倍）
        assert!(
            record_reserve_minutes * 60 > processing_expire_seconds * 10,
            "CONSUMED TTL 应远大于 CONSUMING TTL"
        );
        println!(
            "TTL 配置验证: CONSUMING={}s, CONSUMED={}s ({}倍)",
            processing_expire_seconds,
            record_reserve_minutes * 60,
            (record_reserve_minutes * 60) / processing_expire_seconds
        );
    }

    /// 测试 del 不存在的 key 不报错
    #[tokio::test]
    async fn test_del_nonexistent_key_safe() {
        let cache = MemService::default();
        let result = cache.del_string("nonexistent_key_12345").await;
        assert!(result.is_ok(), "删除不存在的 key 应返回 Ok");
        println!("del 不存在的 key 安全测试通过");
    }
}