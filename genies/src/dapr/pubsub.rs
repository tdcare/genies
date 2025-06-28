/*
 * @Author: tzw
 * @Date: 2021-11-02 20:20:15
 * @LastEditors: tzw
 * @LastEditTime: 2021-11-03 21:25:32
 */

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct DaprTopicSubscription {
    #[serde(rename = "pubsubName")]
    pub pubsub_name: Option<String>,
    pub topic: Option<String>,
    pub route: Option<String>,
    pub routes: Option<DaprRoute>,
    pub metadata: Option<HashMap<String, String>>,
}
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Rule{
    pub r#match:Option<String>,
    pub path:Option<String>,
}


#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct DaprRoute{
    pub rules:Option<Vec<Rule>>,
    pub default:Option<String>,
}
