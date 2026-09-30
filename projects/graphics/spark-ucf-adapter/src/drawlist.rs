//! 将 UCF 计算结果排队进 [`spark_renderer::DrawList`]（不经 ECS）。

use spark_renderer::{DrawList, TextureId};
use ucf::SchedulerError;

use crate::executor::{SparkComputePass, SparkUcfExecutor};

/// UCF CPU Fill → RGBA8 → `DrawList::create_texture`。
pub fn enqueue_color_fill_texture(
    draw: &mut DrawList,
    exec: &mut SparkUcfExecutor,
    pass: &SparkComputePass,
) -> Result<TextureId, SchedulerError> {
    let rgba = exec.compute_color_fill_rgba8(pass)?;
    draw.create_texture(pass.width, pass.height, rgba)
        .map_err(|e| SchedulerError::Backend("drawlist".into(), e.to_string()))
}
