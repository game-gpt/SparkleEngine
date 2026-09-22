//! PNG → [`TextureUpload`]（及 RGBA8 编码）。
//!
//! 使用 image-rs 生态的 pure Rust [`png`] crate，**不**依赖 umbrella `image`，
//! **不**依赖 `*-sys` / FFI。解码经 `normalize_to_color8` 后扩成 RGBA8。
//!
//! 解码错误：`io`、`image_decode`、`image_dimension_overflow`、`texture_data_length_mismatch`。
//! 编码错误：`image_rgba_length_mismatch`、`image_encode`、`io`。

#![deny(missing_docs)]

use std::{io::Cursor, path::Path, sync::Arc};

use spark_types::{ErrorArg, SparkError, codes};
use spark_texture::TextureUpload;

/// PNG 解码选项：控制上传包的色彩空间标注与 GPU 格式。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct DecodeOptions {
    /// `true` → [`spark_texture::TextureFormat::Rgba8UnormSrgb`]；
    /// `false` → [`spark_texture::TextureFormat::Rgba8Unorm`]。
    ///
    /// 默认 `false`。与文件内 sRGB chunk 无关——由调用方显式选择。
    pub srgb: bool,
}

impl DecodeOptions {
    /// 标注为 sRGB（漫反射 / UI 常见）。
    pub const fn srgb() -> Self {
        Self { srgb: true }
    }

    /// 标注为线性（数据图、遮罩等）。
    pub const fn linear() -> Self {
        Self { srgb: false }
    }
}

/// 从路径读 PNG 并解码为 [`TextureUpload`]（RGBA8，单 mip，默认 CPU 生成 mip）。
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

/// 从内存 PNG 字节解码为 [`TextureUpload`]。
///
/// 宽高单位像素；字节为行主序 RGBA8。`options.srgb` 决定格式与 `color_space`。
pub fn decode_memory(bytes: &[u8], options: DecodeOptions) -> Result<TextureUpload, SparkError> {
    let (w, h, rgba) = decode_rgba8(bytes)?;
    TextureUpload::rgba8(w, h, rgba, options.srgb)
}

/// 解码为 RGBA8 原始字节（行主序，长度 = `width * height * 4`）。
///
/// 支持输出色型：`Rgba` / `Rgb`（补 alpha=255）/ `Grayscale` / `GrayscaleAlpha`；
/// 其余 → `image_decode`（`unsupported_color`）。
pub fn decode_rgba8(bytes: &[u8]) -> Result<(u32, u32, Vec<u8>), SparkError> {
    let mut decoder = png::Decoder::new(Cursor::new(bytes));
    decoder.set_transformations(png::Transformations::normalize_to_color8());
    let mut reader = decoder.read_info().map_err(|e| {
        SparkError::new(codes::image_decode())
            .arg("format", ErrorArg::String(Arc::from("png")))
            .arg("op", ErrorArg::String(Arc::from("read_info")))
            .arg("bytes", ErrorArg::Unsigned(bytes.len() as u64))
            .caused_by(e)
    })?;
    let info = reader.info();
    let width = info.width;
    let height = info.height;
    let color = reader.output_color_type().0;
    let buf_len = reader.output_buffer_size().ok_or_else(|| {
        SparkError::new(codes::image_dimension_overflow())
            .arg("width", ErrorArg::Unsigned(width as u64))
            .arg("height", ErrorArg::Unsigned(height as u64))
    })?;
    let mut buf = vec![0u8; buf_len];
    let frame = reader.next_frame(&mut buf).map_err(|e| {
        SparkError::new(codes::image_decode())
            .arg("format", ErrorArg::String(Arc::from("png")))
            .arg("op", ErrorArg::String(Arc::from("next_frame")))
            .caused_by(e)
    })?;
    let raw = &buf[..frame.buffer_size()];
    let rgba = expand_to_rgba8(color, raw, width, height)?;
    Ok((width, height, rgba))
}

