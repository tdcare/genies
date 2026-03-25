use serde::{Deserialize, Serialize};
use serde_json::Value;

/*
 * @Author: tzw
 * @Date: 2021-11-16 21:16:13
 * @LastEditors: tzw
 * @LastEditTime: 2021-12-19 20:46:41
 */

#[derive(Debug, Serialize, Deserialize, Clone, Default)]
pub struct CloudEvent {
    pub id: Option<String>,
    pub traceid: Option<String>,
    pub topic: Option<String>,
    #[serde(rename = "pubsubname")]
    pub pubsub_name: Option<String>,
    pub source: Option<String>,
    pub r#type: Option<String>,
    pub specversion: Option<String>,
    pub datacontenttype: Option<String>,
    pub data: Value,
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 测试构造有效的 CloudEvent JSON 字符串，验证解析成功
    #[test]
    fn test_valid_cloud_event_parse() {
        let json = serde_json::json!({
            "id": "test-id-123",
            "traceid": "trace-456",
            "topic": "me.tdcarefor.tdnis.device.domain.DeptDeviceEntity",
            "pubsubname": "messagebus",
            "source": "test-source",
            "type": "com.example.event",
            "specversion": "1.0",
            "datacontenttype": "application/json",
            "data": {"payload": "test"}
        });

        let cloud_event: CloudEvent = serde_json::from_str(&json.to_string()).unwrap();
        
        assert_eq!(cloud_event.id, Some("test-id-123".to_string()));
        assert_eq!(cloud_event.traceid, Some("trace-456".to_string()));
        assert_eq!(cloud_event.topic, Some("me.tdcarefor.tdnis.device.domain.DeptDeviceEntity".to_string()));
        assert_eq!(cloud_event.pubsub_name, Some("messagebus".to_string()));
        assert_eq!(cloud_event.source, Some("test-source".to_string()));
        assert_eq!(cloud_event.r#type, Some("com.example.event".to_string()));
        assert_eq!(cloud_event.specversion, Some("1.0".to_string()));
        assert_eq!(cloud_event.datacontenttype, Some("application/json".to_string()));
        println!("CloudEvent 解析成功: {:?}", cloud_event);
    }

    /// 测试无效 JSON -> unwrap_or_default() -> 验证得到 CloudEvent::default()
    #[test]
    fn test_invalid_json_cloud_event_default() {
        let invalid_json = "this is not valid json {{{";
        
        let cloud_event: CloudEvent = serde_json::from_str(invalid_json).unwrap_or_default();
        let default_event = CloudEvent::default();
        
        assert_eq!(cloud_event.id, default_event.id);
        assert_eq!(cloud_event.topic, default_event.topic);
        assert_eq!(cloud_event.data, default_event.data);
        println!("无效 JSON 返回默认 CloudEvent: {:?}", cloud_event);
    }

    /// 测试 CloudEvent::default() 的 event_type 解析后为空
    #[test]
    fn test_default_cloud_event_type_is_empty() {
        use genies_ddd::message::MessageImpl;
        
        // CloudEvent::default() 的 data 是默认 Value
        let default_cloud_event = CloudEvent::default();
        
        // 尝试从 default data 解析 MessageImpl，会失败返回 default
        let message_impl: MessageImpl = serde_json::from_value(default_cloud_event.data).unwrap_or_default();
        let header_event_type = message_impl.headers.event_type.unwrap_or_default();
        
        // 空字符串不应匹配任何已订阅事件
        assert!(header_event_type.is_empty(), "默认 CloudEvent 的 event_type 应为空");
        println!("默认 CloudEvent 的 event_type='{}'", header_event_type);
    }

    /// 测试无效 payload 导致 panic（非 JSON 格式）
    #[test]
    #[should_panic]
    fn test_invalid_payload_causes_panic() {
        use serde::Deserialize;
        
        #[derive(Debug, Default, Deserialize)]
        struct TestEvent {
            pub id: Option<String>,
            pub name: Option<String>,
        }
        
        // 使用无效的 JSON 格式字符串，确保解析失败
        let _: TestEvent = serde_json::from_str("not valid json {{{").unwrap();
    }
}
