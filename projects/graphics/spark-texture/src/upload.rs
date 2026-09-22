//! 上传策略与纹理上传包。

use std::sync::Arc;

use spark_types::{ErrorArg, SparkError, codes};

use crate::{
    data::TextureData,
    desc::TextureDesc,
    format::{AlphaMode, ColorSpace, TextureDimension, TextureFormat},
    usage::DeviceCaps,
};

/// 何时调度 GPU 创建 / 拷贝（由渲染后端解释，格式层只携带意图）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum UploadPolicy {
    /// 本帧立即创建并拷贝（默认，小图 / 启动资源）。
    #[default]
    Immediate,
    /// 延迟到帧预算允许时再上传。
    Deferred,
    /// 按块流式上传（大图 / 流式关卡）。
    Streaming,
    /// 稀疏驻留意图（能力后置；当前后端可降级为常驻）。
    Sparse,
}

/// 上传完成后是否仍保留 [`crate::TextureData`] 中的 CPU 字节。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum CpuCopyPolicy {
    /// 保留 CPU 副本（热重载、CPU 读回、调试用）。
    Keep,
    /// 上传成功后可丢弃 CPU 字节以省内存（默认）。
    #[default]
    Discard,
}

/// 期望显存驻留策略（换出实现由后端决定）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum Residency {
    /// 短期占用（过场、一次性特效）。
    Transient,
    /// 会话内常驻（默认，UI / 主角贴图等）。
    #[default]
    Resident,
    /// 允许按需换出 / 再流回。
    Streamable,
}

/// mip 链来源策略（须与 `desc.mip_levels` / `data.layout.mip_offsets` 一致）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum MipmapPolicy {
    /// 只上传数据里已有的层级；不生成额外 mip。
    None,
    /// 数据仅含 base 级；后端可在 CPU 生成其余 mip（仅未压缩路径；默认）。
    #[default]
    GenerateCpu,
    /// 数据已含完整 mip 链，`mip_offsets.len() == desc.mip_levels`。
    Provided,
}

/// 一次可调度的 GPU 纹理创建任务。
#[derive(Debug, Clone)]
pub struct TextureUpload {
    /// 逻辑描述。
    pub desc: TextureDesc,
    /// 字节与布局。
    pub data: TextureData,
    /// 调度策略。
    pub upload_policy: UploadPolicy,
    /// CPU 副本策略。
    pub cpu_copy: CpuCopyPolicy,
    /// 驻留策略。
    pub residency: Residency,
    /// mip 策略。
    pub mipmap: MipmapPolicy,
    /// 调试名。
    pub debug_name: Option<Arc<str>>,
}

impl TextureUpload {
    /// 构造 sRGB RGBA8 单 mip 上传包；默认 [`MipmapPolicy::GenerateCpu`]。
    ///
    /// 等价于 `rgba8(..., srgb = true)`。`rgba.len()` 须为 `width * height * 4`。
    pub fn rgba8_srgb(width: u32, height: u32, rgba: impl Into<Arc<[u8]>>) -> Result<Self, SparkError> {
        Self::rgba8(width, height, rgba, true)
    }

    /// 构造 RGBA8 单 mip 2D 上传包。
    ///
    /// `srgb = true` → `Rgba8UnormSrgb` + `ColorSpace::Srgb`；否则线性格式。
    /// 默认立即上传、丢弃 CPU 副本、常驻、[`MipmapPolicy::GenerateCpu`]。
    /// 长度不符或尺寸为 0 时返回与 [`crate::TextureData::from_uncompressed`] 相同的错误。
    pub fn rgba8(width: u32, height: u32, rgba: impl Into<Arc<[u8]>>, srgb: bool) -> Result<Self, SparkError> {
        let format = if srgb { TextureFormat::Rgba8UnormSrgb } else { TextureFormat::Rgba8Unorm };
        let data = TextureData::from_uncompressed(format, width, height, rgba)?;
        let mut desc = TextureDesc::d2(width, height, format);
        desc.color_space = if srgb { ColorSpace::Srgb } else { ColorSpace::Linear };
        desc.alpha_mode = AlphaMode::Blend;
        // GenerateCpu 时后端会扩展 mip_levels；描述先记 1，由上传器改写实际 GPU mip 数。
        Ok(Self {
            desc,
            data,
            upload_policy: UploadPolicy::Immediate,
            cpu_copy: CpuCopyPolicy::Discard,
            residency: Residency::Resident,
            mipmap: MipmapPolicy::GenerateCpu,
            debug_name: None,
        })
    }

