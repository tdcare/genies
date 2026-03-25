use crate::cache_service::ICacheService;
use async_trait::async_trait;
use genies_core::Result;
use std::collections::hash_map::RandomState;
use std::collections::HashMap;
use std::ops::Sub;
use std::sync::Mutex;
use std::time::{Duration, Instant};

///内存缓存服务
pub struct MemService {
    pub cache: Mutex<HashMap<String, (String, Option<(Instant, Duration)>), RandomState>>,
}

impl MemService {
    pub fn recycling(&self) {
        if let Ok(mut map_lock_guard) = self.cache.lock() {
            let mut need_removed = vec![];
            for (k, v) in map_lock_guard.iter() {
                match v.1 {
                    None => {}
                    Some((i, d)) => {
                        if i.elapsed() >= d {
                            //out of time
                            need_removed.push(k.to_string());
                        }
                    }
                }
            }
            for x in need_removed {
                map_lock_guard.remove(&x);
            }
        }
    }
}

impl Default for MemService {
    fn default() -> Self {
        Self {
            cache: Default::default(),
        }
    }
}

// impl<T> std::convert::From<PoisonError<T>> for Error {
//     fn from(arg: PoisonError<T>) -> Self {
//         Error::E(arg.to_string())
//     }
// }

#[async_trait]
impl ICacheService for MemService {
    async fn set_string(&self, k: &str, v: &str) -> Result<String> {
        self.recycling();
        let mut guard = self.cache.lock()?;
        guard.insert(k.to_string(), (v.to_string(), None));
        return Ok(v.to_string());
    }
    async fn get_string(&self, k: &str) -> Result<String> {
        self.recycling();
        let guard = self.cache.lock()?;
        let v = guard.get(k);
        match v {
            Some((v, _)) => {
                return Ok(v.to_string());
            }
            _ => {
                return Ok("".to_string());
            }
        }
    }
    async fn del_string(&self, k: &str) -> Result<String> {
        self.recycling();
        let mut guard = self.cache.lock()?;
        let v = guard.remove(k);
        match v {
            Some((v, _)) => {
                return Ok(v.to_string());
            }
            _ => {
                return Ok("".to_string());
            }
        }
    }
    async fn set_string_ex(&self, k: &str, v: &str, t: Option<Duration>) -> Result<String> {
        self.recycling();
        let mut locked = self.cache.lock()?;
        let mut e = Option::None;
        if let Some(ex) = t {
            e = Some((Instant::now(), ex));
        }
        let inserted = locked.insert(k.to_string(), (v.to_string(), e));
        if inserted.is_some() {
            return Ok(v.to_string());
        }
        return Result::Err(genies_core::error::Error::E(format!(
            "[mem_service]insert fail!"
        )));
    }

