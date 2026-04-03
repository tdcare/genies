use serde::{Deserialize, Serialize};


#[derive(Serialize, Deserialize,Clone, Debug)]
pub struct CdcConfig {
    pub service_name: Option<String>,
    pub database_url: Option<String>,
    // Dapr cdc 参数
    pub dapr_pubsub_name: Option<String>,
    // 每次投递消息的数量
    pub dapr_pub_message_limit: Option<i64>,
    // #time::Duration::from_millis(dapr_cdc_message_period);
    pub dapr_cdc_message_period: Option<i64>,

    pub clear_message_before_second:Option<i64>,
}
