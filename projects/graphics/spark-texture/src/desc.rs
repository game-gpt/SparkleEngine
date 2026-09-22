//! 纹理逻辑描述（不含像素）。

use spark_types::{ErrorArg, SparkError, codes};

use crate::{
    format::{AlphaMode, ColorSpace, TextureDimension, TextureFormat},
    usage::TextureUsage,
};

/// 纹理逻辑描述（不含像素字节；上传前须与 [`crate::TextureData`] 对齐）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TextureDesc {
    /// 几何维度，决定其余尺寸字段语义。
    pub dimension: TextureDimension,
    /// 宽（像素），必须 `> 0`。
    pub width: u32,
    /// 高（像素），必须 `> 0`；1D 时应为 1。
    pub height: u32,
    /// 深度切片数、数组层数，或 Cube 时固定为 6；必须 `> 0`。
    pub depth_or_layers: u32,
    /// mip 级数，至少为 1。
    pub mip_levels: u32,
    /// MSAA 采样数，至少为 1；普通采样纹理为 1。
    pub sample_count: u32,
    /// GPU 像素 / 块格式。
    pub format: TextureFormat,
    /// 逻辑色彩空间（应与 `format.is_srgb()` 一致，除非刻意覆盖）。
    pub color_space: ColorSpace,
    /// Alpha 合成模式。
    pub alpha_mode: AlphaMode,
    /// 用途标志（须覆盖实际上传 / 绑定路径）。
    pub usage: TextureUsage,
}

impl TextureDesc {
    /// 构造常见 2D 采样纹理描述：单层、单 mip、`sample_count = 1`、`usage = sampled()`。
    ///
    /// `color_space` 由 `format.is_srgb()` 推导；调用方可再改 `mip_levels` / `usage` 等字段。
    /// 本方法不校验尺寸；上传前请调用 [`Self::validate`]。
    pub fn d2(width: u32, height: u32, format: TextureFormat) -> Self {
        let color_space = if format.is_srgb() { ColorSpace::Srgb } else { ColorSpace::Linear };
        Self {
            dimension: TextureDimension::D2,
            width,
            height,
            depth_or_layers: 1,
            mip_levels: 1,
            sample_count: 1,
            format,
            color_space,
            alpha_mode: AlphaMode::Blend,
            usage: TextureUsage::sampled(),
        }
    }

    /// 校验尺寸与 mip / 采样数基本合法。
    ///
    /// 失败：`width|height|depth_or_layers == 0` → `texture_size_invalid`；
    /// `mip_levels|sample_count == 0` 或 Cube 层数≠6 → `texture_upload_invalid`。
    pub fn validate(&self) -> Result<(), SparkError> {
        if self.width == 0 || self.height == 0 || self.depth_or_layers == 0 {
            return Err(SparkError::new(codes::texture_size_invalid())
                .arg("width", ErrorArg::Unsigned(self.width as u64))
                .arg("height", ErrorArg::Unsigned(self.height as u64))
                .arg("depth_or_layers", ErrorArg::Unsigned(self.depth_or_layers as u64)));
        }
        if self.mip_levels == 0 || self.sample_count == 0 {
            return Err(SparkError::new(codes::texture_upload_invalid())
                .arg("mip_levels", ErrorArg::Unsigned(self.mip_levels as u64))
                .arg("sample_count", ErrorArg::Unsigned(self.sample_count as u64)));
        }
        if self.dimension == TextureDimension::Cube && self.depth_or_layers != 6 {
            return Err(SparkError::new(codes::texture_upload_invalid())
                .arg("reason", ErrorArg::String("cube_layers_must_be_6".into()))
                .arg("depth_or_layers", ErrorArg::Unsigned(self.depth_or_layers as u64)));
        }
        Ok(())
    }
}

/// 仅尺寸与格式的轻量信息（Sprite / Widget / 图集用，无像素）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct TextureInfo {
    /// 宽（像素）。
    pub width: u32,
    /// 高（像素）。
    pub height: u32,
    /// GPU 格式。
    pub format: TextureFormat,
    /// mip 级数（至少 1）。
    pub mip_levels: u32,
}

impl TextureInfo {
    /// 从 [`TextureDesc`] 提取。
    pub fn from_desc(desc: &TextureDesc) -> Self {
        Self { width: desc.width, height: desc.height, format: desc.format, mip_levels: desc.mip_levels }
    }
}
