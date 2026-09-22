//! PNG → [`TextureUpload`]。
//!
//! **不**暴露 CPU `PixelImage`，**不**依赖 `wgpu` / `spark-renderer`。
//! 仅解码 PNG，不含 JPEG / WebP。

#![warn(missing_docs)]

use std::{io::Cursor, path::Path, sync::Arc};

use spark_core::{ErrorArg, SparkError, codes};
use spark_texture::TextureUpload;

/// PNG 解码选项。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct DecodeOptions {
    /// `true` → `Rgba8UnormSrgb`；`false` → `Rgba8Unorm`。
    pub srgb: bool,
}

impl DecodeOptions {
    /// sRGB（漫反射 / UI 常见）。
    pub const fn srgb() -> Self {
        Self { srgb: true }
    }

    /// 线性（数据图等）。
    pub const fn linear() -> Self {
        Self { srgb: false }
    }
}

/// 从路径读 PNG 并解码为 [`TextureUpload`]。
pub fn decode_path(path: impl AsRef<Path>, options: DecodeOptions) -> Result<TextureUpload, SparkError> {
    let path = path.as_ref();
    let path_arg = ErrorArg::Path(Arc::from(path.to_string_lossy().as_ref()));
    let bytes = std::fs::read(path).map_err(|e| {
        SparkError::new(codes::io()).arg("path", path_arg.clone()).arg("op", ErrorArg::String(Arc::from("read"))).caused_by(e)
    })?;
    decode_memory(&bytes, options).map_err(|e| e.arg("path", path_arg))
}

/// 从内存 PNG 字节解码为 [`TextureUpload`]。
pub fn decode_memory(bytes: &[u8], options: DecodeOptions) -> Result<TextureUpload, SparkError> {
    let decoder = image::codecs::png::PngDecoder::new(Cursor::new(bytes)).map_err(|e| {
        SparkError::new(codes::image_decode())
            .arg("format", ErrorArg::String(Arc::from("png")))
            .arg("op", ErrorArg::String(Arc::from("open")))
            .arg("bytes", ErrorArg::Unsigned(bytes.len() as u64))
            .caused_by(e)
    })?;
    let dyn_img = image::DynamicImage::from_decoder(decoder).map_err(|e| {
        SparkError::new(codes::image_decode())
            .arg("format", ErrorArg::String(Arc::from("png")))
            .arg("op", ErrorArg::String(Arc::from("decode")))
            .arg("bytes", ErrorArg::Unsigned(bytes.len() as u64))
            .caused_by(e)
    })?;
    let rgba = dyn_img.to_rgba8();
    let (w, h) = rgba.dimensions();
    TextureUpload::rgba8(w, h, rgba.into_raw(), options.srgb)
}
