//! 纹理逻辑描述（不含像素）。

use spark_types::{ErrorArg, SparkError, codes};

use crate::{
    format::{AlphaMode, ColorSpace, TextureDimension, TextureFormat},
    usage::TextureUsage,
};

/// 纹理逻辑描述。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TextureDesc {
    /// 维度。
    pub dimension: TextureDimension,
    /// 宽（像素）。
    pub width: u32,
    /// 高（像素）；1D 时为 1。
    pub height: u32,
    /// 深度或数组层数 / Cube 为 6。
    pub depth_or_layers: u32,
    /// mip 级数（至少 1）。
    pub mip_levels: u32,
    /// MSAA 采样数。
    pub sample_count: u32,
    /// GPU 格式。
    pub format: TextureFormat,
    /// 色彩空间标注。
    pub color_space: ColorSpace,
    /// Alpha 模式。
    pub alpha_mode: AlphaMode,
    /// 用途。
    pub usage: TextureUsage,
}

impl TextureDesc {
    /// 常见 2D 采样纹理描述（单 mip，调用方可再改 `mip_levels`）。
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

/// 仅尺寸与格式的轻量信息（Sprite / Widget 用，无像素）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct TextureInfo {
    /// 宽。
    pub width: u32,
    /// 高。
    pub height: u32,
    /// 格式。
    pub format: TextureFormat,
    /// mip 数。
    pub mip_levels: u32,
}

impl TextureInfo {
    /// 从 [`TextureDesc`] 提取。
    pub fn from_desc(desc: &TextureDesc) -> Self {
        Self { width: desc.width, height: desc.height, format: desc.format, mip_levels: desc.mip_levels }
    }
}
