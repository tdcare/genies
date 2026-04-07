//! Genies 测试基础设施
//!
//! 提供 HTTP 对比测试、数据库快照/差异/还原、Deep Diff 等通用测试工具。

pub mod config;
pub mod db;
pub mod diff;
pub mod assertions;
pub mod mutation;

// Re-export 所有公共 API，方便使用者直接 `use genies_test::*;`
pub use config::*;
pub use db::*;
pub use diff::*;
pub use assertions::*;
pub use mutation::*;