    /// 附调试名。
    pub fn with_debug_name(mut self, name: impl Into<Arc<str>>) -> Self {
        self.debug_name = Some(name.into());
        self
    }

    /// 改 mip 策略。
    pub fn with_mipmap(mut self, policy: MipmapPolicy) -> Self {
        self.mipmap = policy;
        self
    }

    /// 校验描述与数据 / mip 策略是否自洽。
    ///
    /// `GenerateCpu`：禁止压缩格式，且 `mip_levels` 与 `mip_offsets` 须为单级；
    /// `None` / `Provided`：走 [`crate::TextureData::validate_against`]。
    pub fn validate(&self) -> Result<(), SparkError> {
        self.desc.validate()?;
        match self.mipmap {
            MipmapPolicy::GenerateCpu => {
                if self.desc.format.is_compressed() {
                    return Err(SparkError::new(codes::texture_upload_invalid())
                        .arg("reason", ErrorArg::String("generate_cpu_not_for_compressed".into())));
                }
                if self.desc.mip_levels != 1 {
                    return Err(SparkError::new(codes::texture_upload_invalid())
                        .arg("reason", ErrorArg::String("generate_cpu_expects_base_mip".into()))
                        .arg("mip_levels", ErrorArg::Unsigned(self.desc.mip_levels as u64)));
                }
                if self.data.layout.mip_offsets.len() != 1 {
                    return Err(SparkError::new(codes::texture_layout_invalid())
                        .arg("reason", ErrorArg::String("generate_cpu_single_mip_offset".into())));
                }
            }
            MipmapPolicy::None | MipmapPolicy::Provided => {
                self.data.validate_against(&self.desc)?;
            }
        }
        Ok(())
    }

    /// 在 [`Self::validate`] 之上再按 [`DeviceCaps`] 检查格式与尺寸上限。
    ///
    /// 失败：格式不受支持 → `texture_format_unsupported`；边长超限 → `texture_size_invalid`；
    /// 2D 数组而设备无数组能力 → `texture_upload_invalid`。
    pub fn validate_for_device(&self, caps: &DeviceCaps) -> Result<(), SparkError> {
        self.validate()?;
        if !caps.supports_format(self.desc.format) {
            return Err(SparkError::new(codes::texture_format_unsupported())
                .arg("format", ErrorArg::String(Arc::from(format!("{:?}", self.desc.format))))
                .arg("supports_bc", ErrorArg::Bool(caps.supports_bc))
                .arg("supports_etc2", ErrorArg::Bool(caps.supports_etc2))
                .arg("supports_astc", ErrorArg::Bool(caps.supports_astc))
                .arg("supports_float16", ErrorArg::Bool(caps.supports_float16)));
        }
        let desc = &self.desc;
        let max_edge = match desc.dimension {
            TextureDimension::D1 => desc.width,
            TextureDimension::D2 | TextureDimension::Cube => desc.width.max(desc.height),
            TextureDimension::D3 => desc.width.max(desc.height).max(desc.depth_or_layers),
        };
        if max_edge > caps.max_texture_dimension {
            return Err(SparkError::new(codes::texture_size_invalid())
                .arg("reason", ErrorArg::String(Arc::from("exceeds_device_max")))
                .arg("max_edge", ErrorArg::Unsigned(max_edge as u64))
                .arg("max_texture_dimension", ErrorArg::Unsigned(caps.max_texture_dimension as u64)));
        }
        if desc.dimension == TextureDimension::D2 && desc.depth_or_layers > 1 && !caps.supports_texture_arrays {
            return Err(SparkError::new(codes::texture_upload_invalid())
                .arg("reason", ErrorArg::String(Arc::from("texture_arrays_unsupported")))
                .arg("depth_or_layers", ErrorArg::Unsigned(desc.depth_or_layers as u64)));
        }
        Ok(())
    }
}
