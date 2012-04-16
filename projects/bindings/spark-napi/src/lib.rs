//! Spark **Node-API** 绑定入口。
//!
//! 默认仅暴露可测试的 Rust 宿主门面 [`SparkJsHost`]，便于 `cargo test`。
//! 启用 feature `node` 后编译 napi 导出，配合 `package.json` 经 npm 派发。
//!
//! ```text
//! cargo build -p spark-napi --release --features node
//! ```

mod host;

#[cfg(feature = "node")]
mod node;

pub use host::{EngineInfo, SparkJsHost};

/// npm 侧约定的包名（与 `package.json` / `package.metadata.napi` 对齐）。
pub const NPM_PACKAGE_NAME: &str = "spark-engine";
