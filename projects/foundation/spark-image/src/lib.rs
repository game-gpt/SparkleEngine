//! 像素图装载（过渡）、精灵裁切与九宫格布局。
//!
//! **几何**（[`Sprite`] / [`NineSlice`]）只依赖纹理宽高，不依赖 CPU 像素缓冲。
//! **解码**走 formats 分仓（`spark-png` / `spark-jpeg` / `spark-webp`），**不**依赖 umbrella
//! `image` crate。目标管线是 formats → `spark-texture` 的 `TextureUpload`。本 crate **不**碰 GPU。

#![warn(missing_docs)]
mod nine;
mod sprite;

pub use nine::{Margin, NineQuad, NineSlice, NineSliceMode};
pub use sprite::{Sprite, SpriteSheet};

use std::{path::Path, sync::Arc};

use spark_core::{Color, ErrorArg, SparkError, codes};

pub use spark_core::Rect;

/// CPU 侧 RGBA8 像素图（行主序，每像素 4 字节）。过渡类型，新代码优先 `TextureUpload`。
#[derive(Debug, Clone)]
pub struct PixelImage {
    width: u32,
    height: u32,
    rgba: Vec<u8>,
}

impl PixelImage {
    /// 从原始 RGBA8 构造；长度须为 `width * height * 4`。
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
        Ok(Self { width, height, rgba })
    }

    /// 纯色图（测试 / 占位）。
    pub fn solid(width: u32, height: u32, color: Color) -> Result<Self, SparkError> {
        let n = (width as usize).checked_mul(height as usize).ok_or_else(|| {
            SparkError::new(codes::image_dimension_overflow())
                .arg("width", ErrorArg::Unsigned(width as u64))
                .arg("height", ErrorArg::Unsigned(height as u64))
        })?;
        let r = (color.r.clamp(0.0, 1.0) * 255.0) as u8;
        let g = (color.g.clamp(0.0, 1.0) * 255.0) as u8;
        let b = (color.b.clamp(0.0, 1.0) * 255.0) as u8;
        let a = (color.a.clamp(0.0, 1.0) * 255.0) as u8;
        let mut rgba = Vec::with_capacity(n * 4);
        for _ in 0..n {
            rgba.extend_from_slice(&[r, g, b, a]);
        }
        Self::from_rgba8(width, height, rgba)
    }

    /// 从文件解码（按扩展名分发到 `spark-png` / `spark-jpeg` / `spark-webp`）。
    pub fn load(path: impl AsRef<Path>) -> Result<Self, SparkError> {
        let path = path.as_ref();
        let path_arg = ErrorArg::Path(Arc::from(path.to_string_lossy().as_ref()));
        let bytes = std::fs::read(path).map_err(|e| {
            SparkError::new(codes::io()).arg("path", path_arg.clone()).arg("op", ErrorArg::String(Arc::from("read"))).caused_by(e)
        })?;
        Self::load_from_memory(&bytes).map_err(|e| e.arg("path", path_arg))
    }

    /// 从内存字节解码（按魔数识别 PNG / JPEG / WebP）。
    pub fn load_from_memory(bytes: &[u8]) -> Result<Self, SparkError> {
        let (w, h, rgba) = decode_rgba8_auto(bytes)?;
        Self::from_rgba8(w, h, rgba)
    }

    pub fn width(&self) -> u32 {
        self.width
    }

    pub fn height(&self) -> u32 {
        self.height
    }

    pub fn rgba(&self) -> &[u8] {
        &self.rgba
    }

    pub fn into_rgba(self) -> Vec<u8> {
        self.rgba
    }

    /// 写成 PNG（`spark-png` / pure Rust `png`）。
    pub fn save_png(&self, path: impl AsRef<Path>) -> Result<(), SparkError> {
        spark_png::encode_path(path, self.width, self.height, &self.rgba)
    }

    /// 整图作为源矩形（像素坐标，原点左上）。
    pub fn bounds(&self) -> Rect {
        Rect::new(0.0, 0.0, self.width as f32, self.height as f32)
    }

    /// 将像素矩形转为归一化 UV（`[0,1]`，V 向下与像素一致）。
    pub fn uv_rect(&self, region: Rect) -> Result<Rect, SparkError> {
        validate_region(self.width, self.height, region)?;
        let w = self.width as f32;
        let h = self.height as f32;
        Ok(Rect::new(region.x / w, region.y / h, region.w / w, region.h / h))
    }

    /// 采样单像素（越界报错）。
    pub fn pixel(&self, x: u32, y: u32) -> Result<[u8; 4], SparkError> {
        if x >= self.width || y >= self.height {
            return Err(SparkError::new(codes::image_pixel_out_of_bounds())
                .arg("x", ErrorArg::Unsigned(x as u64))
                .arg("y", ErrorArg::Unsigned(y as u64))
                .arg("width", ErrorArg::Unsigned(self.width as u64))
                .arg("height", ErrorArg::Unsigned(self.height as u64)));
        }
        let i = ((y * self.width + x) * 4) as usize;
        Ok([self.rgba[i], self.rgba[i + 1], self.rgba[i + 2], self.rgba[i + 3]])
    }
}

fn decode_rgba8_auto(bytes: &[u8]) -> Result<(u32, u32, Vec<u8>), SparkError> {
    if bytes.starts_with(&[0x89, b'P', b'N', b'G', b'\r', b'\n', 0x1A, b'\n']) {
        return spark_png::decode_rgba8(bytes);
    }
    if bytes.starts_with(&[0xFF, 0xD8, 0xFF]) {
        return spark_jpeg::decode_rgba8(bytes);
    }
    if bytes.len() >= 12 && &bytes[0..4] == b"RIFF" && &bytes[8..12] == b"WEBP" {
        return spark_webp::decode_rgba8(bytes);
    }
    Err(SparkError::new(codes::image_decode())
        .arg("op", ErrorArg::String(Arc::from("detect_format")))
        .arg("bytes", ErrorArg::Unsigned(bytes.len() as u64))
        .arg("reason", ErrorArg::String(Arc::from("unsupported_or_unknown_format"))))
}

pub(crate) fn validate_region(img_w: u32, img_h: u32, region: Rect) -> Result<(), SparkError> {
    if region.w <= 0.0 || region.h <= 0.0 {
        return Err(SparkError::new(codes::image_region_invalid())
            .arg("w", ErrorArg::Float(region.w as f64))
            .arg("h", ErrorArg::Float(region.h as f64)));
    }
    if region.x < 0.0 || region.y < 0.0 || region.x + region.w > img_w as f32 + 1e-3 || region.y + region.h > img_h as f32 + 1e-3 {
        return Err(SparkError::new(codes::image_region_out_of_bounds())
            .arg("x", ErrorArg::Float(region.x as f64))
            .arg("y", ErrorArg::Float(region.y as f64))
            .arg("w", ErrorArg::Float(region.w as f64))
            .arg("h", ErrorArg::Float(region.h as f64))
            .arg("img_w", ErrorArg::Unsigned(img_w as u64))
            .arg("img_h", ErrorArg::Unsigned(img_h as u64)));
    }
    Ok(())
}
