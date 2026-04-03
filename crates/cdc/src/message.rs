/*
 * @Author: tzw
 * @Date: 2021-10-31 23:26:50
 * @LastEditors: tzw
 * @LastEditTime: 2022-01-18 09:30:14
 */
use rbatis::executor::Executor;
use rbatis::rbdc::db::ExecResult;
use rbatis::Error;
use rbs::Value;
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;
use std::collections::HashMap;


#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct Headers {
    pub PARTITION_ID: Option<String>,
    #[serde(rename = "event-aggregate-type")]
    pub event_aggregate_type: Option<String>,
    pub DATE: Option<String>,
    #[serde(rename = "event-aggregate-id")]
    pub event_aggregate_id: Option<String>,
    #[serde(rename = "event-type")]
    pub event_type: Option<String>,
    pub DESTINATION: Option<String>,
    pub ID: Option<String>,

    #[serde(flatten)]
    pub extra: HashMap<String, JsonValue>,
}

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct MessageImpl {
    pub payload: Option<String>,
    pub headers: Option<Headers>,
}

impl MessageImpl {
    pub fn new(headers: Headers, payload: String) -> Self {
        MessageImpl {
            payload: Some(payload),
            headers: Some(headers),
        }
    }
}

/// 保存到数据库的 message
// #[crud_table(table_name:"message")]
#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct Message {
    pub id: Option<String>,
    pub destination: Option<String>,
    pub headers: Option<String>,
    pub payload: Option<String>,
    pub published: Option<u32>,
    pub creation_time: Option<i64>,
}

rbatis::crud!(Message{});

impl Message {
    /// Select unpublished messages with limit
    pub async fn select_no_published_message(
        executor: &dyn Executor,
        dapr_pub_message_limit: u64,
    ) -> Result<Vec<Message>, Error> {
        let sql = format!(
            "SELECT id, destination, headers, payload, published, creation_time FROM message WHERE published = 0 LIMIT {}",
            dapr_pub_message_limit
        );
        let value = executor.query(&sql, vec![]).await?;
        rbatis::decode(value)
    }

    /// Update single message as published
    pub async fn update_single_message_published(
        executor: &dyn Executor,
        message: &Message,
        id: String,
    ) -> Result<ExecResult, Error> {
        let published = message.published.unwrap_or(1);
        let sql = "UPDATE message SET published = ? WHERE id = ?";
        executor.exec(sql, vec![
            Value::U32(published),
            Value::String(id),
        ]).await
    }

    /// Delete old messages in batch
    pub async fn delete_old_message_batch(
        executor: &dyn Executor,
        before: u64,
        limit: u64,
    ) -> Result<ExecResult, Error> {
        let sql = format!(
            "DELETE FROM message WHERE published = 1 AND creation_time < {} LIMIT {}",
            before, limit
        );
        executor.exec(&sql, vec![]).await
    }

    /// Delete old published messages
    #[allow(dead_code)]
    pub async fn delete_old_message(
        executor: &dyn Executor,
        before: u64,
    ) -> Result<ExecResult, Error> {
        let sql = "DELETE FROM message WHERE published = 1 AND creation_time < ?";
        executor.exec(sql, vec![Value::U64(before)]).await
    }

    /// Update multiple messages as published
    #[allow(dead_code)]
    pub async fn update_message_published(
        executor: &dyn Executor,
        message: &Message,
        ids: Vec<String>,
    ) -> Result<ExecResult, Error> {
        if ids.is_empty() {
            return Ok(ExecResult {
                rows_affected: 0,
                last_insert_id: Value::Null,
            });
        }
        let published = message.published.unwrap_or(1);
        let placeholders: Vec<&str> = ids.iter().map(|_| "?").collect();
        let sql = format!(
            "UPDATE message SET published = ? WHERE id IN ({})",
            placeholders.join(", ")
        );
        let mut args = vec![Value::U32(published)];
        for id in ids {
            args.push(Value::String(id));
        }
        executor.exec(&sql, args).await
    }
}
