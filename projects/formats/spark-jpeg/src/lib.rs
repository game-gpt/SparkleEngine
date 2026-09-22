//! JPEG → [`TextureUpload`]。
//!
//! 使用 image-rs 生态的 pure Rust [`jpeg_decoder`]，**不**依赖 umbrella `image`，
//! **不**依赖 libjpeg / `*-sys`。解码结果扩成 RGBA8（不透明 alpha = 255）。
//!
//! 支持像素格式：`RGB24`、`L8`；其余 → `image_decode`（`unsupported_pixel_format`）。
//! 另可能返回 `io`、`image_dimension_overflow`、`texture_data_length_mismatch`。

#![deny(missing_docs)]

use std::{io::Cursor, path::Path, sync::Arc};

use jpeg_decoder::{Decoder, PixelFormat};
use spark_types::{ErrorArg, SparkError, codes};
use spark_texture::TextureUpload;

/// JPEG 解码选项：控制上传包的色彩空间标注与 GPU 格式。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct DecodeOptions {
    /// `true` → [`spark_texture::TextureFormat::Rgba8UnormSrgb`]；
    /// `false` → [`spark_texture::TextureFormat::Rgba8Unorm`]。
    ///
    /// 默认 `false`（线性）。JPEG 本身无 alpha；像素按伽马源解释由调用方决定。
    pub srgb: bool,
}

impl DecodeOptions {
    /// 标注为 sRGB（漫反射 / UI 常见）。
    pub const fn srgb() -> Self {
        Self { srgb: true }
    }

    /// 标注为线性（数据图、法线等非显示色）。
    pub const fn linear() -> Self {
        Self { srgb: false }
    }
}

/// 从路径读 JPEG 并解码为 [`TextureUpload`]（RGBA8，单 mip，默认 CPU 生成 mip）。
///
/// 读盘失败 → `io`；解码失败透传 [`decode_memory`] 并附加 `path`。
pub fn decode_path(path: impl AsRef<Path>, options: DecodeOptions) -> Result<TextureUpload, SparkError> {
    let path = path.as_ref();
    let path_arg = ErrorArg::Path(Arc::from(path.to_string_lossy().as_ref()));
    let bytes = std::fs::read(path).map_err(|e| {
        SparkError::new(codes::io()).arg("path", path_arg.clone()).arg("op", ErrorArg::String(Arc::from("read"))).caused_by(e)
    })?;
    decode_memory(&bytes, options).map_err(|e| e.arg("path", path_arg))
}

/// 从内存 JPEG 字节解码为 [`TextureUpload`]。
///
/// 宽高单位像素；字节为行主序 RGBA8。`options.srgb` 决定格式与 `color_space`。
pub fn decode_memory(bytes: &[u8], options: DecodeOptions) -> Result<TextureUpload, SparkError> {
    let (w, h, rgba) = decode_rgba8(bytes)?;
    TextureUpload::rgba8(w, h, rgba, options.srgb)
}

/// 解码为 RGBA8 原始字节（行主序，长度 = `width * height * 4`）。
///
/// - `RGB24`：每像素补 alpha=255
/// - `L8`：灰度复制到 RGB，alpha=255
///
/// 失败：`image_decode`（解析 / 缺 info / 不支持格式）、尺寸溢出、载荷长度不匹配。
pub fn decode_rgba8(bytes: &[u8]) -> Result<(u32, u32, Vec<u8>), SparkError> {
    let mut decoder = Decoder::new(Cursor::new(bytes));
    let pixels = decoder.decode().map_err(|e| {
        SparkError::new(codes::image_decode())
            .arg("format", ErrorArg::String(Arc::from("jpeg")))
            .arg("op", ErrorArg::String(Arc::from("decode")))
            .arg("bytes", ErrorArg::Unsigned(bytes.len() as u64))
            .caused_by(e)
    })?;
    let info = decoder.info().ok_or_else(|| {
        SparkError::new(codes::image_decode())
            .arg("format", ErrorArg::String(Arc::from("jpeg")))
            .arg("reason", ErrorArg::String(Arc::from("missing_info")))
    })?;
    let width = info.width as u32;
    let height = info.height as u32;
    let rgba = match info.pixel_format {
        PixelFormat::RGB24 => {
            let n = (width as usize).checked_mul(height as usize).ok_or_else(|| {
                SparkError::new(codes::image_dimension_overflow())
                    .arg("width", ErrorArg::Unsigned(width as u64))
                    .arg("height", ErrorArg::Unsigned(height as u64))
            })?;
            if pixels.len() != n * 3 {
                return Err(SparkError::new(codes::texture_data_length_mismatch())
                    .arg("expected", ErrorArg::Unsigned((n * 3) as u64))
                    .arg("got", ErrorArg::Unsigned(pixels.len() as u64)));
            }
            let mut out = Vec::with_capacity(n * 4);
            for chunk in pixels.chunks_exact(3) {
                out.extend_from_slice(&[chunk[0], chunk[1], chunk[2], 255]);
            }
            out
        }
        PixelFormat::L8 => {
            let n = (width as usize).checked_mul(height as usize).ok_or_else(|| {
                SparkError::new(codes::image_dimension_overflow())
                    .arg("width", ErrorArg::Unsigned(width as u64))
                    .arg("height", ErrorArg::Unsigned(height as u64))
            })?;
            if pixels.len() != n {
                return Err(SparkError::new(codes::texture_data_length_mismatch())
                    .arg("expected", ErrorArg::Unsigned(n as u64))
                    .arg("got", ErrorArg::Unsigned(pixels.len() as u64)));
            }
            let mut out = Vec::with_capacity(n * 4);
            for &g in &pixels {
                out.extend_from_slice(&[g, g, g, 255]);
            }
            out
        }
        other => {
            return Err(SparkError::new(codes::image_decode())
                .arg("format", ErrorArg::String(Arc::from("jpeg")))
                .arg(
                    "reason",
                    ErrorArg::String(Arc::from(format!("unsupported_pixel_format:{other:?}"))),
                ));
        }
    };
    Ok((width, height, rgba))
}
