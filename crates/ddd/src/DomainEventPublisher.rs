#[warn(non_snake_case)]

use fastdate::DateTime;
use rbatis::executor::Executor;
use uuid::Uuid;

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
        id: Some(Uuid::new_v4().to_string()),
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

    let  headers = Headers {
        ID: Some(aggregate_id.clone()),
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
    let aggregate_id = Some(Uuid::new_v4().to_string());
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
