//! 可测试的 Wasm 宿主门面（非导出；导出见 crate 根 `extern "C"`）。

use spark_asset::AssetCache;
use spark_core::Vec2;
use spark_geometry::Vec2Ext;

#[derive(Debug, Default)]
pub struct SparkWasmHost {
    pub assets: AssetCache,
}

impl SparkWasmHost {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn vec2_length(&self, x: f64, y: f64) -> f64 {
        Vec2::new(x as f32, y as f32).length() as f64
    }
}
