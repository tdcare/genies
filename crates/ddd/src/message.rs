/*
 * @Author: tzw
 * @Date: 2021-10-31 23:26:50
 * @LastEditors: tzw
 * @LastEditTime: 2022-01-18 09:30:14
 */
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use rbatis::crud;

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct Headers {
    pub ID: Option<String>,
    pub PARTITION_ID: Option<String>,
    pub DESTINATION: Option<String>,
    pub DATE: Option<String>,
    #[serde(rename = "event-aggregate-type")]
    pub event_aggregate_type: Option<String>,
    #[serde(rename = "event-aggregate-id")]
    pub event_aggregate_id: Option<String>,
    #[serde(rename = "event-type")]
    pub event_type: Option<String>,
    #[serde(flatten)]
    pub extra: HashMap<String, Value>,
}

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct MessageImpl {
    pub payload: String,
    pub headers: Headers,
}

impl MessageImpl {
    pub fn new(headers: Headers, payload: String) -> Self {
        MessageImpl {
            payload,
            headers,
        }
    }
}

/// 保存到数据库的 message
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Message {
    pub id: Option<String>,
    pub destination: Option<String>,
    pub headers: Option<String>,
    pub payload: String,
    pub published: Option<u32>,
    pub creation_time: Option<i64>,
}
crud!(Message{},"message");

#[cfg(test)]
mod tests {
    use super::*;

    /// 测试有效的 CloudEvent.data 正确解析为 MessageImpl
    #[test]
    fn test_cloud_event_data_to_message_impl() {
        let message_data = serde_json::json!({
            "payload": "{\"id\":1,\"name\":\"test-device\"}",
            "headers": {
                "ID": "msg-id-001",
                "PARTITION_ID": "partition-1",
                "DESTINATION": "dest-topic",
                "DATE": "2024-01-01",
                "event-aggregate-type": "DeptDeviceEntity",
                "event-aggregate-id": "agg-123",
                "event-type": "me.tdcarefor.tdnis.device.event.DeviceUseEvent"
            }
        });

        let message_impl: MessageImpl = serde_json::from_value(message_data).unwrap();

        assert_eq!(message_impl.payload, "{\"id\":1,\"name\":\"test-device\"}");
        assert_eq!(message_impl.headers.ID, Some("msg-id-001".to_string()));
        assert_eq!(message_impl.headers.event_type, Some("me.tdcarefor.tdnis.device.event.DeviceUseEvent".to_string()));
        println!("MessageImpl 解析成功: {:?}", message_impl);
    }

    /// 测试无效数据解析为 MessageImpl 使用 unwrap_or_default()
    #[test]
    fn test_invalid_data_message_impl_default() {
        let invalid_data = serde_json::json!("invalid message impl data");

        let message_impl: MessageImpl = serde_json::from_value(invalid_data).unwrap_or_default();
        let default_impl = MessageImpl::default();

        assert_eq!(message_impl.headers.event_type, default_impl.headers.event_type);
        assert!(message_impl.headers.event_type.is_none(), "默认 MessageImpl 的 event_type 应为 None");
        println!("无效数据返回默认 MessageImpl: {:?}", message_impl);
    }

    /// 测试 MessageImpl::default() 的 headers.event_type 为 None
    #[test]
    fn test_message_impl_default_event_type_empty() {
        let default_impl = MessageImpl::default();
        
        assert!(default_impl.headers.event_type.is_none(), "默认 event_type 应为 None");
        println!("MessageImpl::default() 的 event_type: {:?}", default_impl.headers.event_type);
        
        let event_type_str = default_impl.headers.event_type.unwrap_or_default();
        assert_eq!(event_type_str, "", "unwrap_or_default() 应得到空字符串");
    }

    /// 测试 Headers 的各字段正确解析
    #[test]
    fn test_headers_fields_parse() {
        let headers_json = serde_json::json!({
            "ID": "header-id-001",
            "PARTITION_ID": "partition-abc",
            "DESTINATION": "destination-topic",
            "DATE": "2024-03-26",
            "event-aggregate-type": "TestAggregateType",
            "event-aggregate-id": "agg-id-456",
            "event-type": "me.tdcarefor.tdnis.device.event.DeviceUseEvent"
        });

        let headers: Headers = serde_json::from_value(headers_json).unwrap();

        assert_eq!(headers.ID, Some("header-id-001".to_string()));
        assert_eq!(headers.PARTITION_ID, Some("partition-abc".to_string()));
        assert_eq!(headers.DESTINATION, Some("destination-topic".to_string()));
        assert_eq!(headers.DATE, Some("2024-03-26".to_string()));
        assert_eq!(headers.event_aggregate_type, Some("TestAggregateType".to_string()));
        assert_eq!(headers.event_aggregate_id, Some("agg-id-456".to_string()));
        assert_eq!(headers.event_type, Some("me.tdcarefor.tdnis.device.event.DeviceUseEvent".to_string()));
        println!("Headers 解析成功: {:?}", headers);
    }

    /// 测试 None headers ID 导致 panic
    #[test]
    #[should_panic]
    fn test_none_headers_id_causes_panic() {
        let headers = Headers::default();
        let _id = headers.ID.clone().unwrap();
    }
}
