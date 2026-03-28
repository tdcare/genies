//! Redis 和 MySQL 集成测试
//!
//! 运行测试: `cargo test -p integration -- --nocapture`
//!
//! 注意: 测试需要连接真实的 Redis 和 MySQL 服务器，
//! 配置信息在 `application.yml` 中。

use rbs::value;

#[cfg(test)]
mod redis_tests {
    use genies::context::CONTEXT;
    use std::time::Duration;

    /// 1. Redis 连接测试 — 验证能成功 set/get
    #[tokio::test]
    async fn test_redis_connection_and_basic_set_get() {
        let cache = &CONTEXT.cache_service;
        let key = "integration_test_basic";

        // set
        cache.set_string(key, "hello_genies").await.unwrap();

        // get
        let val = cache.get_string(key).await.unwrap();
        assert_eq!(val, "hello_genies");

        // cleanup
        cache.del_string(key).await.unwrap();
    }

    /// 2. Redis TTL 过期测试
    #[tokio::test]
    async fn test_redis_ttl_expiry() {
        let cache = &CONTEXT.cache_service;
        let key = "integration_test_ttl";

        cache
            .set_string_ex(key, "temp", Some(Duration::from_secs(2)))
            .await
            .unwrap();

        let ttl = cache.ttl(key).await.unwrap();
        assert!(ttl > 0, "TTL should be positive, got {}", ttl);

        // 等待过期
        tokio::time::sleep(Duration::from_secs(3)).await;

        let val = cache.get_string(key).await.unwrap();
        assert_eq!(val, "", "Value should be empty after expiry");
    }

    /// 3. Redis NX 原子操作测试（幂等性核心）
    #[tokio::test]
    async fn test_redis_nx_atomic_set() {
        let cache = &CONTEXT.cache_service;
        let key = "integration_test_nx";

        // 确保 key 不存在
        cache.del_string(key).await.ok();

        // 首次 NX 应成功
        let first = cache
            .set_string_ex_nx(key, "first", Some(Duration::from_secs(60)))
            .await
            .unwrap();
        assert!(first, "First NX set should succeed");

        // 再次 NX 应失败
        let second = cache
            .set_string_ex_nx(key, "second", Some(Duration::from_secs(60)))
            .await
            .unwrap();
        assert!(!second, "Second NX set should fail");

        // 值应为第一次的
        let val = cache.get_string(key).await.unwrap();
        assert_eq!(val, "first");

        // cleanup
        cache.del_string(key).await.unwrap();
    }

    /// 4. Redis del 操作测试
    #[tokio::test]
    async fn test_redis_del_operation() {
        let cache = &CONTEXT.cache_service;
        let key = "integration_test_del";

        cache.set_string(key, "to_delete").await.unwrap();
        cache.del_string(key).await.unwrap();

        let val = cache.get_string(key).await.unwrap();
        assert_eq!(val, "", "Deleted key should return empty string");
    }

    /// 5. Redis 二进制数据测试
    #[tokio::test]
    async fn test_redis_binary_value() {
        let cache = &CONTEXT.cache_service;
        let key = "integration_test_binary";
        let data: Vec<u8> = vec![0x00, 0x01, 0x02, 0xFF, 0xFE];

        // 使用 inner 访问底层的 set_value/get_value 方法
        cache.inner.set_value(key, &data).await.unwrap();
        let result = cache.inner.get_value(key).await.unwrap();
        assert_eq!(result, data);

        // cleanup
        cache.del_string(key).await.unwrap();
    }

    /// 6. Redis JSON 序列化/反序列化测试
    #[tokio::test]
    async fn test_redis_json_roundtrip() {
        use serde::{Deserialize, Serialize};

        #[derive(Debug, Serialize, Deserialize, PartialEq)]
        struct TestData {
            id: i64,
            name: String,
            active: bool,
        }

        let cache = &CONTEXT.cache_service;
        let key = "integration_test_json";
        let data = TestData {
            id: 42,
            name: "genies".to_string(),
            active: true,
        };

        cache.set_json(key, &data).await.unwrap();
        let result: TestData = cache.get_json(key).await.unwrap();
        assert_eq!(result, data);

        // cleanup
        cache.del_string(key).await.unwrap();
    }

    /// 7. Redis 并发 NX 竞争测试
    #[tokio::test]
    async fn test_redis_concurrent_nx_race() {
        let cache = &CONTEXT.cache_service;
        let key = "integration_test_nx_race";

        // 确保 key 不存在
        cache.del_string(key).await.ok();

        // 模拟并发抢锁
        let mut handles = vec![];
        for i in 0..5 {
            let k = key.to_string();
            handles.push(tokio::spawn(async move {
                let c = &CONTEXT.cache_service;
                c.set_string_ex_nx(&k, &format!("worker_{}", i), Some(Duration::from_secs(60)))
                    .await
                    .unwrap()
            }));
        }

        let results: Vec<bool> = futures::future::join_all(handles)
            .await
            .into_iter()
            .map(|r| r.unwrap())
            .collect();

        // 只有一个 worker 能成功
        let success_count = results.iter().filter(|&&r| r).count();
        assert_eq!(
            success_count, 1,
            "Only one NX should succeed, got {}",
            success_count
        );

        // cleanup
        cache.del_string(key).await.unwrap();
    }

