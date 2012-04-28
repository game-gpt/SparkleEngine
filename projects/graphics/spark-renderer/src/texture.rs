//! CPU 侧纹理句柄与待上传像素（无游戏资源格式）。

use std::sync::atomic::{AtomicU32, Ordering};

use spark_core::SparkError;

/// 不透明纹理 ID。进程内单调分配，跨帧可缓存。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct TextureId(pub u32);

static NEXT_TEXTURE_ID: AtomicU32 = AtomicU32::new(1);

/// 分配稳定 [`TextureId`]（勿每帧重复分配同一逻辑贴图）。
pub fn alloc_texture_id() -> TextureId {
    TextureId(NEXT_TEXTURE_ID.fetch_add(1, Ordering::Relaxed))
}

/// RGBA8 行主序像素块。
#[derive(Debug, Clone)]
pub struct RgbaImage {
    pub width: u32,
    pub height: u32,
    pub rgba: Vec<u8>,
}

impl RgbaImage {
    pub fn from_rgba8(width: u32, height: u32, rgba: Vec<u8>) -> Result<Self, SparkError> {
        let need = (width as usize)
            .checked_mul(height as usize)
            .and_then(|n| n.checked_mul(4))
            .ok_or_else(|| SparkError::Message("纹理尺寸溢出".into()))?;
        if rgba.len() != need {
            return Err(SparkError::Message(format!(
                "RGBA 长度不符：期望 {need}，得到 {}",
                rgba.len()
            )));
        }
        if width == 0 || height == 0 {
            return Err(SparkError::Message("纹理宽高须为正".into()));
        }
        Ok(Self {
            width,
            height,
            rgba,
        })
    }
}
