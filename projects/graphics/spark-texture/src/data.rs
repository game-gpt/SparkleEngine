//! 纹理字节布局与数据块。

use std::sync::Arc;

use spark_types::{ErrorArg, SparkError, codes};

use crate::{desc::TextureDesc, format::TextureFormat};

/// 压缩 / 多 mip / 多层纹理的字节布局。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TextureLayout {
    /// 每行字节数（含对齐 padding）；`0` 表示按紧凑布局由后端推算。
    pub row_pitch: u32,
    /// 每层 / 每 depth slice 字节数；`0` 表示紧凑。
    pub slice_pitch: u32,
    /// 各级 mip 在 [`TextureData::bytes`] 中的起始偏移。
    pub mip_offsets: Vec<u64>,
    /// 各层起始偏移（相对整块或相对 mip0，由约定：相对整块字节缓冲）。
    pub layer_offsets: Vec<u64>,
    /// 压缩块宽（像素）。
    pub block_width: u32,
    /// 压缩块高（像素）。
    pub block_height: u32,
    /// 每块字节数（非压缩时等于每像素字节）。
    pub bytes_per_block: u32,
}

impl TextureLayout {
    /// 单层单 mip、紧凑行主序布局（适用于未压缩格式）。
    pub fn tightly_packed_2d(format: TextureFormat, width: u32, height: u32) -> Self {
        let (bw, bh) = format.block_extent();
        let bpb = format.bytes_per_block();
        let blocks_x = width.div_ceil(bw);
        let row_pitch = blocks_x.saturating_mul(bpb);
        Self {
            row_pitch,
            slice_pitch: row_pitch.saturating_mul(height.div_ceil(bh)),
            mip_offsets: vec![0],
            layer_offsets: vec![0],
            block_width: bw,
            block_height: bh,
            bytes_per_block: bpb,
        }
    }

    /// 期望的紧凑单层单 mip 字节数。
    pub fn expected_tight_bytes(&self, width: u32, height: u32) -> Option<usize> {
        let blocks_x = width.div_ceil(self.block_width) as usize;
        let blocks_y = height.div_ceil(self.block_height) as usize;
        blocks_x.checked_mul(blocks_y)?.checked_mul(self.bytes_per_block as usize)
    }
}

/// 纹理像素 / 压缩块字节。
#[derive(Debug, Clone)]
pub struct TextureData {
    /// 布局。
    pub layout: TextureLayout,
    /// 原始字节（可含多 mip / 多层）。
    pub bytes: Arc<[u8]>,
}

impl TextureData {
    /// 从 RGBA8（或同 stride 的未压缩）构造单 mip 2D 数据并校验长度。
    pub fn from_rgba8(width: u32, height: u32, rgba: impl Into<Arc<[u8]>>) -> Result<Self, SparkError> {
        Self::from_uncompressed(TextureFormat::Rgba8UnormSrgb, width, height, rgba)
    }

    /// 未压缩单 mip 2D。
    pub fn from_uncompressed(format: TextureFormat, width: u32, height: u32, bytes: impl Into<Arc<[u8]>>) -> Result<Self, SparkError> {
        if format.is_compressed() {
            return Err(SparkError::new(codes::texture_upload_invalid())
                .arg("reason", ErrorArg::String("compressed_use_explicit_layout".into())));
        }
        if width == 0 || height == 0 {
            return Err(SparkError::new(codes::texture_size_invalid())
                .arg("width", ErrorArg::Unsigned(width as u64))
                .arg("height", ErrorArg::Unsigned(height as u64)));
        }
        let bytes = bytes.into();
        let layout = TextureLayout::tightly_packed_2d(format, width, height);
        let need = layout.expected_tight_bytes(width, height).ok_or_else(|| {
            SparkError::new(codes::image_dimension_overflow())
                .arg("width", ErrorArg::Unsigned(width as u64))
                .arg("height", ErrorArg::Unsigned(height as u64))
        })?;
        if bytes.len() != need {
            return Err(SparkError::new(codes::texture_data_length_mismatch())
                .arg("expected", ErrorArg::Unsigned(need as u64))
                .arg("got", ErrorArg::Unsigned(bytes.len() as u64)));
        }
        Ok(Self { layout, bytes })
    }

    /// 相对 [`TextureDesc`] 做轻量一致性检查（单层、mip 偏移数量）。
    pub fn validate_against(&self, desc: &TextureDesc) -> Result<(), SparkError> {
        if self.layout.mip_offsets.len() as u32 != desc.mip_levels {
            return Err(SparkError::new(codes::texture_layout_invalid())
                .arg("mip_offsets", ErrorArg::Unsigned(self.layout.mip_offsets.len() as u64))
                .arg("mip_levels", ErrorArg::Unsigned(desc.mip_levels as u64)));
        }
        if self.layout.layer_offsets.len() as u32 != desc.depth_or_layers && desc.dimension != crate::format::TextureDimension::D3 {
            // D3 可用 depth 表达，层偏移可为 1。
            if self.layout.layer_offsets.len() != 1 && self.layout.layer_offsets.len() as u32 != desc.depth_or_layers {
                return Err(SparkError::new(codes::texture_layout_invalid())
                    .arg("layer_offsets", ErrorArg::Unsigned(self.layout.layer_offsets.len() as u64))
                    .arg("depth_or_layers", ErrorArg::Unsigned(desc.depth_or_layers as u64)));
            }
        }
        if self.bytes.is_empty() {
            return Err(SparkError::new(codes::texture_data_length_mismatch())
                .arg("expected", ErrorArg::Unsigned(1))
                .arg("got", ErrorArg::Unsigned(0)));
        }
        Ok(())
    }
}
