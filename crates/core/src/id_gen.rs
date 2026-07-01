//! 统一雪花 ID 生成器（48-bit 缩短版）
//!
//! 启动时由 `ApplicationContext` 自动初始化 worker_id，
//! 业务代码通过 `next_id()` 获取全局唯一 ID。
//!
//! ## 位布局 (48 bits)
//! - 32 bits: 秒级时间戳（以 2024-01-01 为起点）→ 可用约 136 年
//! - 10 bits: worker_id → 最多 1024 个 worker
//! -  6 bits: 序列号 → 每秒每个 worker 最多 64 个 ID
//!
//! 生成的 ID 为 13~15 位十进制数字（标准 64-bit 雪花算法为 18~19 位，缩短约 25%~30%）

use std::sync::{Mutex, OnceLock};
use std::time::{SystemTime, UNIX_EPOCH};

/// 自定义起始时间：2024-01-01 00:00:00 UTC
const EPOCH_SECONDS: u64 = 1_704_067_200;

const WORKER_ID_BITS: u32 = 10;
const SEQUENCE_BITS: u32 = 6;
const TIMESTAMP_BITS: u32 = 32;

const MAX_WORKER_ID: u32 = (1 << WORKER_ID_BITS) - 1;       // 1023
const MAX_SEQUENCE: u32 = (1 << SEQUENCE_BITS) - 1;          // 63
const MAX_TIMESTAMP: u64 = (1u64 << TIMESTAMP_BITS) - 1;     // ~136 years

const WORKER_ID_SHIFT: u32 = SEQUENCE_BITS;                   // 6
const TIMESTAMP_SHIFT: u32 = SEQUENCE_BITS + WORKER_ID_BITS;  // 16

struct IdGenerator {
    worker_id: u32,
    sequence: u32,
    last_timestamp: u64,
}

static ID_GEN: OnceLock<Mutex<IdGenerator>> = OnceLock::new();

/// 初始化 ID 生成器（启动时由 ApplicationContext 调用一次）
///
/// - `machine_id`: 机器ID（0..1023），直接作为 worker_id 使用
/// - `datacenter_id`: 数据中心ID（保留参数，当前未参与位运算）
pub fn init(machine_id: i32, datacenter_id: i32) {
    let worker_id = (machine_id as u32) & MAX_WORKER_ID;
    log::info!(
        "48-bit Snowflake ID generator initialized: machine_id={}, datacenter_id={}, worker_id={}",
        machine_id,
        datacenter_id,
        worker_id
    );
    ID_GEN.get_or_init(|| {
        Mutex::new(IdGenerator {
            worker_id,
            sequence: 0,
            last_timestamp: 0,
        })
    });
}

/// 生成全局唯一 ID（13~15 位十进制数字字符串）
pub fn next_id() -> String {
    ID_GEN.get()
        .expect("ID generator not initialized, call genies_core::id_gen::init() first")
        .lock()
        .unwrap()
        .next_id()
        .to_string()
}

impl IdGenerator {
    fn next_id(&mut self) -> u64 {
        let mut ts = self.now_seconds();

        if ts > MAX_TIMESTAMP {
            panic!(
                "Snowflake timestamp overflow: {} exceeds max {} (epoch 2024-01-01)",
                ts, MAX_TIMESTAMP
            );
        }

        if ts < self.last_timestamp {
            log::warn!(
                "Clock moved backwards: {} -> {}, waiting to catch up...",
                self.last_timestamp,
                ts
            );
            ts = self.wait_next_second(self.last_timestamp);
        }

        if ts == self.last_timestamp {
            self.sequence = (self.sequence + 1) & MAX_SEQUENCE;
            if self.sequence == 0 {
                // 当前秒序列号耗尽，等待下一秒
                ts = self.wait_next_second(self.last_timestamp);
            }
        } else {
            self.sequence = 0;
        }

        self.last_timestamp = ts;

        (ts << TIMESTAMP_SHIFT)
            | ((self.worker_id as u64) << WORKER_ID_SHIFT)
            | (self.sequence as u64)
    }

    fn now_seconds(&self) -> u64 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs()
            .saturating_sub(EPOCH_SECONDS)
    }

    fn wait_next_second(&self, last_ts: u64) -> u64 {
        loop {
            let now = self.now_seconds();
            if now > last_ts {
                return now;
            }
            std::thread::sleep(std::time::Duration::from_millis(100));
        }
    }
}