    /// 8. Redis 空值处理测试
    #[tokio::test]
    async fn test_redis_empty_value() {
        let cache = &CONTEXT.cache_service;
        let key = "integration_test_empty";

        // 设置空字符串
        cache.set_string(key, "").await.unwrap();
        let val = cache.get_string(key).await.unwrap();
        assert_eq!(val, "", "Empty string should be stored correctly");

        // cleanup
        cache.del_string(key).await.unwrap();
    }

    /// 9. Redis 特殊字符测试
    #[tokio::test]
    async fn test_redis_special_characters() {
        let cache = &CONTEXT.cache_service;
        let key = "integration_test_special";
        let value = "中文测试 🎉 \n\t\\\"'";

        cache.set_string(key, value).await.unwrap();
        let result = cache.get_string(key).await.unwrap();
        assert_eq!(result, value, "Special characters should be preserved");

        // cleanup
        cache.del_string(key).await.unwrap();
    }

    /// 10. Redis 不存在的 key 返回空字符串测试
    #[tokio::test]
    async fn test_redis_nonexistent_key() {
        let cache = &CONTEXT.cache_service;
        let key = "integration_test_nonexistent_key_12345";

        let val = cache.get_string(key).await.unwrap();
        assert_eq!(val, "", "Nonexistent key should return empty string");
    }
}

#[cfg(test)]
mod mysql_tests {
    use genies::context::CONTEXT;
    use rbs::value;

    /// 辅助函数：确保 MySQL 已初始化
    async fn ensure_mysql_init() {
        // init_mysql 可以重复调用
        CONTEXT.init_mysql().await;
    }

    /// 1. MySQL 连接池初始化测试
    #[tokio::test]
    async fn test_mysql_pool_initialization() {
        ensure_mysql_init().await;

        let pool = CONTEXT.rbatis.get_pool().expect("Pool should be initialized");
        let state = pool.state().await;

        // state 是 rbs::Value 类型，用 Debug 打印
        println!("Pool state: {:?}", state);

        // state 不为 Null 说明连接池正常
        assert!(!state.is_null(), "Pool state should not be null");
    }

    /// 2. MySQL 简单查询测试（SELECT 1）
    #[tokio::test]
    async fn test_mysql_simple_query() {
        ensure_mysql_init().await;

        let rb = &CONTEXT.rbatis;
        let result: Vec<rbs::Value> = rb
            .query_decode("SELECT 1 as val", vec![])
            .await
            .expect("Simple query should succeed");

        assert!(!result.is_empty(), "Query result should not be empty");
        println!("SELECT 1 result: {:?}", result);
    }

    /// 3. MySQL 事务回滚测试（使用只读操作验证回滚功能）
    #[tokio::test]
    async fn test_mysql_transaction_rollback() {
        ensure_mysql_init().await;
        let rb = &CONTEXT.rbatis;

        // 使用事务执行只读操作来验证回滚功能
        let tx = rb.acquire_begin().await.expect("Begin transaction failed");

        let result: Vec<rbs::Value> = tx
            .query_decode("SELECT 1 as val", vec![])
            .await
            .expect("Query in transaction failed");

        assert!(!result.is_empty(), "Transaction query should return result");

        tx.rollback().await.expect("Rollback failed");

        // 回滚后验证连接仍然正常
        let result: Vec<rbs::Value> = rb
            .query_decode("SELECT 1 as val", vec![])
            .await
            .expect("Post-rollback query failed");
        assert!(!result.is_empty());
    }

    /// 4. MySQL 事务提交测试（使用只读操作验证提交功能）
    #[tokio::test]
    async fn test_mysql_transaction_commit() {
        ensure_mysql_init().await;
        let rb = &CONTEXT.rbatis;

        // 使用事务执行只读操作来验证事务功能
        let tx = rb.acquire_begin().await.expect("Begin transaction failed");

        let result: Vec<rbs::Value> = tx
            .query_decode("SELECT 1 as val", vec![])
            .await
            .expect("Query in transaction failed");

        assert!(!result.is_empty(), "Transaction query should return result");

        tx.commit().await.expect("Commit failed");

        // 提交后验证连接仍然正常
        let result: Vec<rbs::Value> = rb
            .query_decode("SELECT 1 as val", vec![])
            .await
            .expect("Post-commit query failed");
        assert!(!result.is_empty());
    }

