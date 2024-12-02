/*
 * @Author: tzw
 * @Date: 2021-10-17 21:43:48
 * @LastEditors: tzw
 * @LastEditTime: 2021-12-22 02:05:34
 */
use std::time::Duration;

use log::error;
use redis::aio::Connection;

use crate::cache::ICacheService;
use async_trait::async_trait;
use crate::error::{Error};
use crate::Result;

use redis::RedisResult;
///Redis缓存服务
pub struct RedisService {
    pub client: redis::Client,
}

impl RedisService {
    pub fn new(url: &str) -> Self {
        log::info!("conncect redis:{}", &url);
        let client = redis::Client::open(url).unwrap();
        log::info!("conncect redis success!");
        Self { client }
    }

    pub async fn get_conn(&self) -> Result<Connection> {
        let conn = self.client.get_async_connection().await;
        if conn.is_err() {
            let err = format!("RedisService connect fail:{}", conn.err().unwrap());
            error!("{}", err);
            return Err(crate::error::Error::from(err));
        }
        return Ok(conn.unwrap());
    }
}

#[async_trait]
impl ICacheService for RedisService {
    async fn set_string(&self, k: &str, v: &str) -> Result<String> {
        return self.set_string_ex(k, v, None).await;
    }

    async fn get_string(&self, k: &str) -> Result<String> {
        let mut conn = self.get_conn().await?;
        let result: RedisResult<Option<String>> =
            redis::cmd("GET").arg(&[k]).query_async(&mut conn).await;
        match result {
            Ok(v) => {
                return Ok(v.unwrap_or(String::new()));
            }
            Err(e) => {
                return Err(Error::from(format!(
                    "RedisService get_string({}) fail:{}",
                    k,
                    e.to_string()
                )));
            }
        }
    }
    async fn del_string(&self, k: &str) -> Result<String> {
        let mut conn = self.get_conn().await?;
        let result: RedisResult<Option<String>> =
            redis::cmd("DEL").arg(&[k]).query_async(&mut conn).await;
        match result {
            Ok(v) => {
                return Ok(v.unwrap_or(String::new()));
            }
            Err(e) => {
                return Err(Error::from(format!(
                    "RedisService del_string({}) fail:{}",
                    k,
                    e.to_string()
                )));
            }
        }
    }
    ///set_string 自动过期
    async fn set_string_ex(&self, k: &str, v: &str, ex: Option<Duration>) -> Result<String> {
        let mut conn = self.get_conn().await?;
        if ex.is_none() {
            return match redis::cmd("SET").arg(&[k, v]).query_async(&mut conn).await {
                Ok(v) => Ok(v),
                Err(e) => Err(Error::from(format!(
                    "RedisService set_string_ex fail:{}",
                    e.to_string()
                ))),
            };
        } else {
            return match redis::cmd("SET")
                .arg(&[k, v, "EX", &ex.unwrap().as_secs().to_string()])
                .query_async(&mut conn)
                .await
            {
                Ok(v) => Ok(v),
                Err(e) => Err(Error::from(format!(
                    "RedisService set_string_ex fail:{}",
                    e.to_string()
                ))),
            };
        }
    }

    ///set_string 自动过期
    async fn ttl(&self, k: &str) -> Result<i64> {
        let mut conn = self.get_conn().await?;
        return match redis::cmd("TTL").arg(&[k]).query_async(&mut conn).await {
            Ok(v) => Ok(v),
            Err(e) => Err(Error::from(format!(
                "RedisService ttl fail:{}",
                e.to_string()
            ))),
        };
    }

    async fn get_value(&self, k: &str) -> Result<Vec<u8>> {
        let mut conn = self.get_conn().await?;
        let result: RedisResult<Vec<u8>> = redis::cmd("GET").arg(&[k]).query_async(&mut conn).await;
        match result {
            Ok(v) => {
                return Ok(v);
            }
            Err(e) => {
                return Err(Error::from(format!(
                    "RedisService get_value({}) fail:{}",
                    k,
                    e.to_string()
                )));
            }
        }
    }
    async fn set_value(&self, k: &str, v: &[u8]) -> Result<String> {
        return self.set_value_ex(k, v, None).await;
    }

    async fn set_value_ex(&self, k: &str, v: &[u8], ex: Option<Duration>) -> Result<String> {
        let mut conn = self.get_conn().await?;
        if ex.is_none() {
            return match redis::cmd("SET").arg(k).arg(v).query_async(&mut conn).await {
                Ok(v) => Ok(v),
                Err(e) => Err(Error::from(format!(
                    "RedisService set_value_ex fail:{}",
                    e.to_string()
                ))),
            };
        } else {
            return match redis::cmd("SET")
                .arg(k)
                .arg(v)
                .arg("EX")
                .arg(&ex.unwrap().as_secs().to_string())
                .query_async(&mut conn)
                .await
            {
                Ok(v) => Ok(v),
                Err(e) => Err(Error::from(format!(
                    "RedisService set_value_ex fail:{}",
                    e.to_string()
                ))),
            };
        }
    }
}
