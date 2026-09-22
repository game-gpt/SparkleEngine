//! 采样器描述与纹理驻留状态。

/// 过滤模式。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum FilterMode {
    /// 最近点（像素艺术常用）。
    #[default]
    Nearest,
    /// 线性。
    Linear,
}

/// 寻址模式。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum AddressMode {
    /// 钳制到边。
    #[default]
    ClampToEdge,
    /// 重复。
    Repeat,
    /// 镜像重复。
    MirrorRepeat,
}

/// GPU 采样器描述（后端创建 `Sampler`）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct SamplerDesc {
    /// 放大过滤。
    pub mag_filter: FilterMode,
    /// 缩小过滤。
    pub min_filter: FilterMode,
    /// mip 过滤。
    pub mipmap_filter: FilterMode,
    /// U 寻址。
    pub address_u: AddressMode,
    /// V 寻址。
    pub address_v: AddressMode,
    /// W 寻址。
    pub address_w: AddressMode,
    /// 最大各向异性（1 = 关闭）。
    pub max_anisotropy: u16,
}

impl SamplerDesc {
    /// 像素艺术默认：全最近点 + clamp。
    pub const fn nearest_clamp() -> Self {
        Self {
            mag_filter: FilterMode::Nearest,
            min_filter: FilterMode::Nearest,
            mipmap_filter: FilterMode::Nearest,
            address_u: AddressMode::ClampToEdge,
            address_v: AddressMode::ClampToEdge,
            address_w: AddressMode::ClampToEdge,
            max_anisotropy: 1,
        }
    }

    /// 线性过滤 + clamp。
    pub const fn linear_clamp() -> Self {
        Self {
            mag_filter: FilterMode::Linear,
            min_filter: FilterMode::Linear,
            mipmap_filter: FilterMode::Linear,
            address_u: AddressMode::ClampToEdge,
            address_v: AddressMode::ClampToEdge,
            address_w: AddressMode::ClampToEdge,
            max_anisotropy: 1,
        }
    }
}

impl Default for SamplerDesc {
    fn default() -> Self {
        Self::nearest_clamp()
    }
}

/// GPU 纹理资源生命周期状态。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum TextureState {
    /// 未加载。
    #[default]
    Unloaded,
    /// 资产读取中。
    Loading,
    /// 已解码 / 转码为可上传包（不必是 RGBA）。
    Decoded,
    /// 正在上传。
    Uploading,
    /// 显存驻留可用。
    Resident,
    /// 已换出。
    Evicted,
    /// 失败。
    Failed,
}
