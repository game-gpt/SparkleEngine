//! GPU 纹理格式（不是 PNG/JPEG 等源文件格式）。

/// 纹理维度（与 GPU 资源类型对应，非源文件维度）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum TextureDimension {
    /// 一维纹理；`TextureDesc::height` 应为 1。
    D1,
    /// 二维纹理（默认）；数组层数写在 `depth_or_layers`。
    #[default]
    D2,
    /// 三维体积纹理；`depth_or_layers` 为 depth（像素）。
    D3,
    /// 立方体贴图（6 个 2D 面）；`depth_or_layers` 必须为 6。
    Cube,
}

/// GPU 可采样 / 可上传的纹理格式（块压缩与未压缩子集）。
///
/// 块大小与每块字节见 [`Self::block_extent`] / [`Self::bytes_per_block`]；
/// 设备是否支持见 [`crate::DeviceCaps::supports_format`]。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TextureFormat {
    /// 单通道 8-bit UNORM（1 字节/像素）。
    R8Unorm,
    /// 双通道 8-bit UNORM（2 字节/像素）。
    Rg8Unorm,
    /// RGBA8 线性 UNORM（4 字节/像素）。
    Rgba8Unorm,
    /// RGBA8 sRGB UNORM（采样时做 sRGB→线性）。
    Rgba8UnormSrgb,
    /// RGBA 半精度浮点（8 字节/像素）；需 `supports_float16`。
    Rgba16Float,
    /// RGBA 单精度浮点（16 字节/像素）。
    Rgba32Float,
    /// BC1 / DXT1 RGBA（4×4，8 字节/块）；需 `supports_bc`。
    Bc1RgbaUnorm,
    /// BC1 sRGB。
    Bc1RgbaUnormSrgb,
    /// BC3 / DXT5 RGBA（4×4，16 字节/块）。
    Bc3RgbaUnorm,
    /// BC3 sRGB。
    Bc3RgbaUnormSrgb,
    /// BC5 双通道（法线等，4×4，16 字节/块）。
    Bc5RgUnorm,
    /// BC7 RGBA（4×4，16 字节/块）。
    Bc7RgbaUnorm,
    /// BC7 sRGB。
    Bc7RgbaUnormSrgb,
    /// ETC2 RGBA8（4×4，16 字节/块）；需 `supports_etc2`。
    Etc2Rgba8Unorm,
    /// ETC2 RGBA8 sRGB。
    Etc2Rgba8UnormSrgb,
    /// ASTC 4×4 UNORM（16 字节/块）；需 `supports_astc`。
    Astc4x4Unorm,
    /// ASTC 4×4 sRGB。
    Astc4x4UnormSrgb,
    /// ASTC 6×6 UNORM（16 字节/块）。
    Astc6x6Unorm,
    /// ASTC 6×6 sRGB。
    Astc6x6UnormSrgb,
}

impl TextureFormat {
    /// 是否为块压缩格式（不可走 CPU mip 生成路径）。
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

    /// 压缩块宽高（像素）；非压缩为 `(1, 1)`。
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

    /// 每块字节数；非压缩时等于每像素字节数。
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

    /// 是否为 sRGB 采样语义的格式变体（与 [`ColorSpace::Srgb`] 宜一致）。
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

/// 逻辑色彩空间标注（可与 [`TextureFormat::is_srgb`] 交叉校验；不改变字节布局）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum ColorSpace {
    /// 线性数值（光照计算、数据图）。
    Linear,
    /// sRGB 显示色（默认；漫反射 / UI）。
    #[default]
    Srgb,
}

/// Alpha 通道解释（影响混合，不改变存储格式）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum AlphaMode {
    /// 忽略 alpha，当作不透明。
    Opaque,
    /// 阈值裁剪（具体阈值由材质 / 后端约定）。
    Mask,
    /// 标准混合，非预乘（默认）。
    #[default]
    Blend,
    /// 预乘 alpha。
    Premultiplied,
}
