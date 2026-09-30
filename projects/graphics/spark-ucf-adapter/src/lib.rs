//! 可选 UCF 计算 / 后处理适配层。
//!
//! **边界（强制）：**
//! - 不依赖 `spark-ecs` / `SparkRuntime` 权威世界。
//! - 不实现 `WindowPump2d`，不改窗口泵。
//! - `spark-renderer` / `spark-renderer-wgpu` 不硬依赖本 crate；由调用方 opt-in。
//! - 无 UCF 时可用 [`fill_rgba8_cpu_fallback`] 保持原路径。

#![forbid(missing_docs)]

#[cfg(feature = "cpu")]
mod executor;

#[cfg(feature = "wgpu")]
mod wgpu_target;

#[cfg(feature = "renderer")]
mod drawlist;

#[cfg(feature = "cpu")]
pub use executor::{SparkComputePass, SparkUcfExecutor};

#[cfg(feature = "wgpu")]
pub use wgpu_target::WgpuRgba8Target;

#[cfg(feature = "renderer")]
pub use drawlist::enqueue_color_fill_texture;

/// 无 UCF 时的纯色 RGBA8 填充（软件 fallback）。
pub fn fill_rgba8_cpu_fallback(width: u32, height: u32, rgba: [u8; 4]) -> Vec<u8> {
    let n = (width as usize).saturating_mul(height as usize);
    let mut out = Vec::with_capacity(n.saturating_mul(4));
    for _ in 0..n {
        out.extend_from_slice(&rgba);
    }
    out
}
