/*
 * @Author: tzw
 * @Date: 2021-10-17 21:43:48
 * @LastEditors: tzw
 * @LastEditTime: 2021-12-22 02:05:34
 */
use std::time::Duration;

use log::error;
use redis::aio::MultiplexedConnection;
use tokio::sync::Mutex;

use crate::cache_service::ICacheService;
use async_trait::async_trait;
use genies_core::error::*;
use genies_core::Result;


use redis::RedisResult;
///Redis缓存服务
pub struct RedisService {
    pub client: redis::Client,
    conn: Mutex<Option<MultiplexedConnection>>,
}

impl RedisService {
    pub fn new(url: &str) -> Self {
        log::info!("conncect redis:{}", &url);
        let client = redis::Client::open(url).unwrap();
        log::info!("conncect redis success!");
        Self { client, conn: Mutex::new(None) }
    }

    /// 获取或创建共享的 MultiplexedConnection（复用连接，避免每次操作新建连接）
    pub async fn get_conn(&self) -> Result<MultiplexedConnection> {
        // 快速路径：直接复用已缓存的连接
        {
            let guard = self.conn.lock().await;
            if let Some(conn) = guard.as_ref() {
                return Ok(conn.clone());
            }
        }
        // 慢路径：创建新连接并缓存
        let new_conn = self.client.get_multiplexed_async_connection()
            .await
            .map_err(|e| {
                let err = format!("RedisService connect fail:{}", e);
                error!("{}", err);
                Error::from(err)
            })?;
        let mut guard = self.conn.lock().await;
        // 双重检查，避免竞态导致连接泄漏
        if let Some(conn) = guard.as_ref() {
            return Ok(conn.clone());
        }
        *guard = Some(new_conn.clone());
        Ok(new_conn)
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
        // DEL 命令返回 integer（删除的 key 数量），不是 string
        let result: RedisResult<i64> =
            redis::cmd("DEL").arg(&[k]).query_async(&mut conn).await;
        match result {
            Ok(deleted_count) => {
                // 返回删除的 key 数量作为字符串
                return Ok(deleted_count.to_string());
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

    /// 原子操作：仅当 key 不存在时设置值并设置过期时间 (SET key value NX EX seconds)
    /// 返回 true 表示设置成功（key 原本不存在），false 表示设置失败（key 已存在）
    async fn set_string_ex_nx(&self, k: &str, v: &str, ex: Option<Duration>) -> Result<bool> {
        let mut conn = self.get_conn().await?;
        let result: RedisResult<Option<String>> = if ex.is_none() {
            redis::cmd("SET")
                .arg(&[k, v, "NX"])
                .query_async(&mut conn)
                .await
        } else {
            redis::cmd("SET")
                .arg(&[k, v, "NX", "EX", &ex.unwrap().as_secs().to_string()])
                .query_async(&mut conn)
                .await
        };
        match result {
            Ok(v) => {
                // SET NX 返回 "OK" 表示设置成功，返回 nil 表示 key 已存在
                Ok(v.is_some())
            }
            Err(e) => Err(Error::from(format!(
                "RedisService set_string_ex_nx({}) fail:{}",
                k,
                e.to_string()
            ))),
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