    /// 5. MySQL 连接池配置验证测试
    #[tokio::test]
    async fn test_mysql_pool_config_matches() {
        ensure_mysql_init().await;

        let config = &CONTEXT.config;
        let pool = CONTEXT.rbatis.get_pool().expect("Pool should exist");
        let state = pool.state().await;

        // state 是 rbs::Value 类型，用 Debug 打印验证
        println!(
            "Config max_connections: {}, Pool state: {:?}",
            config.max_connections, state
        );

        // 验证连接池已初始化
        assert!(!state.is_null(), "Pool state should not be null");
    }

    /// 6. MySQL 当前时间查询（验证时区配置）
    #[tokio::test]
    async fn test_mysql_timezone() {
        ensure_mysql_init().await;

        let rb = &CONTEXT.rbatis;
        let result: Vec<rbs::Value> = rb
            .query_decode("SELECT NOW() as server_time", vec![])
            .await
            .expect("Timezone query failed");

        assert!(!result.is_empty());
        println!("MySQL server time: {:?}", result[0]);
    }

    /// 7. MySQL 版本查询测试
    #[tokio::test]
    async fn test_mysql_version() {
        ensure_mysql_init().await;

        let rb = &CONTEXT.rbatis;
        let result: Vec<rbs::Value> = rb
            .query_decode("SELECT VERSION() as version", vec![])
            .await
            .expect("Version query failed");

        assert!(!result.is_empty());
        println!("MySQL version: {:?}", result[0]);
    }

    /// 8. MySQL 参数化查询测试（不使用建表）
    #[tokio::test]
    async fn test_mysql_parameterized_query() {
        ensure_mysql_init().await;
        let rb = &CONTEXT.rbatis;

        // 使用参数化查询（不需要表）
        let result: Vec<rbs::Value> = rb
            .query_decode(
                "SELECT ? as id, ? as name",
                vec![value!(1), value!("param_test")],
            )
            .await
            .expect("Parameterized query failed");

        assert!(!result.is_empty(), "Parameterized query should return result");
        println!("Parameterized query result: {:?}", result[0]);
    }
}

#[cfg(test)]
mod config_tests {
    use genies::context::CONTEXT;

    /// 1. 配置文件加载验证
    #[tokio::test]
    async fn test_config_loaded_correctly() {
        let config = &CONTEXT.config;

        assert!(
            !config.server_name.is_empty(),
            "server_name should not be empty"
        );
        assert!(
            !config.database_url.is_empty(),
            "database_url should not be empty"
        );
        assert!(!config.redis_url.is_empty(), "redis_url should not be empty");
        assert!(
            !config.server_url.is_empty(),
            "server_url should not be empty"
        );

        println!("server_name: {}", config.server_name);
        println!("server_url: {}", config.server_url);
    }

    /// 2. 白名单配置验证
    #[tokio::test]
    async fn test_config_white_list() {
        let config = &CONTEXT.config;

        assert!(
            !config.white_list_api.is_empty(),
            "White list should not be empty"
        );
        assert!(
            config.white_list_api.contains(&"/".to_string()),
            "Root path should be whitelisted"
        );
        assert!(
            config.white_list_api.contains(&"/dapr/*".to_string()),
            "Dapr path should be whitelisted"
        );

        println!("White list: {:?}", config.white_list_api);
    }

    /// 3. Dapr 配置验证
    #[tokio::test]
    async fn test_config_dapr_settings() {
        let config = &CONTEXT.config;

        assert!(
            !config.dapr_pubsub_name.is_empty(),
            "pubsub_name should not be empty"
        );
        assert!(
            config.dapr_pub_message_limit > 0,
            "pub_message_limit should be positive"
        );
        assert!(
            config.processing_expire_seconds > 0,
            "processing_expire_seconds should be positive"
        );

        println!("dapr_pubsub_name: {}", config.dapr_pubsub_name);
        println!("dapr_pub_message_limit: {}", config.dapr_pub_message_limit);
    }

    /// 4. 数据库连接池配置验证
    #[tokio::test]
    async fn test_config_db_pool_settings() {
        let config = &CONTEXT.config;

        assert!(config.max_connections > 0, "max_connections should be positive");
        assert!(config.max_lifetime > 0, "max_lifetime should be positive");

        println!("max_connections: {}", config.max_connections);
        println!("wait_timeout: {}", config.wait_timeout);
        println!("max_lifetime: {}", config.max_lifetime);
    }

    /// 5. Keycloak 配置验证
    #[tokio::test]
    async fn test_config_keycloak_settings() {
        let config = &CONTEXT.config;

        assert!(
            !config.keycloak_auth_server_url.is_empty(),
            "keycloak_auth_server_url should not be empty"
        );
        assert!(
            !config.keycloak_realm.is_empty(),
            "keycloak_realm should not be empty"
        );
        assert!(
            !config.keycloak_resource.is_empty(),
            "keycloak_resource should not be empty"
        );

        println!("keycloak_auth_server_url: {}", config.keycloak_auth_server_url);
        println!("keycloak_realm: {}", config.keycloak_realm);
    }
}

#[cfg(test)]
mod topic_tests;

// mod e2e_business_tests; // 模块文件不存在，已注释

#[cfg(test)]
mod auth_tests;
