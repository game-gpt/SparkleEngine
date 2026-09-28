//! 场景视口网格与世界原点绘制（`UiRenderBatch`，叠在 Widget 之下）。

use spark_renderer::UiRenderBatch;
use spark_types::{Color, Rect};

use crate::state::{Tool, TransformState, ViewportState};

use super::camera::{viewport_center, world_to_screen};
use super::gizmo::{paint_move_gizmo, paint_rotate_gizmo, paint_scale_gizmo};

/// 场景画布底色。
const CANVAS: Color = Color::rgb(0.055, 0.060, 0.068);
/// 次网格线。
const GRID_MINOR: Color = Color::rgb(0.10, 0.11, 0.13);
/// 主网格线（每 8 格）。
const GRID_MAJOR: Color = Color::rgb(0.16, 0.17, 0.20);
/// 世界 X 轴。
const AXIS_X: Color = Color::rgb(0.55, 0.22, 0.22);
/// 世界 Y 轴。
const AXIS_Y: Color = Color::rgb(0.22, 0.48, 0.30);
/// 相机框描边。
const CAMERA_FRAME: Color = Color::rgb(0.35, 0.55, 0.75);
/// 选中对象占位框。
const SELECTION: Color = Color::rgb(0.286, 0.475, 0.655);

const GRID_STEP: f32 = 24.0;

/// 在视口矩形内绘制场景网格、原点十字、相机框与选中占位。
pub fn paint_scene_viewport(batch: &mut UiRenderBatch, rect: Rect, vp: &ViewportState, tool: Tool, selection: Option<&TransformState>) {
    if rect.w < 1.0 || rect.h < 1.0 {
        return;
    }

    batch.push_clip(rect);
    batch.fill_rect(rect, CANVAS);

    let (origin_x, origin_y) = viewport_center(rect, vp);
    let step = GRID_STEP * vp.zoom.max(0.01);

    let x_start = rect.x + (origin_x - rect.x).rem_euclid(step);
    let mut x = x_start;
    while x < rect.x + rect.w {
        let idx = ((x - origin_x) / step).round() as i32;
        let color = if idx == 0 {
            AXIS_Y
        }
        else if idx.rem_euclid(8) == 0 {
            GRID_MAJOR
        }
        else {
            GRID_MINOR
        };
        batch.fill_rect(Rect::new(x, rect.y, 1.0, rect.h), color);
        x += step;
    }

    let y_start = rect.y + (origin_y - rect.y).rem_euclid(step);
    let mut y = y_start;
    while y < rect.y + rect.h {
        let idx = ((y - origin_y) / step).round() as i32;
        let color = if idx == 0 {
            AXIS_X
        }
        else if idx.rem_euclid(8) == 0 {
            GRID_MAJOR
        }
        else {
            GRID_MINOR
        };
        batch.fill_rect(Rect::new(rect.x, y, rect.w, 1.0), color);
        y += step;
    }

    let cam_w = rect.w.min(220.0);
    let cam_h = rect.h.min(140.0);
    let cam_x = origin_x - cam_w * 0.5;
    let cam_y = origin_y - cam_h * 0.5;
    stroke_rect(batch, Rect::new(cam_x, cam_y, cam_w, cam_h), CAMERA_FRAME);

    if let Some(t) = selection {
        let (sx, sy) = world_to_screen(t.pos_x, t.pos_y, rect, vp);
        let w = 80.0 * t.scale_x.abs().max(0.1) * vp.zoom;
        let h = 56.0 * t.scale_y.abs().max(0.1) * vp.zoom;
        stroke_rect(batch, Rect::new(sx - w * 0.5, sy - h * 0.5, w, h), SELECTION);
        if tool == Tool::Move {
            paint_move_gizmo(batch, t.pos_x, t.pos_y, rect, vp);
        }
        else if tool == Tool::Rotate {
            paint_rotate_gizmo(batch, t.pos_x, t.pos_y, rect, vp);
        }
        else if tool == Tool::Scale {
            paint_scale_gizmo(batch, t.pos_x, t.pos_y, t.scale_x, t.scale_y, rect, vp);
        }
    }

    let zoom_pct = (vp.zoom * 100.0).round();
    batch.text(rect.x + 8.0, rect.y + 6.0, 11.0, Color::rgb(0.67, 0.69, 0.72), format!("世界原点 (0, 0) · {zoom_pct}%"));

    batch.pop_clip();
}

fn stroke_rect(batch: &mut UiRenderBatch, rect: Rect, color: Color) {
    batch.fill_rect(Rect::new(rect.x, rect.y, rect.w, 1.0), color);
    batch.fill_rect(Rect::new(rect.x, rect.y + rect.h - 1.0, rect.w, 1.0), color);
    batch.fill_rect(Rect::new(rect.x, rect.y, 1.0, rect.h), color);
    batch.fill_rect(Rect::new(rect.x + rect.w - 1.0, rect.y, 1.0, rect.h), color);
}
