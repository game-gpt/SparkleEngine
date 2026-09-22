//! UI 纹理解析：Widget 持 `AssetId`，宿主提供 GPU `TextureId` 与驻留状态。

use spark_asset::AssetId;
use spark_types::{Color, Rect, Vec2};
use spark_renderer::{SamplerDesc, TextureId, TextureState};

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

/// 宿主解析结果：句柄 + 采样描述 + 驻留状态。
///
/// Widget **不**负责上传；`Pending` / `Loading` / `Uploading` 时可画占位，仅 `Resident` 提交贴图。
#[derive(Debug, Clone, Copy)]
pub struct ResolvedTexture {
    /// 可用时的 GPU 纹理句柄。
    pub texture: Option<TextureId>,
    /// 纹理像素尺寸（供 intrinsic / 调试）；未知可为零。
    pub size: Vec2,
    /// 驻留状态（与 `spark-texture::TextureState` 对齐）。
    pub status: TextureState,
    /// 采样偏好（后端可忽略并使用全局默认）。
    pub sampler: SamplerDesc,
}

impl ResolvedTexture {
    /// 已驻留、可绘制。
    pub fn resident(texture: TextureId, size: Vec2) -> Self {
        Self { texture: Some(texture), size, status: TextureState::Resident, sampler: SamplerDesc::nearest_clamp() }
    }

    /// 加载 / 解码 / 上传中。
    pub fn pending(size: Vec2) -> Self {
        Self { texture: None, size, status: TextureState::Loading, sampler: SamplerDesc::nearest_clamp() }
    }

    /// 失败。
    pub fn failed() -> Self {
        Self { texture: None, size: Vec2::ZERO, status: TextureState::Failed, sampler: SamplerDesc::nearest_clamp() }
    }

    /// 是否可提交贴图绘制。
    pub fn is_drawable(&self) -> bool {
        self.status == TextureState::Resident && self.texture.is_some()
    }

    /// 是否应画加载占位。
    pub fn is_pending(&self) -> bool {
        matches!(self.status, TextureState::Unloaded | TextureState::Loading | TextureState::Decoded | TextureState::Uploading)
    }

    pub fn with_sampler(mut self, sampler: SamplerDesc) -> Self {
        self.sampler = sampler;
        self
    }
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

/// 测试用：固定映射一个 `AssetId` → 驻留纹理。
#[derive(Debug, Clone)]
pub struct MapTextureResolver {
    pub asset: AssetId,
    pub texture: TextureId,
    pub size: Vec2,
}

impl UiTextureResolver for MapTextureResolver {
    fn resolve(&mut self, asset: AssetId) -> Option<ResolvedTexture> {
        if asset == self.asset { Some(ResolvedTexture::resident(self.texture, self.size)) } else { None }
    }
}
