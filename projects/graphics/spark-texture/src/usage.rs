//! 纹理用途与设备能力。

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

/// 渲染设备纹理相关能力（由后端填入，格式层按此选型）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct DeviceCaps {
    /// BC / DXT 族。
    pub supports_bc: bool,
    /// ETC2。
    pub supports_etc2: bool,
    /// ASTC。
    pub supports_astc: bool,
    /// 半精度浮点纹理。
    pub supports_float16: bool,
    /// 纹理数组。
    pub supports_texture_arrays: bool,
    /// GPU 生成 mip。
    pub supports_mip_generation: bool,
    /// 单边最大尺寸。
    pub max_texture_dimension: u32,
}

impl DeviceCaps {
    /// 保守兜底：仅未压缩 2D，最大 8192。
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
}

impl Default for DeviceCaps {
    fn default() -> Self {
        Self::conservative()
    }
}
