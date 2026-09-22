//! Spark **Wasm** 绑定（`wasm32-unknown-unknown`）。
//!
//! 导出稳定 C ABI，供 `packages/spark-unknown-wasm32` 的 TypeScript 加载。
//!
//! ```text
//! rustup target add wasm32-unknown-unknown
//! cargo build -p spark-wasm --target wasm32-unknown-unknown --release
//! # 再运行 packages 内 copy 脚本，或：
//! # copy target/wasm32-unknown-unknown/release/spark_wasm.wasm
//! #   → packages/spark-unknown-wasm32/spark_engine_bg.wasm
//! ```

#![forbid(missing_docs)]
mod host;

pub use host::SparkWasmHost;

use spark_types::Vec2;

/// npm 平台包名（与 TS 包对齐）。
pub const NPM_PLATFORM_PACKAGE: &str = "spark-unknown-wasm32";

/// 计算二维向量长度（C ABI 探针，供 TS 冒烟测试）。
#[unsafe(no_mangle)]
pub extern "C" fn spark_vec2_length(x: f64, y: f64) -> f64 {
    Vec2::new(x as f32, y as f32).length() as f64
}

/// 版本编码：`major * 1_000_000 + minor * 1_000 + patch`（当前 0.0.0 → 0）。
#[unsafe(no_mangle)]
pub extern "C" fn spark_version_code() -> u32 {
    let v = env!("CARGO_PKG_VERSION");
    parse_version_code(v)
}

fn parse_version_code(v: &str) -> u32 {
    let mut parts = v.split('.');
    let major: u32 = parts.next().and_then(|s| s.parse().ok()).unwrap_or(0);
    let minor: u32 = parts.next().and_then(|s| s.parse().ok()).unwrap_or(0);
    let patch: u32 = parts.next().and_then(|s| s.parse().ok()).unwrap_or(0);
    major.saturating_mul(1_000_000) + minor.saturating_mul(1_000) + patch
}
