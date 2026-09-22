//! 可测试的 Wasm 宿主门面（非导出；导出见 crate 根 `extern "C"`）。

use spark_asset::AssetCache;
use spark_types::Vec2;

/// Wasm / 单测共用的轻量宿主：资产缓存 + 几何探针。
///
/// 不拥有窗口或帧循环；真正的 C ABI 入口在 crate 根。
#[derive(Debug, Default)]
pub struct SparkWasmHost {
    /// 进程内资产字节缓存。
    pub assets: AssetCache,
}

impl SparkWasmHost {
    /// 空宿主（默认资产缓存）。
    pub fn new() -> Self {
        Self::default()
    }

    /// 计算二维向量长度（与导出 `spark_vec2_length` 同语义，便于纯 Rust 测试）。
    pub fn vec2_length(&self, x: f64, y: f64) -> f64 {
        Vec2::new(x as f32, y as f32).length() as f64
    }
}
