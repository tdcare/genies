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
