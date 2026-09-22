//! 上传策略与纹理上传包。

use std::sync::Arc;

use spark_core::{ErrorArg, SparkError, codes};

use crate::{
    data::TextureData,
    desc::TextureDesc,
    format::{AlphaMode, ColorSpace, TextureFormat},
};

/// 何时调度 GPU 创建 / 拷贝。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum UploadPolicy {
    /// 本帧立即上传。
    #[default]
    Immediate,
    /// 延迟到预算允许。
    Deferred,
    /// 流式分块。
    Streaming,
    /// 稀疏驻留（后置）。
    Sparse,
}

/// 上传后是否保留 CPU 字节副本。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum CpuCopyPolicy {
    /// 保留（热重载 / 调试）。
    Keep,
    /// 上传后可丢弃。
    #[default]
    Discard,
}

/// 期望显存驻留策略。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum Residency {
    /// 短期（过场等）。
    Transient,
    /// 常驻。
    #[default]
    Resident,
    /// 可换出。
    Streamable,
}

/// mip 来源策略。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum MipmapPolicy {
    /// 仅上传数据中已有层级（由 `desc.mip_levels` / layout 决定）。
    None,
    /// 数据仅含 base 级；后端可在 CPU 生成其余 mip（未压缩 RGBA 路径）。
    #[default]
    GenerateCpu,
    /// 数据已含完整 mip 链。
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
    /// RGBA8 sRGB 单 mip；默认 CPU 生成 mip 链。
    pub fn rgba8_srgb(width: u32, height: u32, rgba: impl Into<Arc<[u8]>>) -> Result<Self, SparkError> {
        Self::rgba8(width, height, rgba, true)
    }

    /// RGBA8 线性或 sRGB。
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

    /// 校验描述与数据。
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
}
