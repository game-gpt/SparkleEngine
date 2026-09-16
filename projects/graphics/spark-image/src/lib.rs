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

use image::ImageReader;
use spark_core::{Color, Rect, SparkError};

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
            .ok_or_else(|| SparkError::Message("图像尺寸溢出".into()))?;
        if rgba.len() != need {
            return Err(SparkError::Message(format!(
                "RGBA 长度不符：期望 {need}，得到 {}",
                rgba.len()
            )));
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
            .ok_or_else(|| SparkError::Message("图像尺寸溢出".into()))?;
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
        let reader = ImageReader::open(path)
            .map_err(|e| SparkError::Message(format!("打开图像失败 {}: {e}", path.display())))?
            .with_guessed_format()
            .map_err(|e| SparkError::Message(format!("探测图像格式失败 {}: {e}", path.display())))?;
        let dyn_img = reader
            .decode()
            .map_err(|e| SparkError::Message(format!("解码图像失败 {}: {e}", path.display())))?;
        let rgba = dyn_img.to_rgba8();
        let (w, h) = rgba.dimensions();
        Self::from_rgba8(w, h, rgba.into_raw())
    }

    /// 从内存字节解码。
    pub fn load_from_memory(bytes: &[u8]) -> Result<Self, SparkError> {
        let dyn_img = image::load_from_memory(bytes)
            .map_err(|e| SparkError::Message(format!("解码图像失败：{e}")))?;
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
            return Err(SparkError::Message(format!(
                "像素越界 ({x},{y}) 于 {}x{}",
                self.width, self.height
            )));
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
        return Err(SparkError::Message("源矩形宽高须为正".into()));
    }
    if region.x < 0.0
        || region.y < 0.0
        || region.x + region.w > img_w as f32 + 1e-3
        || region.y + region.h > img_h as f32 + 1e-3
    {
        return Err(SparkError::Message(format!(
            "源矩形 {:?} 超出图像 {}x{}",
            region, img_w, img_h
        )));
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
        assert!((uv.y - 0.25).abs() < 1e-5);
        assert!((uv.w - 0.25).abs() < 1e-5);
        assert!((uv.h - 0.25).abs() < 1e-5);
    }
}
