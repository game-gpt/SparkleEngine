//! 纹理句柄与上传队列类型。
//!
//! 权威上传对象为 [`spark_texture::TextureUpload`]。
//! [`RgbaImage`] 仅保留为 RGBA8 特例的薄封装，便于旧调用迁移。

use std::sync::atomic::{AtomicU32, Ordering};

use spark_core::{ErrorArg, SparkError, codes};
use spark_texture::TextureUpload;

/// 不透明纹理 ID。进程内单调分配，跨帧可缓存。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct TextureId(pub u32);

static NEXT_TEXTURE_ID: AtomicU32 = AtomicU32::new(1);

/// 分配稳定 [`TextureId`]（勿每帧重复分配同一逻辑贴图）。
pub fn alloc_texture_id() -> TextureId {
    TextureId(NEXT_TEXTURE_ID.fetch_add(1, Ordering::Relaxed))
}

/// RGBA8 行主序像素块（`TextureUpload` 的特例视图，非权威模型）。
#[derive(Debug, Clone)]
pub struct RgbaImage {
    pub width: u32,
    pub height: u32,
    pub rgba: Vec<u8>,
}

impl RgbaImage {
    /// 从原始 RGBA8 构造并校验长度。
    pub fn from_rgba8(width: u32, height: u32, rgba: Vec<u8>) -> Result<Self, SparkError> {
        let need = (width as usize).checked_mul(height as usize).and_then(|n| n.checked_mul(4)).ok_or_else(|| {
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
        Ok(Self { width, height, rgba })
    }

    /// 转为 sRGB [`TextureUpload`]（默认 CPU 生成 mip）。
    pub fn into_upload(self) -> Result<TextureUpload, SparkError> {
        TextureUpload::rgba8_srgb(self.width, self.height, self.rgba)
    }

    /// 借用转为 sRGB [`TextureUpload`]。
    pub fn to_upload(&self) -> Result<TextureUpload, SparkError> {
        TextureUpload::rgba8_srgb(self.width, self.height, self.rgba.clone())
    }
}
