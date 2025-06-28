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