    /// 原子操作：仅当 key 不存在时设置值并设置过期时间
    /// 返回 true 表示设置成功（key 原本不存在），false 表示设置失败（key 已存在）
    async fn set_string_ex_nx(&self, k: &str, v: &str, t: Option<Duration>) -> Result<bool> {
        self.recycling();
        let mut locked = self.cache.lock()?;
        // 在锁内完成 check-and-set 原子操作
        if locked.contains_key(k) {
            // key 已存在，返回 false
            return Ok(false);
        }
        // key 不存在，设置值
        let e = t.map(|ex| (Instant::now(), ex));
        locked.insert(k.to_string(), (v.to_string(), e));
        Ok(true)
    }
    async fn set_value(&self, _k: &str, _v: &[u8]) -> Result<String> {
        todo!();
    }
    async fn get_value(&self, _k: &str) -> Result<Vec<u8>> {
        todo!();
    }
    async fn set_value_ex(&self, _k: &str, _v: &[u8], _t: Option<Duration>) -> Result<String> {
        todo!();
    }
    async fn ttl(&self, k: &str) -> Result<i64> {
        self.recycling();
        let locked = self.cache.lock()?;
        let v = locked.get(k).cloned();
        drop(locked);
        return match v {
            None => Ok(-2),
            Some((_r, o)) => match o {
                None => Ok(-1),
                Some((i, d)) => {
                    let use_time = i.elapsed();
                    if d > use_time {
                        return Ok(d.sub(use_time).as_secs() as i64);
                    }
                    Ok(0)
                }
            },
        };
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;
    use std::time::Duration;
    use tokio::time::sleep;

    /// 测试1: set_string_ex_nx 首次设置成功返回 true
    #[tokio::test]
    async fn test_nx_first_set_returns_true() {
        let service = MemService::default();
        let result = service
            .set_string_ex_nx("key1", "value1", Some(Duration::from_secs(60)))
            .await
            .unwrap();
        assert!(result, "首次 NX 设置应返回 true");
    }

    /// 测试2: key 已存在时 set_string_ex_nx 返回 false（NX 语义）
    #[tokio::test]
    async fn test_nx_existing_key_returns_false() {
        let service = MemService::default();
        // 首次设置
        let first = service
            .set_string_ex_nx("key2", "value1", Some(Duration::from_secs(60)))
            .await
            .unwrap();
        assert!(first, "首次设置应成功");

        // 再次设置同一个 key
        let second = service
            .set_string_ex_nx("key2", "value2", Some(Duration::from_secs(60)))
            .await
            .unwrap();
        assert!(!second, "key 已存在时 NX 应返回 false");
    }

    /// 测试3: NX 设置后 get_string 能读到正确值
    #[tokio::test]
    async fn test_nx_then_get_returns_correct_value() {
        let service = MemService::default();
        let _ = service
            .set_string_ex_nx("key3", "expected_value", Some(Duration::from_secs(60)))
            .await
            .unwrap();

        let value = service.get_string("key3").await.unwrap();
        assert_eq!(value, "expected_value", "get_string 应返回正确的值");
    }

    /// 测试4: 模拟完整幂等流程
    /// NX 抢锁("CONSUMING") -> 成功(true)
    /// set_string_ex("CONSUMED") 更新状态
    /// 再次 NX 返回 false
    /// get_string 返回 "CONSUMED"
    #[tokio::test]
    async fn test_full_idempotent_flow_success() {
        let service = MemService::default();
        let key = "idempotent_key_success";
        let ttl = Some(Duration::from_secs(300));

        // Step 1: NX 抢锁
        let lock_acquired = service
            .set_string_ex_nx(key, "CONSUMING", ttl)
            .await
            .unwrap();
        assert!(lock_acquired, "NX 抢锁应成功");

        // Step 2: 处理成功后更新状态为 CONSUMED
        let _ = service
            .set_string_ex(key, "CONSUMED", ttl)
            .await
            .unwrap();

        // Step 3: 再次 NX 应返回 false（key 已存在）
        let retry = service
            .set_string_ex_nx(key, "CONSUMING", ttl)
            .await
            .unwrap();
        assert!(!retry, "key 已存在，再次 NX 应返回 false");

        // Step 4: 验证状态为 CONSUMED
        let status = service.get_string(key).await.unwrap();
        assert_eq!(status, "CONSUMED", "状态应为 CONSUMED");
    }

    /// 测试5: 模拟处理失败流程
    /// NX 抢锁("CONSUMING") -> 成功(true)
    /// del_string 删除 key
    /// 再次 NX -> 成功(true)（可以重新消费）
    #[tokio::test]
    async fn test_idempotent_flow_handler_failure() {
        let service = MemService::default();
        let key = "idempotent_key_failure";
        let ttl = Some(Duration::from_secs(300));

        // Step 1: NX 抢锁
        let lock_acquired = service
            .set_string_ex_nx(key, "CONSUMING", ttl)
            .await
            .unwrap();
        assert!(lock_acquired, "NX 抢锁应成功");

        // Step 2: 处理失败，删除 key 以允许重试
        let _ = service.del_string(key).await.unwrap();

        // Step 3: 再次 NX 应成功（key 已删除）
        let retry = service
            .set_string_ex_nx(key, "CONSUMING", ttl)
            .await
            .unwrap();
        assert!(retry, "key 删除后，再次 NX 应成功");
    }

    /// 测试6: TTL 过期后 key 自动消失
    /// NX 设置 key，TTL 设为极短
    /// 等待过期
    /// 再次 NX 可以成功返回 true
    #[tokio::test]
    async fn test_nx_ttl_expiry() {
        let service = MemService::default();
        let key = "ttl_expiry_key";

        // 设置一个极短 TTL 的 key (50ms)
        let first = service
            .set_string_ex_nx(key, "value", Some(Duration::from_millis(50)))
            .await
            .unwrap();
        assert!(first, "首次 NX 应成功");

        // 等待 TTL 过期
        sleep(Duration::from_millis(100)).await;

        // 再次 NX 应成功（recycling 会清理过期 key）
        let after_expiry = service
            .set_string_ex_nx(key, "new_value", Some(Duration::from_secs(60)))
            .await
            .unwrap();
        assert!(after_expiry, "TTL 过期后，NX 应再次成功");
    }

    /// 测试7: 并发竞争测试
    /// 启动多个 tokio task 同时对同一 key 执行 NX
    /// 验证只有一个返回 true，其余返回 false
    #[tokio::test]
    async fn test_nx_concurrent_only_one_wins() {
        let service = Arc::new(MemService::default());
        let key = "concurrent_key";
        let num_tasks = 10;

        let mut handles = vec![];
        for _ in 0..num_tasks {
            let svc = Arc::clone(&service);
            let k = key.to_string();
            let handle = tokio::spawn(async move {
                svc.set_string_ex_nx(&k, "winner", Some(Duration::from_secs(60)))
                    .await
                    .unwrap()
            });
            handles.push(handle);
        }

        let mut results = vec![];
        for handle in handles {
            results.push(handle.await.unwrap());
        }

        let winners: Vec<_> = results.iter().filter(|&&r| r).collect();
        assert_eq!(
            winners.len(),
            1,
            "只有一个 task 应该返回 true，实际有 {} 个",
            winners.len()
        );

        let losers: Vec<_> = results.iter().filter(|&&r| !r).collect();
        assert_eq!(
            losers.len(),
            num_tasks - 1,
            "其余 {} 个 task 应返回 false",
            num_tasks - 1
        );
    }

    /// 测试8: 不带 TTL 的 NX 操作也正常工作
    #[tokio::test]
    async fn test_nx_without_ttl() {
        let service = MemService::default();
        let key = "no_ttl_key";

        // NX 设置不带 TTL
        let first = service
            .set_string_ex_nx(key, "value_no_ttl", None)
            .await
            .unwrap();
        assert!(first, "首次 NX（无 TTL）应成功");

        // 再次 NX 应失败
        let second = service
            .set_string_ex_nx(key, "another_value", None)
            .await
            .unwrap();
        assert!(!second, "key 已存在，NX 应返回 false");

        // 验证值正确
        let value = service.get_string(key).await.unwrap();
        assert_eq!(value, "value_no_ttl", "值应保持不变");
    }

    /// 测试9: 不同 key 的 NX 互不影响
    #[tokio::test]
    async fn test_nx_different_keys_independent() {
        let service = MemService::default();
        let ttl = Some(Duration::from_secs(60));

        // 设置多个不同的 key
        let key_a = service
            .set_string_ex_nx("key_a", "value_a", ttl)
            .await
            .unwrap();
        let key_b = service
            .set_string_ex_nx("key_b", "value_b", ttl)
            .await
            .unwrap();
        let key_c = service
            .set_string_ex_nx("key_c", "value_c", ttl)
            .await
            .unwrap();

        assert!(key_a, "key_a 首次 NX 应成功");
        assert!(key_b, "key_b 首次 NX 应成功");
        assert!(key_c, "key_c 首次 NX 应成功");

        // 验证各自的值
        assert_eq!(service.get_string("key_a").await.unwrap(), "value_a");
        assert_eq!(service.get_string("key_b").await.unwrap(), "value_b");
        assert_eq!(service.get_string("key_c").await.unwrap(), "value_c");

        // 再次对 key_a 执行 NX 应失败，不影响其他 key
        let key_a_retry = service
            .set_string_ex_nx("key_a", "new_value", ttl)
            .await
            .unwrap();
        assert!(!key_a_retry, "key_a 已存在，NX 应返回 false");

        // key_b 和 key_c 的值不受影响
        assert_eq!(service.get_string("key_b").await.unwrap(), "value_b");
        assert_eq!(service.get_string("key_c").await.unwrap(), "value_c");
    }
}
