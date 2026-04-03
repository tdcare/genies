use base64ct::{Base64, Encoding};
use log::*;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use rbatis::RBatis;
use std::time::Duration;
use crate::error::*;
use fastdate::DateTime;
use crate::message::{Headers, Message, MessageImpl};

/// 创建带超时的 HTTP 客户端
fn build_http_client() -> Result<reqwest::Client> {
    reqwest::Client::builder()
        .timeout(Duration::from_secs(30))
        .build()
        .map_err(|e| Error::DaprApi(e.to_string()))
}

pub async fn dapr(
    rb: RBatis,
    addr: String,
    dapr_pubsub_name: String,
    dapr_pub_message_limit: i64,
    clear_message_before_second: i64,
) -> Result<()> {
    let publish_url = format!("{}/v1.0/publish/{}", addr, dapr_pubsub_name);
    debug!("publish_url: {}", publish_url);
    let mut conn = rb.acquire().await?;
    let client = build_http_client()?;

    // Phase 1: Query unpublished messages
    let message_vec: Vec<Message> =
        Message::select_no_published_message(&mut conn, dapr_pub_message_limit as u64).await?;

    // Phase 2: Publish each message to Dapr and confirm immediately
    for v in message_vec {
        let msg_id = v.id.clone().ok_or(Error::General("message id is none".into()))?;
        let headers_string = v.headers.as_ref().ok_or(Error::General("headers is none".into()))?;
        let headers_map: Headers = serde_json::from_str(headers_string)?;
        let topic_name = headers_map.DESTINATION.clone().ok_or(Error::General("destination is none".into()))?;
        let aggregate_type = headers_map.event_aggregate_type.clone().ok_or(Error::General("aggregate_type is none".into()))?;
        let aggregate_id = headers_map.event_aggregate_id.clone().ok_or(Error::General("aggregate_id is none".into()))?;

        // Compute partition key
        let key_string = format!("{}{}", aggregate_type, aggregate_id);
        let hash = Sha256::new().chain_update(key_string).finalize();
        let key_base64_hash = Base64::encode_string(&hash);
        debug!("partition_key_hash: {}", key_base64_hash);

        let payload = v.payload.as_ref().ok_or(Error::General("payload is none".into()))?;
        let message_impl = MessageImpl::new(headers_map, payload.to_string());
        let topic_url = format!("{}/{}", publish_url, topic_name);

        #[derive(Serialize, Deserialize)]
        struct MetaData {
            #[serde(rename = "metadata.partitionKey")]
            partition_key: String,
        }
        let metadata = MetaData {
            partition_key: key_base64_hash,
        };

        // Call Dapr publish API with timeout
        let response = client
            .post(&topic_url)
            .query(&metadata)
            .json(&message_impl)
            .send()
            .await
            .map_err(|e| Error::DaprApi(format!("Failed to publish to {}: {}", topic_url, e)))?;

        let status = response.status();
        if status == reqwest::StatusCode::OK || status == reqwest::StatusCode::NO_CONTENT {
            // Immediately mark this message as published
            let m = Message {
                id: None,
                creation_time: None,
                destination: None,
                headers: None,
                payload: None,
                published: Some(1),
            };
            Message::update_single_message_published(&mut conn, &m, msg_id).await?;
            debug!("Successfully published message to topic: {}", topic_name);
        } else {
            warn!("Dapr publish returned unexpected status {}: topic={}", status, topic_name);
        }
    }

    // Phase 3: Clean up expired messages in batches
    let before = DateTime::now().unix_timestamp_millis() - clear_message_before_second * 1000;
    let batch_size: u64 = 10000;
    loop {
        let rows_affected = Message::delete_old_message_batch(&mut conn, before as u64, batch_size)
            .await?
            .rows_affected;
        debug!("Deleted {} expired messages", rows_affected);
        if rows_affected < batch_size {
            break;
        }
        tokio::time::sleep(Duration::from_millis(100)).await;
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_partition_key() {
        use sha2::{Digest, Sha256};
        use base64ct::{Base64, Encoding};
        let key = format!("{}{}", "OrderAggregate", "order-123");
        let hash = Sha256::new().chain_update(key).finalize();
        let encoded = Base64::encode_string(&hash);
        assert!(!encoded.is_empty());
    }
}