/// 将 RGBA8（行主序，长度须为 `width * height * 4`）编码为 PNG 字节。
///
/// 长度不符 → `image_rgba_length_mismatch`；尺寸溢出 → `image_dimension_overflow`；
/// 写头 / 写像素失败 → `image_encode`。
pub fn encode_rgba8(width: u32, height: u32, rgba: &[u8]) -> Result<Vec<u8>, SparkError> {
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
    let mut out = Vec::new();
    {
        let mut encoder = png::Encoder::new(&mut out, width, height);
        encoder.set_color(png::ColorType::Rgba);
        encoder.set_depth(png::BitDepth::Eight);
        let mut writer = encoder.write_header().map_err(|e| {
            SparkError::new(codes::image_encode())
                .arg("format", ErrorArg::String(Arc::from("png")))
                .arg("op", ErrorArg::String(Arc::from("write_header")))
                .caused_by(e)
        })?;
        writer.write_image_data(rgba).map_err(|e| {
            SparkError::new(codes::image_encode())
                .arg("format", ErrorArg::String(Arc::from("png")))
                .arg("op", ErrorArg::String(Arc::from("write_image_data")))
                .caused_by(e)
        })?;
    }
    Ok(out)
}

/// 将 RGBA8 编码并写入 PNG 文件。
///
/// 编码失败透传 [`encode_rgba8`]；写盘失败 → `io`（`op=write`）。
pub fn encode_path(path: impl AsRef<Path>, width: u32, height: u32, rgba: &[u8]) -> Result<(), SparkError> {
    let path = path.as_ref();
    let path_arg = ErrorArg::Path(Arc::from(path.to_string_lossy().as_ref()));
    let bytes = encode_rgba8(width, height, rgba)?;
    std::fs::write(path, bytes).map_err(|e| {
        SparkError::new(codes::io()).arg("path", path_arg).arg("op", ErrorArg::String(Arc::from("write"))).caused_by(e)
    })
}

fn expand_to_rgba8(color: png::ColorType, raw: &[u8], width: u32, height: u32) -> Result<Vec<u8>, SparkError> {
    let pixels = (width as usize).checked_mul(height as usize).ok_or_else(|| {
        SparkError::new(codes::image_dimension_overflow())
            .arg("width", ErrorArg::Unsigned(width as u64))
            .arg("height", ErrorArg::Unsigned(height as u64))
    })?;
    match color {
        png::ColorType::Rgba => {
            if raw.len() != pixels * 4 {
                return Err(SparkError::new(codes::texture_data_length_mismatch())
                    .arg("expected", ErrorArg::Unsigned((pixels * 4) as u64))
                    .arg("got", ErrorArg::Unsigned(raw.len() as u64)));
            }
            Ok(raw.to_vec())
        }
        png::ColorType::Rgb => {
            if raw.len() != pixels * 3 {
                return Err(SparkError::new(codes::texture_data_length_mismatch())
                    .arg("expected", ErrorArg::Unsigned((pixels * 3) as u64))
                    .arg("got", ErrorArg::Unsigned(raw.len() as u64)));
            }
            let mut out = Vec::with_capacity(pixels * 4);
            for chunk in raw.chunks_exact(3) {
                out.extend_from_slice(&[chunk[0], chunk[1], chunk[2], 255]);
            }
            Ok(out)
        }
        png::ColorType::Grayscale => {
            if raw.len() != pixels {
                return Err(SparkError::new(codes::texture_data_length_mismatch())
                    .arg("expected", ErrorArg::Unsigned(pixels as u64))
                    .arg("got", ErrorArg::Unsigned(raw.len() as u64)));
            }
            let mut out = Vec::with_capacity(pixels * 4);
            for &g in raw {
                out.extend_from_slice(&[g, g, g, 255]);
            }
            Ok(out)
        }
        png::ColorType::GrayscaleAlpha => {
            if raw.len() != pixels * 2 {
                return Err(SparkError::new(codes::texture_data_length_mismatch())
                    .arg("expected", ErrorArg::Unsigned((pixels * 2) as u64))
                    .arg("got", ErrorArg::Unsigned(raw.len() as u64)));
            }
            let mut out = Vec::with_capacity(pixels * 4);
            for chunk in raw.chunks_exact(2) {
                out.extend_from_slice(&[chunk[0], chunk[0], chunk[0], chunk[1]]);
            }
            Ok(out)
        }
        other => Err(SparkError::new(codes::image_decode())
            .arg("format", ErrorArg::String(Arc::from("png")))
            .arg("reason", ErrorArg::String(Arc::from(format!("unsupported_color:{other:?}")))),
        ),
    }
}
