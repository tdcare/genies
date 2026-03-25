use salvo::prelude::*;
use salvo::Router;

mod UseDeviceListeners;
mod DeviceUseEvent;

use genies::dapr::dapr_sub::dapr_sub;
// 手动注册时需要导入每个 hoop
use crate::UseDeviceListeners::{onDeviceUseEvent1_hoop, onDeviceUseEvent_hoop};
// 手动注册订阅发现端点时需要导入每个 _dapr 函数
use crate::UseDeviceListeners::{onDeviceUseEvent_dapr, onDeviceUseEvent1_dapr};

/// 方式一：完全自动化（推荐）— 一行代码搞定
/// 
/// 包含：
/// - GET /dapr/subscribe — 订阅发现端点
/// - POST /daprsub/consumers — 事件消费端点
pub fn event_consumer_config_full_auto() -> Router {
    genies::dapr_event_router()
}

/// 方式二：半自动 — 自动收集 handler，手动组装路由
/// 
/// 适用于需要自定义路由结构的场景
pub fn event_consumer_config_auto() -> Router {
    Router::new()
        .push(Router::with_path("/dapr/subscribe").get(genies::dapr_subscribe_handler))
        .push(genies::collect_topic_routers().post(dapr_sub))
}

/// 方式三：手动注册 — 完全手动（与原生产代码一致）
/// 
/// 使用这种方式需要：
/// 1. 手动导入每个 handler 的 `_hoop` 和 `_dapr` 函数
/// 2. 依次调用 `.hoop()` 添加每个 handler
/// 3. 手动编写订阅发现端点 handler
pub fn event_consumer_config_manual() -> Router {
    Router::new()
        // 订阅发现端点：手动返回订阅列表
        .push(Router::with_path("/dapr/subscribe").get(manual_subscribe_handler))
        // 事件消费端点：手动添加每个 hoop
        .push(
            Router::with_path("/daprsub/consumers")
                .hoop(onDeviceUseEvent_hoop)
                .hoop(onDeviceUseEvent1_hoop)
                .post(dapr_sub)
        )
}

/// 手动编写的订阅发现 handler
/// 返回所有手动配置的 DaprTopicSubscription
#[handler]
async fn manual_subscribe_handler(res: &mut Response) {
    let subscriptions = vec![
        onDeviceUseEvent_dapr(),
        onDeviceUseEvent1_dapr(),
    ];
    res.render(Json(&subscriptions));
}

/// 默认的事件消费者配置（使用完全自动化方式）
pub fn event_consumer_config() -> Router {
    event_consumer_config_full_auto()
}

