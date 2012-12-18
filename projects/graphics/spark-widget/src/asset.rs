//! UI 纹理解析：Widget 持 `AssetId`，宿主提供 GPU `TextureId`。

use spark_asset::AssetId;
use spark_core::{Color, Rect, Vec2};
use spark_renderer::TextureId;

/// 控件上的图片源（不持 GPU 资源）。
#[derive(Debug, Clone)]
pub struct UiImage {
    pub asset: AssetId,
    /// 归一化 UV（默认整图）。
    pub uv: Rect,
    pub tint: Color,
    /// 固有尺寸提示（逻辑像素）；缺省时 layout 用 32×32。
    pub preferred_size: Option<Vec2>,
}

impl UiImage {
    pub fn new(asset: AssetId) -> Self {
        Self { asset, uv: Rect::new(0.0, 0.0, 1.0, 1.0), tint: Color::rgb(1.0, 1.0, 1.0), preferred_size: None }
    }

    pub fn with_uv(mut self, uv: Rect) -> Self {
        self.uv = uv;
        self
    }

    pub fn with_tint(mut self, tint: Color) -> Self {
        self.tint = tint;
        self
    }

    pub fn with_preferred_size(mut self, size: Vec2) -> Self {
        self.preferred_size = Some(size);
        self
    }
}

/// 宿主解析结果。
#[derive(Debug, Clone, Copy)]
pub struct ResolvedTexture {
    pub texture: TextureId,
    /// 纹理像素尺寸（供 intrinsic / 调试）。
    pub size: Vec2,
}

/// 将 `AssetId` 解析为可绘制纹理。由游戏 / Studio 实现缓存与上传。
pub trait UiTextureResolver: Send {
    fn resolve(&mut self, asset: AssetId) -> Option<ResolvedTexture>;
}

/// 默认空解析器（无图可画）。
#[derive(Debug, Default, Clone, Copy)]
pub struct NullTextureResolver;

impl UiTextureResolver for NullTextureResolver {
    fn resolve(&mut self, _asset: AssetId) -> Option<ResolvedTexture> {
        None
    }
}

/// 测试用：固定映射一个 `AssetId` → 纹理。
#[derive(Debug, Clone)]
pub struct MapTextureResolver {
    pub asset: AssetId,
    pub texture: TextureId,
    pub size: Vec2,
}

impl UiTextureResolver for MapTextureResolver {
    fn resolve(&mut self, asset: AssetId) -> Option<ResolvedTexture> {
        if asset == self.asset { Some(ResolvedTexture { texture: self.texture, size: self.size }) } else { None }
    }
}
