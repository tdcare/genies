#![allow(non_snake_case)]

use fastdate::DateTime;
use genies_core::id_gen;
use rbatis::executor::Executor;

use crate::{
    aggregate::{AggregateType, WithAggregateId},
    event::DomainEvent,
    message::{Headers, Message, MessageImpl},
};

/*
 * @Author: tzw
 * @Date: 2021-10-21 23:58:37
 * @LastEditors: tzw
 * @LastEditTime: 2022-01-18 09:41:39
 */

/// 发送通用领域事件
pub async fn publishGenericDomainEvent(tx: &mut dyn Executor, domain_event: Box<dyn DomainEvent>) {
    let m = buildGenericMessage(domain_event);
    let headers = m.headers;
    let payload = m.payload;

    let message = Message {
        headers: Some(serde_json::to_string(&headers).unwrap()),
        id: headers.ID,
        destination: headers.DESTINATION,
        payload,
        published: Some(0),
        creation_time: Some(DateTime::now().unix_timestamp_millis()),
    };
    Message::insert(tx, &message).await.unwrap();
}
/// 发送聚合根产生的 领域事件
pub async fn publish<A: AggregateType + WithAggregateId>(
    tx: &mut dyn Executor,
    aggregate: &A,
    domain_event: Box<dyn DomainEvent>,
) {
    let m = buildMessage(aggregate, domain_event);
    let headers = m.headers;
    let payload = m.payload;
    let message = Message {
        id: headers.ID.clone(),
        destination: headers.clone().DESTINATION,
        headers: Some(serde_json::to_string(&headers).unwrap()),
        payload,
        published: Some(0),
        creation_time: Some(DateTime::now().unix_timestamp_millis()),
    };
    Message::insert(tx, &message).await.unwrap();
}

pub fn buildMessage<A: AggregateType + WithAggregateId>(
    aggregate: &A,
    domain_event: Box<dyn DomainEvent>,
) -> MessageImpl {
    let mut aggregate_id = serde_json::to_string(aggregate.aggregate_id()).unwrap();
    if aggregate_id.starts_with("\"") && aggregate_id.ends_with("\"") {
        aggregate_id = (&aggregate_id[1..aggregate_id.len() - 1]).to_string();
    }
    let aggregate_type = aggregate.aggregate_type().to_string();
    // 修复：为每条领域事件分配全局唯一消息 ID，作为 CloudEvent id 与下游 #[topic] 消费端
    // 幂等去重键（key=server-handler-事件类型-headers.ID）。此前误用 aggregate_id 作 ID，
    // 会导致同一聚合根+同一事件类型的后续事件命中相同去重键被当重复丢弃；与 buildGenericMessage
    // 对齐（其 ID 即 next_id()）。聚合归属仍由 PARTITION_ID 与 event_aggregate_id 承载
    // （cdc 按 event_aggregate_id 分区，同实体有序不受影响）。
    let event_id = id_gen::next_id();

    let  headers = Headers {
        ID: Some(event_id),
        PARTITION_ID: Some(aggregate_id.clone()),
        DESTINATION: Some(aggregate_type.clone()),
        DATE: None,
        event_aggregate_type: Some(aggregate_type),
        event_aggregate_id: Some(aggregate_id),
        event_type: Some(domain_event.event_type().to_string()),
        extra: Default::default(),
    };
    // let payload = serde_json::to_string(domain_event).unwrap();
    let payload = domain_event.json();
    MessageImpl::new(headers, payload)
}

pub fn buildGenericMessage(domain_event: Box<dyn DomainEvent>) -> MessageImpl {
    let aggregate_id = Some(id_gen::next_id());
    let  headers = Headers {
        ID: aggregate_id.clone(),
        PARTITION_ID: aggregate_id.clone(),
        DESTINATION: Some("GenericDomainEvent".to_string()),
        DATE: None,
        event_aggregate_type: Some("GenericDomainEvent".to_string()),
        event_aggregate_id: aggregate_id,
        event_type: Some(domain_event.event_type().to_string()),
        extra: Default::default(),
    };
    // let payload = serde_json::to_string(domain_event).unwrap();
    let payload = domain_event.json();
    MessageImpl::new(headers, payload)
}

#[cfg(test)]
mod id_fix_tests {
    use super::buildMessage;
    use crate::aggregate::{AggregateType, WithAggregateId};
    use crate::event::DomainEvent;

    struct DummyAgg { id: String }
    impl AggregateType for DummyAgg {
        fn aggregate_type(&self) -> String { "dummy.Agg".to_string() }
        fn atype() -> String { "dummy.Agg".to_string() }
    }
    impl WithAggregateId for DummyAgg {
        type Id = String;
        fn aggregate_id(&self) -> &Self::Id { &self.id }
    }

    #[derive(serde::Serialize)]
    struct DummyEv;
    impl DomainEvent for DummyEv {
        fn event_type_version(&self) -> String { "V1".to_string() }
        fn event_type(&self) -> String { "dummy.Ev".to_string() }
        fn event_source(&self) -> String { "dummy".to_string() }
        fn json(&self) -> String { serde_json::to_string(self).unwrap() }
    }

    #[test]
    fn messages_id_unique_but_aggregate_partition_stable() {
        genies_core::id_gen::init(1, 1); // 单测环境手动初始化雪花 ID 生成器
        let a = DummyAgg { id: "entity-1".to_string() };
        let m1 = buildMessage(&a, Box::new(DummyEv));
        let m2 = buildMessage(&a, Box::new(DummyEv));
        assert_ne!(m1.headers.ID, m2.headers.ID, "headers.ID must be unique per message");
        assert_eq!(m1.headers.event_aggregate_id.as_deref(), Some("entity-1"));
        assert_eq!(m2.headers.event_aggregate_id.as_deref(), Some("entity-1"));
        assert_eq!(m1.headers.PARTITION_ID.as_deref(), Some("entity-1"));
    }
}
