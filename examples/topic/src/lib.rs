use salvo::Router;

mod UseDeviceListeners;
mod DeviceUseEvent;

use genies::dapr::dapr_sub::dapr_sub;
// 手动注册时需要导入每个 hoop
use crate::UseDeviceListeners::{onDeviceUseEvent1_hoop, onDeviceUseEvent_hoop};

/// 手动注册方式 - 显式导入每个 topic handler 并添加 hoop
/// 
/// 使用这种方式需要：
/// 1. 手动导入每个 handler 的 `_hoop` 函数
/// 2. 依次调用 `.hoop()` 添加每个 handler
pub fn event_consumer_config_manual() -> Router {
    Router::new().push(
        Router::with_path("/daprsub/consumers")
            .hoop(onDeviceUseEvent_hoop)
            .hoop(onDeviceUseEvent1_hoop)
            .post(dapr_sub)
    )
}

/// 自动注册方式 - 通过 inventory 自动收集所有 topic handler
/// 
/// 使用这种方式：
/// 1. 无需手动导入每个 handler
/// 2. 所有使用 `#[topic]` 标注的函数会自动被收集
pub fn event_consumer_config_auto() -> Router {
    Router::new().push(
        genies::collect_topic_routers().post(dapr_sub)
    )
}

/// 默认的事件消费者配置（使用自动注册方式）
pub fn event_consumer_config() -> Router {
    event_consumer_config_auto()
}

/// 手动收集所有 topic handler 的订阅配置
/// 用于测试验证与自动收集的一致性
pub fn collect_subscriptions_manual() -> Vec<genies::dapr::pubsub::DaprTopicSubscription> {
    use crate::UseDeviceListeners::{onDeviceUseEvent_dapr, onDeviceUseEvent1_dapr};
    vec![
        onDeviceUseEvent_dapr(),
        onDeviceUseEvent1_dapr(),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;
    use genies::dapr::topicpoint::Topicpoint;

    /// 测试手动和自动注册的 Router 的 hoops 数量一致
    #[test]
    fn test_manual_and_auto_routers_have_same_hoops_count() {
        let manual_router = event_consumer_config_manual();
        let auto_router = event_consumer_config_auto();

        // 获取子路由的 hoops 数量
        let manual_hoops_count = manual_router.routers().first()
            .map(|r| r.hoops().len())
            .unwrap_or(0);
        let auto_hoops_count = auto_router.routers().first()
            .map(|r| r.hoops().len())
            .unwrap_or(0);

        println!("手动注册 hoops 数量: {}", manual_hoops_count);
        println!("自动注册 hoops 数量: {}", auto_hoops_count);

        assert_eq!(
            manual_hoops_count, auto_hoops_count,
            "手动注册和自动注册的 hoops 数量应该一致"
        );
        // 确保至少有 2 个 handler
        assert_eq!(manual_hoops_count, 2, "应该有 2 个 topic handler");
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

    /// 测试两种方式的 Router 都注册在正确的路径上
    #[test]
    fn test_both_routers_have_correct_path() {
        let manual_router = event_consumer_config_manual();
        let auto_router = event_consumer_config_auto();

        // 打印路由结构以便调试
        println!("手动注册路由结构: {:#?}", manual_router);
        println!("自动注册路由结构: {:#?}", auto_router);

        // 验证子路由都存在
        assert!(
            !manual_router.routers().is_empty(),
            "手动注册的 Router 应该有子路由"
        );
        assert!(
            !auto_router.routers().is_empty(),
            "自动注册的 Router 应该有子路由"
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