/// 手动收集所有 topic handler 的订阅配置
/// 用于测试验证与自动收集的一致性
pub fn collect_subscriptions_manual() -> Vec<genies::dapr::pubsub::DaprTopicSubscription> {
    vec![
        onDeviceUseEvent_dapr(),
        onDeviceUseEvent1_dapr(),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;
    use genies::dapr::topicpoint::Topicpoint;

    /// 测试完全自动化方式和半自动方式的路由数量一致
    #[test]
    fn test_full_auto_and_auto_routers_have_same_count() {
        let full_auto_router = event_consumer_config_full_auto();
        let auto_router = event_consumer_config_auto();

        let full_auto_count = full_auto_router.routers().len();
        let auto_count = auto_router.routers().len();

        println!("完全自动化路由数量: {}", full_auto_count);
        println!("半自动路由数量: {}", auto_count);

        assert_eq!(
            full_auto_count, auto_count,
            "完全自动化和半自动的子路由数量应该一致"
        );
        // 应该有 2 个子路由：/dapr/subscribe 和 /daprsub/consumers
        assert_eq!(full_auto_count, 2, "应该有 2 个子路由");
    }

    /// 测试 dapr_event_router 包含正确的路由结构
    #[test]
    fn test_dapr_event_router_structure() {
        let router = genies::dapr_event_router();
        
        println!("路由结构: {:#?}", router);
        
        // 应该有 2 个子路由
        assert_eq!(
            router.routers().len(),
            2,
            "dapr_event_router 应该有 2 个子路由"
        );
    }

    /// 测试自动收集的订阅配置与手动收集的一致
    #[test]
    fn test_auto_collect_subscriptions_matches_manual() {
        let manual_subscriptions = collect_subscriptions_manual();
        let auto_subscriptions = genies::collect_topic_subscriptions();

        println!("手动收集的订阅数量: {}", manual_subscriptions.len());
        println!("自动收集的订阅数量: {}", auto_subscriptions.len());

        // 订阅数量应该一致
        assert_eq!(
            manual_subscriptions.len(), auto_subscriptions.len(),
            "手动和自动收集的订阅数量应该一致"
        );

        // 验证订阅配置内容
        for (i, (manual, auto)) in manual_subscriptions.iter().zip(auto_subscriptions.iter()).enumerate() {
            println!("订阅 {} - 手动: topic={:?}, pubsub={:?}", 
                i, manual.topic, manual.pubsub_name);
            println!("订阅 {} - 自动: topic={:?}, pubsub={:?}", 
                i, auto.topic, auto.pubsub_name);

            assert_eq!(
                manual.topic, auto.topic,
                "第 {} 个订阅的 topic 应该一致", i
            );
            assert_eq!(
                manual.pubsub_name, auto.pubsub_name,
                "第 {} 个订阅的 pubsub_name 应该一致", i
            );
            assert_eq!(
                manual.route, auto.route,
                "第 {} 个订阅的 route 应该一致", i
            );
        }
    }

    /// 测试三种方式的 Router 都注册在正确的路径上
    #[test]
    fn test_all_routers_have_correct_structure() {
        let full_auto_router = event_consumer_config_full_auto();
        let auto_router = event_consumer_config_auto();
        let manual_router = event_consumer_config_manual();

        // 打印路由结构以便调试
        println!("完全自动化路由结构: {:#?}", full_auto_router);
        println!("半自动路由结构: {:#?}", auto_router);
        println!("手动注册路由结构: {:#?}", manual_router);

        // 验证所有路由都有子路由
        assert!(
            !full_auto_router.routers().is_empty(),
            "完全自动化 Router 应该有子路由"
        );
        assert!(
            !auto_router.routers().is_empty(),
            "半自动 Router 应该有子路由"
        );
        assert!(
            !manual_router.routers().is_empty(),
            "手动注册 Router 应该有子路由"
        );
    }

    /// 测试通过 inventory 迭代器获取的 Topicpoint 数量正确
    #[test]
    fn test_inventory_topicpoint_count() {
        let mut count = 0;
        for _record in genies::context::inventory::iter::<Topicpoint> {
            count += 1;
        }

        println!("通过 inventory 收集的 Topicpoint 数量: {}", count);
        assert_eq!(count, 2, "应该收集到 2 个 Topicpoint");
    }

    /// 测试每个 Topicpoint 的订阅配置有效
    #[test]
    fn test_each_topicpoint_subscription_is_valid() {
        for record in genies::context::inventory::iter::<Topicpoint> {
            let subscription = (record.subscribe)();
            
            println!("订阅配置: {:?}", subscription);
            
            assert!(
                subscription.topic.is_some(),
                "订阅配置的 topic 不应为空"
            );
            assert!(
                subscription.pubsub_name.is_some(),
                "订阅配置的 pubsub_name 不应为空"
            );
            assert!(
                subscription.route.is_some(),
                "订阅配置的 route 不应为空"
            );
            
            // 验证 route 路径正确
            assert_eq!(
                subscription.route.as_deref(),
                Some("/daprsub/consumers"),
                "route 应该是 /daprsub/consumers"
            );
        }
    }

    /// 测试每个 Topicpoint 生成的 hoop router 有效
    #[test]
    fn test_each_topicpoint_hoop_router_is_valid() {
        for record in genies::context::inventory::iter::<Topicpoint> {
            let hoop_router = (record.hoop)();
            
            println!("hoop router hoops 数量: {}", hoop_router.hoops().len());
            
            // 每个 hoop router 应该有 1 个 hoop
            assert_eq!(
                hoop_router.hoops().len(),
                1,
                "每个 Topicpoint 的 hoop router 应该有 1 个 hoop"
            );
        }
    }
}
