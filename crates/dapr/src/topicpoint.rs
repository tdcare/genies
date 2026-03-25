use salvo::Router;

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