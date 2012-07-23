//! 像素图装载、精灵裁切与九宫格拉伸（CPU 侧）。
//!
//! 本 crate **不**碰 GPU：产出 RGBA 字节、源矩形与九宫格目标四边形，由 `spark-renderer-wgpu`
//! 等后端上传纹理并批绘制。
//! 上传纹理并批绘制。无游戏语义（无方块 / UI 皮肤产品名）。

mod nine;
mod sprite;

pub use nine::{Margin, NineQuad, NineSlice, NineSliceMode};
pub use sprite::{Sprite, SpriteSheet};

use std::path::Path;
use std::sync::Arc;

use image::ImageReader;
use spark_core::{Color, ErrorArg, Rect, SparkError, codes};

/// CPU 侧 RGBA8 像素图（行主序，每像素 4 字节）。
#[derive(Debug, Clone)]
pub struct PixelImage {
    width: u32,
    height: u32,
    rgba: Vec<u8>,
}

impl PixelImage {
    /// 从原始 RGBA8 构造；长度须为 `width * height * 4`。
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
        Ok(Self {
            width,
            height,
            rgba,
        })
    }

    /// 纯色图（测试 / 占位）。
    pub fn solid(width: u32, height: u32, color: Color) -> Result<Self, SparkError> {
        let n = (width as usize)
            .checked_mul(height as usize)
            .ok_or_else(|| {
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

    /// 从文件解码（PNG / JPEG / WebP 等，由 `image` 决定）。
    pub fn load(path: impl AsRef<Path>) -> Result<Self, SparkError> {
        let path = path.as_ref();
        let path_arg = ErrorArg::Path(Arc::from(path.to_string_lossy().as_ref()));
        let reader = ImageReader::open(path)
            .map_err(|e| {
                SparkError::new(codes::io())
                    .arg("path", path_arg.clone())
                    .arg("op", ErrorArg::String(Arc::from("open")))
                    .caused_by(e)
            })?
            .with_guessed_format()
            .map_err(|e| {
                SparkError::new(codes::image_decode())
                    .arg("path", path_arg.clone())
                    .arg("op", ErrorArg::String(Arc::from("guess_format")))
                    .caused_by(e)
            })?;
        let dyn_img = reader.decode().map_err(|e| {
            SparkError::new(codes::image_decode())
                .arg("path", path_arg)
                .arg("op", ErrorArg::String(Arc::from("decode")))
                .caused_by(e)
        })?;
        let rgba = dyn_img.to_rgba8();
        let (w, h) = rgba.dimensions();
        Self::from_rgba8(w, h, rgba.into_raw())
    }

    /// 从内存字节解码。
    pub fn load_from_memory(bytes: &[u8]) -> Result<Self, SparkError> {
        let dyn_img = image::load_from_memory(bytes).map_err(|e| {
            SparkError::new(codes::image_decode())
                .arg("op", ErrorArg::String(Arc::from("decode_memory")))
                .arg("bytes", ErrorArg::Unsigned(bytes.len() as u64))
                .caused_by(e)
        })?;
        let rgba = dyn_img.to_rgba8();
        let (w, h) = rgba.dimensions();
        Self::from_rgba8(w, h, rgba.into_raw())
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

    /// 写成 PNG。格式由 `image` 负责，不在调用方再实现编码器。
    pub fn save_png(&self, path: impl AsRef<Path>) -> Result<(), SparkError> {
        let path = path.as_ref();
        let path_arg = ErrorArg::Path(Arc::from(path.to_string_lossy().as_ref()));
        image::save_buffer_with_format(
            path,
            &self.rgba,
            self.width,
            self.height,
            image::ExtendedColorType::Rgba8,
            image::ImageFormat::Png,
        )
        .map_err(|e| {
            SparkError::new(codes::image_encode())
                .arg("path", path_arg)
                .arg("width", ErrorArg::Unsigned(self.width as u64))
                .arg("height", ErrorArg::Unsigned(self.height as u64))
                .caused_by(e)
        })
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
        Ok([
            self.rgba[i],
            self.rgba[i + 1],
            self.rgba[i + 2],
            self.rgba[i + 3],
        ])
    }
}

pub(crate) fn validate_region(img_w: u32, img_h: u32, region: Rect) -> Result<(), SparkError> {
    if region.w <= 0.0 || region.h <= 0.0 {
        return Err(SparkError::new(codes::image_region_invalid())
            .arg("w", ErrorArg::Float(region.w as f64))
            .arg("h", ErrorArg::Float(region.h as f64)));
    }
    if region.x < 0.0
        || region.y < 0.0
        || region.x + region.w > img_w as f32 + 1e-3
        || region.y + region.h > img_h as f32 + 1e-3
    {
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

#[cfg(test)]
mod tests {
    use super::*;
    use spark_core::Color;

    #[test]
    fn solid_and_uv() {
        let img = PixelImage::solid(64, 32, Color::rgb(1.0, 0.0, 0.0)).unwrap();
        assert_eq!(img.width(), 64);
        assert_eq!(img.pixel(0, 0).unwrap(), [255, 0, 0, 255]);
        let uv = img.uv_rect(Rect::new(16.0, 8.0, 16.0, 8.0)).unwrap();
        assert!((uv.x - 0.25).abs() < 1e-5);
        assert!((uv.w - 0.25).abs() < 1e-5);
    }

    #[test]
    fn errors_are_stable_codes() {
        let err = PixelImage::from_rgba8(1, 1, vec![0, 0, 0]).unwrap_err();
        assert_eq!(err.to_string(), "spark.image.rgba_length_mismatch");
    }

    #[test]
    fn png_roundtrip() {
        let src = PixelImage::solid(2, 1, Color::rgba(0.0, 1.0, 0.0, 0.5)).unwrap();
        let path = std::env::temp_dir().join(format!("spark-image-{}.png", std::process::id()));
        src.save_png(&path).unwrap();
        let loaded = PixelImage::load(&path).unwrap();
        assert_eq!(loaded.width(), 2);
        assert_eq!(loaded.height(), 1);
        assert_eq!(loaded.pixel(0, 0).unwrap(), src.pixel(0, 0).unwrap());
        let _ = std::fs::remove_file(&path);
    }
}
