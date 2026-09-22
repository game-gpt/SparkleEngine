//! 图集区域与精灵元数据（不持有 CPU 像素）。

use spark_core::{Rect, Vec2};

use crate::desc::TextureInfo;

/// 图集或单图上的精灵区域。
#[derive(Debug, Clone, PartialEq)]
pub struct SpriteRegion {
    /// 像素矩形（相对纹理左上）。
    pub pixel_rect: Rect,
    /// 归一化 UV；`None` 表示按 [`TextureInfo`] 现算。
    pub uv_rect: Option<Rect>,
    /// 枢轴（相对像素矩形左上，或约定为逻辑尺寸空间）。
    pub pivot: Vec2,
    /// 逻辑绘制尺寸（可与像素矩形不同，如 UI 缩放）。
    pub logical_size: Vec2,
}

impl SpriteRegion {
    /// 用整张纹理作为区域。
    pub fn full(info: TextureInfo) -> Self {
        let w = info.width as f32;
        let h = info.height as f32;
        Self {
            pixel_rect: Rect::new(0.0, 0.0, w, h),
            uv_rect: Some(Rect::new(0.0, 0.0, 1.0, 1.0)),
            pivot: Vec2::new(w * 0.5, h * 0.5),
            logical_size: Vec2::new(w, h),
        }
    }

    /// 按纹理尺寸计算归一化 UV。
    pub fn uv_for(&self, info: TextureInfo) -> Rect {
        if let Some(uv) = self.uv_rect {
            return uv;
        }
        let w = info.width.max(1) as f32;
        let h = info.height.max(1) as f32;
        Rect::new(self.pixel_rect.x / w, self.pixel_rect.y / h, self.pixel_rect.w / w, self.pixel_rect.h / h)
    }
}

/// 图集元数据：一张纹理上的命名区域（句柄由资产 / 渲染层持有）。
#[derive(Debug, Clone, PartialEq)]
pub struct AtlasMetadata {
    /// 纹理尺寸信息。
    pub texture: TextureInfo,
    /// 区域表（调用方自管键类型时可用平行结构；此处用字符串键的简单表）。
    pub regions: Vec<(String, SpriteRegion)>,
}

impl AtlasMetadata {
    /// 按名查找。
    pub fn get(&self, name: &str) -> Option<&SpriteRegion> {
        self.regions.iter().find(|(k, _)| k == name).map(|(_, r)| r)
    }
}
