//! 纹理用途与设备能力。

use crate::format::TextureFormat;

/// 纹理用途标志（可按位组合）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct TextureUsage(u32);

impl TextureUsage {
    /// 可作为拷贝源。
    pub const COPY_SRC: Self = Self(1 << 0);
    /// 可作为拷贝目标 / 上传目标。
    pub const COPY_DST: Self = Self(1 << 1);
    /// 可绑定为采样纹理。
    pub const TEXTURE_BINDING: Self = Self(1 << 2);
    /// 可绑定为 storage 纹理。
    pub const STORAGE_BINDING: Self = Self(1 << 3);
    /// 可作为渲染附件。
    pub const RENDER_ATTACHMENT: Self = Self(1 << 4);

    /// 常见采样纹理：上传 + 采样。
    pub const fn sampled() -> Self {
        Self(Self::COPY_DST.0 | Self::TEXTURE_BINDING.0)
    }

    /// 空用途。
    pub const fn empty() -> Self {
        Self(0)
    }

    /// 按位或。
    pub const fn union(self, other: Self) -> Self {
        Self(self.0 | other.0)
    }

    /// 是否包含全部 `other` 位。
    pub const fn contains(self, other: Self) -> bool {
        self.0 & other.0 == other.0
    }

    /// 原始位。
    pub const fn bits(self) -> u32 {
        self.0
    }
}

impl Default for TextureUsage {
    fn default() -> Self {
        Self::sampled()
    }
}

/// 渲染设备纹理相关能力（由后端填入；格式插件 / 上传前用 [`DeviceCaps::supports_format`] 选型）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct DeviceCaps {
    /// 是否支持 BC / DXT 族（BC1/3/5/7）。
    pub supports_bc: bool,
    /// 是否支持 ETC2。
    pub supports_etc2: bool,
    /// 是否支持 ASTC。
    pub supports_astc: bool,
    /// 是否支持半精度浮点纹理（`Rgba16Float`）。
    pub supports_float16: bool,
    /// 是否支持 2D 纹理数组（`depth_or_layers > 1`）。
    pub supports_texture_arrays: bool,
    /// 是否支持 GPU 侧自动生成 mip。
    pub supports_mip_generation: bool,
    /// 单边最大尺寸（像素）；超出则 `validate_for_device` 失败。
    pub max_texture_dimension: u32,
}

impl DeviceCaps {
    /// 保守兜底：仅未压缩 8-bit / 32-bit float 2D，最大 8192。
    pub const fn conservative() -> Self {
        Self {
            supports_bc: false,
            supports_etc2: false,
            supports_astc: false,
            supports_float16: false,
            supports_texture_arrays: false,
            supports_mip_generation: false,
            max_texture_dimension: 8192,
        }
    }

    /// 当前能力是否覆盖该 GPU 格式。
    pub const fn supports_format(self, format: TextureFormat) -> bool {
        match format {
            TextureFormat::R8Unorm
            | TextureFormat::Rg8Unorm
            | TextureFormat::Rgba8Unorm
            | TextureFormat::Rgba8UnormSrgb
            | TextureFormat::Rgba32Float => true,
            TextureFormat::Rgba16Float => self.supports_float16,
            TextureFormat::Bc1RgbaUnorm
            | TextureFormat::Bc1RgbaUnormSrgb
            | TextureFormat::Bc3RgbaUnorm
            | TextureFormat::Bc3RgbaUnormSrgb
            | TextureFormat::Bc5RgUnorm
            | TextureFormat::Bc7RgbaUnorm
            | TextureFormat::Bc7RgbaUnormSrgb => self.supports_bc,
            TextureFormat::Etc2Rgba8Unorm | TextureFormat::Etc2Rgba8UnormSrgb => self.supports_etc2,
            TextureFormat::Astc4x4Unorm | TextureFormat::Astc4x4UnormSrgb | TextureFormat::Astc6x6Unorm | TextureFormat::Astc6x6UnormSrgb => {
                self.supports_astc
            }
        }
    }
}

impl Default for DeviceCaps {
    fn default() -> Self {
        Self::conservative()
    }
}
