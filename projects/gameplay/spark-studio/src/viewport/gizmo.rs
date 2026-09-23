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
/// 旋转环色。
const ROTATE_RING: Color = Color::rgb(0.55, 0.75, 0.95);
/// 缩放把手色。
const SCALE_HANDLE: Color = Color::rgb(0.90, 0.72, 0.28);

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

/// 在选中枢轴处绘制 Z 旋转环（2D 编辑器演示）。
pub fn paint_rotate_gizmo(draw: &mut DrawList, wx: f32, wy: f32, rect: spark_types::Rect, vp: &ViewportState) {
    let (cx, cy) = world_to_screen(wx, wy, rect, vp);
    let radius = 44.0 * vp.zoom;
    let thick = (2.5 * vp.zoom).max(1.0);
    let segments = 48usize;
    for i in 0..segments {
        let a = i as f32 / segments as f32 * std::f32::consts::TAU;
        let px = cx + radius * a.cos();
        let py = cy + radius * a.sin();
        draw.fill_rect(spark_types::Rect::new(px - thick * 0.5, py - thick * 0.5, thick, thick), ROTATE_RING);
    }
    let tick = 10.0 * vp.zoom;
    draw.fill_rect(spark_types::Rect::new(cx - thick * 0.5, cy - radius - tick, thick, tick + thick), ROTATE_RING);
}

/// 在选中框四角绘制 uniform 缩放把手。
pub fn paint_scale_gizmo(draw: &mut DrawList, wx: f32, wy: f32, scale_x: f32, scale_y: f32, rect: spark_types::Rect, vp: &ViewportState) {
    let (cx, cy) = world_to_screen(wx, wy, rect, vp);
    let w = 80.0 * scale_x.abs().max(0.1) * vp.zoom;
    let h = 56.0 * scale_y.abs().max(0.1) * vp.zoom;
    let handle = (8.0 * vp.zoom).max(4.0);
    let half = handle * 0.5;
    let corners = [(-w * 0.5, -h * 0.5), (w * 0.5, -h * 0.5), (-w * 0.5, h * 0.5), (w * 0.5, h * 0.5)];
    for (dx, dy) in corners {
        draw.fill_rect(spark_types::Rect::new(cx + dx - half, cy + dy - half, handle, handle), SCALE_HANDLE);
    }
}
