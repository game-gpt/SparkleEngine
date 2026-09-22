//! 动态 uniform 槽：同帧多 draw 不得反复 `write_buffer` 覆盖同一偏移。

use std::num::NonZeroU64;

/// 按设备 `min_uniform_buffer_offset_alignment` 对齐的槽步长。
pub fn uniform_stride(device: &wgpu::Device, element_size: u64) -> u64 {
    let align = u64::from(device.limits().min_uniform_buffer_offset_alignment.max(16));
    element_size.div_ceil(align) * align
}

pub fn binding_size(element_size: u64) -> Option<NonZeroU64> {
    NonZeroU64::new(element_size)
}
