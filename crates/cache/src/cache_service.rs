use std::time::Duration;

/*
 * @Author: tzw
 * @Date: 2021-10-17 21:43:48
 * @LastEditors: tzw
 * @LastEditTime: 2021-12-22 00:16:26
 */
use genies_config::app_config::ApplicationConfig;

use async_trait::async_trait;
use genies_core::Result;
// use prost::Message;
use serde::de::DeserializeOwned;
use serde::Serialize;

use crate::redis_service::RedisService;
use crate::mem_service::MemService;
use genies_core::error::Error;

#[async_trait]
pub trait ICacheService: Sync + Send {
    async fn set_string(&self, k: &str, v: &str) -> Result<String>;
    async fn get_string(&self, k: &str) -> Result<String>;
    async fn del_string(&self, k: &str) -> Result<String>;
    async fn set_string_ex(&self, k: &str, v: &str, ex: Option<Duration>) -> Result<String>;
    async fn set_value(&self, k: &str, v: &[u8]) -> Result<String>;
    async fn get_value(&self, k: &str) -> Result<Vec<u8>>;
    async fn set_value_ex(&self, k: &str, v: &[u8], ex: Option<Duration>) -> Result<String>;
    async fn ttl(&self, k: &str) -> Result<i64>;
}

pub struct CacheService {
    pub inner: Box<dyn ICacheService>,
}

impl CacheService {
    pub fn new(cfg: &ApplicationConfig) -> Self {
        Self {
            inner: match cfg.cache_type.as_str() {
                "redis" => Box::new(RedisService::new(&cfg.redis_url)),
                //"mem"
                _ => Box::new(MemService::default()),
            },
        }
    }
    pub fn new_saved(cfg: &ApplicationConfig) -> Self {
        Self {
            inner: match cfg.cache_type.as_str() {
                "redis" => Box::new(RedisService::new(&cfg.redis_save_url)),
                //"mem"
                _ => Box::new(MemService::default()),
            },
        }
    }
    pub async fn set_string(&self, k: &str, v: &str) -> Result<String> {
        self.inner.set_string(k, v).await
    }

    pub async fn get_string(&self, k: &str) -> Result<String> {
        self.inner.get_string(k).await
    }
    pub async fn del_string(&self, k: &str) -> Result<String> {
        self.inner.del_string(k).await
    }

    pub async fn set_json<T>(&self, k: &str, v: &T) -> Result<String>
    where
        T: Serialize + Sync,
    {
        let data = serde_json::to_string(v);
        if data.is_err() {
            return Err(Error::from(format!(
                "MemCacheService set_json fail:{}",
                data.err().unwrap()
            )));
        }
        let data = self.set_string(k, data.unwrap().as_str()).await?;
        Ok(data)
    }

    pub async fn get_json<T>(&self, k: &str) -> Result<T>
    where
        T: DeserializeOwned + Sync,
    {
        let mut r = self.get_string(k).await?;
        if r.is_empty() {
            r = "null".to_string();
        }
        let data: serde_json::Result<T> = serde_json::from_str(r.as_str());
        if data.is_err() {
            return Err(Error::from(format!(
                "MemCacheService GET fail:{}",
                data.err().unwrap()
            )));
        }
        Ok(data.unwrap())
    }

    pub async fn set_string_ex(&self, k: &str, v: &str, ex: Option<Duration>) -> Result<String> {
        self.inner.set_string_ex(k, v, ex).await
    }
    // pub async fn set_object_use_protobuf<T: Message + Default>(
    //     &self,
    //     k: &str,
    //     _v: &T,
    // ) -> Result<String> {
    //     let  bytes = Vec::new();
    //     // let t = v.encode(&mut bytes).unwrap();
    //     // let b=bytes;
    //     self.inner.set_value(k, &bytes).await
    // }
    //
    // pub async fn get_object_use_protobuf<T: Message + Default>(&self, k: &str) -> Result<T> {
    //     let r = self.inner.get_value(k).await?;
    //     let v: T = T::decode(&*r).unwrap();
    //     return Ok(v);
    // }
    //
    pub async fn ttl(&self, k: &str) -> Result<i64> {
        self.inner.ttl(k).await
    }
}

// fn some_function() -> Result<(), Error> {
//     // ... existing code ...
//     if some_condition {
//         return Err(Error::from(format!("Some error message")));
//     }
//     // ... existing code ...
// }
