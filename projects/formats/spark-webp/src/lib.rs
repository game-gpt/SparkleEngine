//! WebP → [`TextureUpload`]。
//!
//! 使用 image-rs 生态的 pure Rust [`image_webp`]，**不**依赖 umbrella `image`，
//! **不**依赖 libwebp / `*-sys`。

#![warn(missing_docs)]

use std::{io::Cursor, path::Path, sync::Arc};

use image_webp::WebPDecoder;
use spark_core::{ErrorArg, SparkError, codes};
use spark_texture::TextureUpload;

/// WebP 解码选项。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct DecodeOptions {
    /// `true` → `Rgba8UnormSrgb`；`false` → `Rgba8Unorm`。
    pub srgb: bool,
}

impl DecodeOptions {
    /// sRGB。
    pub const fn srgb() -> Self {
        Self { srgb: true }
    }

    /// 线性。
    pub const fn linear() -> Self {
        Self { srgb: false }
    }
}

/// 从路径读 WebP 并解码为 [`TextureUpload`]。
pub fn decode_path(path: impl AsRef<Path>, options: DecodeOptions) -> Result<TextureUpload, SparkError> {
    let path = path.as_ref();
    let path_arg = ErrorArg::Path(Arc::from(path.to_string_lossy().as_ref()));
    let bytes = std::fs::read(path).map_err(|e| {
        SparkError::new(codes::io()).arg("path", path_arg.clone()).arg("op", ErrorArg::String(Arc::from("read"))).caused_by(e)
    })?;
    decode_memory(&bytes, options).map_err(|e| e.arg("path", path_arg))
}

/// 从内存 WebP 字节解码为 [`TextureUpload`]。
pub fn decode_memory(bytes: &[u8], options: DecodeOptions) -> Result<TextureUpload, SparkError> {
    let (w, h, rgba) = decode_rgba8(bytes)?;
    TextureUpload::rgba8(w, h, rgba, options.srgb)
}

/// 解码为 RGBA8 原始字节（行主序）。
pub fn decode_rgba8(bytes: &[u8]) -> Result<(u32, u32, Vec<u8>), SparkError> {
    let mut decoder = WebPDecoder::new(Cursor::new(bytes)).map_err(|e| {
        SparkError::new(codes::image_decode())
            .arg("format", ErrorArg::String(Arc::from("webp")))
            .arg("op", ErrorArg::String(Arc::from("open")))
            .arg("bytes", ErrorArg::Unsigned(bytes.len() as u64))
            .caused_by(e)
    })?;
    let (width, height) = decoder.dimensions();
    let buf_len = decoder.output_buffer_size().ok_or_else(|| {
        SparkError::new(codes::image_dimension_overflow())
            .arg("width", ErrorArg::Unsigned(width as u64))
            .arg("height", ErrorArg::Unsigned(height as u64))
    })?;
    let mut buf = vec![0u8; buf_len];
    decoder.read_image(&mut buf).map_err(|e| {
        SparkError::new(codes::image_decode())
            .arg("format", ErrorArg::String(Arc::from("webp")))
            .arg("op", ErrorArg::String(Arc::from("read_image")))
            .caused_by(e)
    })?;
    let rgba = if decoder.has_alpha() {
        buf
    } else {
        let n = (width as usize).checked_mul(height as usize).ok_or_else(|| {
            SparkError::new(codes::image_dimension_overflow())
                .arg("width", ErrorArg::Unsigned(width as u64))
                .arg("height", ErrorArg::Unsigned(height as u64))
        })?;
        if buf.len() != n * 3 {
            return Err(SparkError::new(codes::texture_data_length_mismatch())
                .arg("expected", ErrorArg::Unsigned((n * 3) as u64))
                .arg("got", ErrorArg::Unsigned(buf.len() as u64)));
        }
        let mut out = Vec::with_capacity(n * 4);
        for chunk in buf.chunks_exact(3) {
            out.extend_from_slice(&[chunk[0], chunk[1], chunk[2], 255]);
        }
        out
    };
    Ok((width, height, rgba))
}
