//! 场景视口 Move 工具 Gizmo（`DrawList` HUD）。

use spark_renderer::DrawList;
use spark_types::Color;

use crate::state::ViewportState;

use super::camera::world_to_screen;

/// X 轴把手色。
const AXIS_X: Color = Color::rgb(0.85, 0.28, 0.28);
/// Y 轴把手色。
const AXIS_Y: Color = Color::rgb(0.28, 0.72, 0.38);
/// 中心枢轴色。
const PIVOT: Color = Color::rgb(0.95, 0.95, 0.95);

const AXIS_LEN: f32 = 56.0;
const AXIS_THICK: f32 = 3.0;
const HEAD: f32 = 8.0;
const PIVOT_SIZE: f32 = 7.0;

/// 在选中枢轴处绘制 XY 平移 Gizmo。
pub fn paint_move_gizmo(draw: &mut DrawList, wx: f32, wy: f32, rect: spark_types::Rect, vp: &ViewportState) {
    let (cx, cy) = world_to_screen(wx, wy, rect, vp);
    let len = AXIS_LEN * vp.zoom;
    let thick = AXIS_THICK.max(1.0);
    let head = HEAD * vp.zoom;

    draw.fill_rect(spark_types::Rect::new(cx - len, cy - thick * 0.5, len, thick), AXIS_X);
    draw.fill_rect(spark_types::Rect::new(cx - head, cy - head * 0.5, head, head), AXIS_X);

    draw.fill_rect(spark_types::Rect::new(cx - thick * 0.5, cy - len, thick, len), AXIS_Y);
    draw.fill_rect(spark_types::Rect::new(cx - head * 0.5, cy - len, head, head), AXIS_Y);

    let half = PIVOT_SIZE * 0.5;
    draw.fill_rect(spark_types::Rect::new(cx - half, cy - half, PIVOT_SIZE, PIVOT_SIZE), PIVOT);
}
