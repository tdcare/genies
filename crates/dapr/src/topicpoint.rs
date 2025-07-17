use std::sync::Arc;

use salvo::{Handler, Router};

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