//! GPU 纹理格式（不是 PNG/JPEG 等源文件格式）。

/// 纹理维度。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum TextureDimension {
    /// 一维。
    D1,
    /// 二维（默认）。
    #[default]
    D2,
    /// 三维体积。
    D3,
    /// 立方体贴图（6 个 2D 面）。
    Cube,
}

/// GPU 可采样 / 可上传的纹理格式。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TextureFormat {
    /// 单通道 8-bit。
    R8Unorm,
    /// 双通道 8-bit。
    Rg8Unorm,
    /// RGBA8 线性。
    Rgba8Unorm,
    /// RGBA8 sRGB。
    Rgba8UnormSrgb,
    /// RGBA 半精度浮点。
    Rgba16Float,
    /// RGBA 单精度浮点。
    Rgba32Float,
    /// BC1（DXT1）。
    Bc1RgbaUnorm,
    /// BC1 sRGB。
    Bc1RgbaUnormSrgb,
    /// BC3（DXT5）。
    Bc3RgbaUnorm,
    /// BC3 sRGB。
    Bc3RgbaUnormSrgb,
    /// BC5。
    Bc5RgUnorm,
    /// BC7。
    Bc7RgbaUnorm,
    /// BC7 sRGB。
    Bc7RgbaUnormSrgb,
    /// ETC2 RGBA8。
    Etc2Rgba8Unorm,
    /// ETC2 RGBA8 sRGB。
    Etc2Rgba8UnormSrgb,
    /// ASTC 4×4。
    Astc4x4Unorm,
    /// ASTC 4×4 sRGB。
    Astc4x4UnormSrgb,
    /// ASTC 6×6。
    Astc6x6Unorm,
    /// ASTC 6×6 sRGB。
    Astc6x6UnormSrgb,
}

impl TextureFormat {
    /// 是否为块压缩格式。
    pub const fn is_compressed(self) -> bool {
        matches!(
            self,
            Self::Bc1RgbaUnorm
                | Self::Bc1RgbaUnormSrgb
                | Self::Bc3RgbaUnorm
                | Self::Bc3RgbaUnormSrgb
                | Self::Bc5RgUnorm
                | Self::Bc7RgbaUnorm
                | Self::Bc7RgbaUnormSrgb
                | Self::Etc2Rgba8Unorm
                | Self::Etc2Rgba8UnormSrgb
                | Self::Astc4x4Unorm
                | Self::Astc4x4UnormSrgb
                | Self::Astc6x6Unorm
                | Self::Astc6x6UnormSrgb
        )
    }

    /// 压缩块宽高（像素）；非压缩为 `1×1`。
    pub const fn block_extent(self) -> (u32, u32) {
        match self {
            Self::Bc1RgbaUnorm
            | Self::Bc1RgbaUnormSrgb
            | Self::Bc3RgbaUnorm
            | Self::Bc3RgbaUnormSrgb
            | Self::Bc5RgUnorm
            | Self::Bc7RgbaUnorm
            | Self::Bc7RgbaUnormSrgb
            | Self::Etc2Rgba8Unorm
            | Self::Etc2Rgba8UnormSrgb
            | Self::Astc4x4Unorm
            | Self::Astc4x4UnormSrgb => (4, 4),
            Self::Astc6x6Unorm | Self::Astc6x6UnormSrgb => (6, 6),
            _ => (1, 1),
        }
    }

    /// 每块或每像素字节数（非压缩时为每像素）。
    pub const fn bytes_per_block(self) -> u32 {
        match self {
            Self::R8Unorm => 1,
            Self::Rg8Unorm => 2,
            Self::Rgba8Unorm | Self::Rgba8UnormSrgb => 4,
            Self::Rgba16Float => 8,
            Self::Rgba32Float => 16,
            Self::Bc1RgbaUnorm | Self::Bc1RgbaUnormSrgb => 8,
            Self::Bc3RgbaUnorm
            | Self::Bc3RgbaUnormSrgb
            | Self::Bc5RgUnorm
            | Self::Bc7RgbaUnorm
            | Self::Bc7RgbaUnormSrgb
            | Self::Etc2Rgba8Unorm
            | Self::Etc2Rgba8UnormSrgb
            | Self::Astc4x4Unorm
            | Self::Astc4x4UnormSrgb
            | Self::Astc6x6Unorm
            | Self::Astc6x6UnormSrgb => 16,
        }
    }

    /// 是否编码为 sRGB 采样语义。
    pub const fn is_srgb(self) -> bool {
        matches!(
            self,
            Self::Rgba8UnormSrgb
                | Self::Bc1RgbaUnormSrgb
                | Self::Bc3RgbaUnormSrgb
                | Self::Bc7RgbaUnormSrgb
                | Self::Etc2Rgba8UnormSrgb
                | Self::Astc4x4UnormSrgb
                | Self::Astc6x6UnormSrgb
        )
    }
}

/// 逻辑色彩空间标注（可与 [`TextureFormat::is_srgb`] 交叉校验）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum ColorSpace {
    /// 线性。
    Linear,
    /// sRGB。
    #[default]
    Srgb,
}

/// Alpha 解释。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum AlphaMode {
    /// 忽略 alpha。
    Opaque,
    /// 阈值裁剪。
    Mask,
    /// 标准混合（非预乘）。
    #[default]
    Blend,
    /// 预乘 alpha。
    Premultiplied,
}
