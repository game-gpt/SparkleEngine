//! WebP → [`TextureUpload`]。
//!
//! 使用 image-rs 生态的 pure Rust [`image_webp`]，**不**依赖 umbrella `image`，
//! **不**依赖 libwebp / `*-sys`。有 alpha 时输出 RGBA8；否则 RGB 扩成 RGBA（alpha=255）。
//!
//! 错误：`io`、`image_decode`、`image_dimension_overflow`、`texture_data_length_mismatch`。

#![forbid(missing_docs)]

use std::{io::Cursor, path::Path, sync::Arc};

use image_webp::WebPDecoder;
use spark_texture::TextureUpload;
use spark_types::{ErrorArg, SparkError, codes};

/// WebP 解码选项：控制上传包的色彩空间标注与 GPU 格式。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct DecodeOptions {
    /// `true` → [`spark_texture::TextureFormat::Rgba8UnormSrgb`]；
    /// `false` → [`spark_texture::TextureFormat::Rgba8Unorm`]。
    ///
    /// 默认 `false`。与 WebP 色彩配置文件无关——由调用方显式选择。
    pub srgb: bool,
}

impl DecodeOptions {
    /// 标注为 sRGB（漫反射 / UI 常见）。
    pub const fn srgb() -> Self {
        Self { srgb: true }
    }

    /// 标注为线性（数据图等）。
    pub const fn linear() -> Self {
        Self { srgb: false }
    }
}

/// 从路径读 WebP 并解码为 [`TextureUpload`]（RGBA8，单 mip，默认 CPU 生成 mip）。
///
/// 读盘失败 → `io`；解码失败透传 [`decode_memory`] 并附加 `path`。
pub fn decode_path(path: impl AsRef<Path>, options: DecodeOptions) -> Result<TextureUpload, SparkError> {
    let path = path.as_ref();
    let path_arg = ErrorArg::Path(Arc::from(path.to_string_lossy().as_ref()));
    let bytes = std::fs::read(path)
        .map_err(|e| SparkError::new(codes::io()).arg("path", path_arg.clone()).arg("op", ErrorArg::String(Arc::from("read"))).caused_by(e))?;
    decode_memory(&bytes, options).map_err(|e| e.arg("path", path_arg))
}

/// 从内存 WebP 字节解码为 [`TextureUpload`]。
///
/// 宽高单位像素；字节为行主序 RGBA8。`options.srgb` 决定格式与 `color_space`。
pub fn decode_memory(bytes: &[u8], options: DecodeOptions) -> Result<TextureUpload, SparkError> {
    let (w, h, rgba) = decode_rgba8(bytes)?;
    TextureUpload::rgba8(w, h, rgba, options.srgb)
}

/// 解码为 RGBA8 原始字节（行主序，长度 = `width * height * 4`）。
///
/// 有 alpha：直接使用解码缓冲；无 alpha：按 RGB 扩成 RGBA（alpha=255）。
/// 打开 / 读帧失败 → `image_decode`；缓冲尺寸异常 → 溢出或长度不匹配。
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
    }
    else {
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
