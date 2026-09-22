//! 源图片格式解码 → [`TextureUpload`]。
//!
//! 本 crate 是 `image` 等解码器的 formats 适配层，**不**暴露 CPU `PixelImage`，
//! **不**依赖 `wgpu` / `spark-renderer`。

#![warn(missing_docs)]

use std::{path::Path, sync::Arc};

use image::ImageReader;
use spark_core::{ErrorArg, SparkError, codes};
use spark_texture::TextureUpload;

/// 解码选项。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct DecodeOptions {
    /// `true` → `Rgba8UnormSrgb`；`false` → `Rgba8Unorm`。
    pub srgb: bool,
}

impl DecodeOptions {
    /// 默认：sRGB + 后端可 CPU 生成 mip。
    pub const fn srgb() -> Self {
        Self { srgb: true }
    }

    /// 线性 RGBA8（法线 / 数据图等）。
    pub const fn linear() -> Self {
        Self { srgb: false }
    }
}

/// 从文件路径解码为 [`TextureUpload`]（PNG / JPEG / WebP 等，由 `image` 猜格式）。
pub fn decode_path(path: impl AsRef<Path>, options: DecodeOptions) -> Result<TextureUpload, SparkError> {
    let path = path.as_ref();
    let path_arg = ErrorArg::Path(Arc::from(path.to_string_lossy().as_ref()));
    let reader = ImageReader::open(path)
        .map_err(|e| {
            SparkError::new(codes::io()).arg("path", path_arg.clone()).arg("op", ErrorArg::String(Arc::from("open"))).caused_by(e)
        })?
        .with_guessed_format()
        .map_err(|e| {
            SparkError::new(codes::image_decode())
                .arg("path", path_arg.clone())
                .arg("op", ErrorArg::String(Arc::from("guess_format")))
                .caused_by(e)
        })?;
    let dyn_img = reader.decode().map_err(|e| {
        SparkError::new(codes::image_decode()).arg("path", path_arg).arg("op", ErrorArg::String(Arc::from("decode"))).caused_by(e)
    })?;
    rgba_dyn_to_upload(dyn_img, options)
}

/// 从内存字节解码为 [`TextureUpload`]。
pub fn decode_memory(bytes: &[u8], options: DecodeOptions) -> Result<TextureUpload, SparkError> {
    let dyn_img = image::load_from_memory(bytes).map_err(|e| {
        SparkError::new(codes::image_decode())
            .arg("op", ErrorArg::String(Arc::from("decode_memory")))
            .arg("bytes", ErrorArg::Unsigned(bytes.len() as u64))
            .caused_by(e)
    })?;
    rgba_dyn_to_upload(dyn_img, options)
}

fn rgba_dyn_to_upload(dyn_img: image::DynamicImage, options: DecodeOptions) -> Result<TextureUpload, SparkError> {
    let rgba = dyn_img.to_rgba8();
    let (w, h) = rgba.dimensions();
    TextureUpload::rgba8(w, h, rgba.into_raw(), options.srgb)
}
