//! 立即模式 Widget。界面控件叫 Widget，避免与 ECS `Component` 混淆。

use spark_core::{Color, Rect, Vec2};
use spark_input::{Input, MouseBtn};
use spark_renderer::DrawList;

/// 面板背景。
pub fn panel(draw: &mut DrawList, rect: Rect, fill: Color) {
    draw.fill_rect(rect, fill);
}

/// 标签文字。
pub fn label(draw: &mut DrawList, x: f32, y: f32, size: f32, color: Color, text: &str) {
    draw.text(x, y, size, color, text);
}

/// 按钮。返回本帧是否点击。
pub fn button(draw: &mut DrawList, input: &Input, rect: Rect, text: &str) -> bool {
    let (mx, my) = input.mouse_pos();
    let hovered = rect.contains(Vec2::new(mx, my));
    let pressed = hovered && input.mouse_down(MouseBtn::Left);
    let clicked = hovered && input.mouse_pressed(MouseBtn::Left);

    let fill = if pressed {
        Color::rgb(0.20, 0.45, 0.75)
    } else if hovered {
        Color::rgb(0.18, 0.35, 0.58)
    } else {
        Color::rgb(0.12, 0.22, 0.38)
    };
    let border = Color::rgb(0.45, 0.70, 0.95);
    draw.fill_rect(
        Rect::new(rect.x - 2.0, rect.y - 2.0, rect.w + 4.0, rect.h + 4.0),
        border,
    );
    draw.fill_rect(rect, fill);

    let size = 22.0;
    let est_w = text.chars().count() as f32 * size * 0.55;
    let tx = rect.x + (rect.w - est_w).max(0.0) * 0.5;
    let ty = rect.y + (rect.h - size) * 0.5;
    draw.text(tx, ty, size, Color::rgb(0.92, 0.96, 1.0), text);

    clicked
}
