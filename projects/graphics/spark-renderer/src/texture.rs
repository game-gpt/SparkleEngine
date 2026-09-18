//! CPU 侧纹理句柄与待上传像素（无游戏资源格式）。

use std::sync::atomic::{AtomicU32, Ordering};

use spark_core::{ErrorArg, SparkError, codes};

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
            .ok_or_else(|| {
                SparkError::new(codes::image_dimension_overflow())
                    .arg("width", ErrorArg::Unsigned(width as u64))
                    .arg("height", ErrorArg::Unsigned(height as u64))
            })?;
        if rgba.len() != need {
            return Err(SparkError::new(codes::image_rgba_length_mismatch())
                .arg("expected", ErrorArg::Unsigned(need as u64))
                .arg("got", ErrorArg::Unsigned(rgba.len() as u64)));
        }
        if width == 0 || height == 0 {
            return Err(SparkError::new(codes::texture_size_invalid())
                .arg("width", ErrorArg::Unsigned(width as u64))
                .arg("height", ErrorArg::Unsigned(height as u64)));
        }
        Ok(Self {
            width,
            height,
            rgba,
        })
    }
}
