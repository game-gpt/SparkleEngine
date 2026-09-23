//! 双摇杆向量读取：键盘 + 手柄 + 鼠标指向/增量。

use crate::{ActionMap, Input};

/// 移动绑定名（与 [`ActionMap`] 一致）。
#[derive(Debug, Clone, Copy)]
pub struct MoveBindings {
    /// 负 X 动作名。
    pub left: &'static str,
    /// 正 X。
    pub right: &'static str,
    /// 负 Y（屏上）。
    pub up: &'static str,
    /// 正 Y（屏下）。
    pub down: &'static str,
}

impl Default for MoveBindings {
    fn default() -> Self {
        Self { left: "move_left", right: "move_right", up: "move_up", down: "move_down" }
    }
}

/// 瞄准模式。
#[derive(Debug, Clone, Copy)]
pub enum AimMode {
    /// 从锚点指向光标（屏/世界像素，由调用方保证同空间）。
    Pointer {
        /// 锚点 X。
        anchor_x: f32,
        /// 锚点 Y。
        anchor_y: f32,
    },
    /// 指针锁定下的鼠标增量（右摇杆语义）。
    MouseDelta {
        /// 像素/弧度增益。
        sensitivity: f32,
    },
}

/// 对单轴应用径向死区后再线性重映射到 [-1, 1]。
pub fn apply_deadzone(v: f32, deadzone: f32) -> f32 {
    let dz = deadzone.clamp(0.0, 0.99);
    let a = v.abs();
    if a <= dz {
        return 0.0;
    }
    let sign = v.signum();
    sign * ((a - dz) / (1.0 - dz)).clamp(0.0, 1.0)
}

/// 对 `(x, y)` 应用圆形死区；零向量保持零。
pub fn normalize_stick(x: f32, y: f32, deadzone: f32) -> (f32, f32) {
    let dx = apply_deadzone(x, deadzone);
    let dy = apply_deadzone(y, deadzone);
    let len = (dx * dx + dy * dy).sqrt();
    if len <= 1.0e-4 {
        return (0.0, 0.0);
    }
    if len > 1.0 {
        return (dx / len, dy / len);
    }
    (dx, dy)
}

/// 读取移动向量：键盘轴优先；手柄左摇杆幅度更大时覆盖。
pub fn read_move(input: &Input, map: &ActionMap, bindings: &MoveBindings, deadzone: f32) -> (f32, f32) {
    let mut x = map.axis(input, bindings.left, bindings.right);
    let mut y = map.axis(input, bindings.up, bindings.down);
    if input.gamepad().connected {
        let (gx, gy) = input.gamepad().left_stick();
        let (sx, sy) = normalize_stick(gx, gy, deadzone);
        if sx.hypot(sy) > x.hypot(y) {
            x = sx;
            y = sy;
        }
    }
    normalize_stick(x, y, 0.0)
}

/// 读取瞄准方向；优先右摇杆，其次鼠标模式。返回单位向量或 `(0,0)`。
pub fn read_aim(input: &Input, mode: AimMode, stick_deadzone: f32) -> (f32, f32) {
    if input.gamepad().connected {
        let (rx, ry) = input.gamepad().right_stick();
        let (sx, sy) = normalize_stick(rx, ry, stick_deadzone);
        let len = sx.hypot(sy);
        if len > 0.2 {
            return (sx / len.max(1.0e-4), sy / len.max(1.0e-4));
        }
    }
    match mode {
        AimMode::Pointer { anchor_x, anchor_y } => {
            let (mx, my) = input.mouse_pos();
            let dx = mx - anchor_x;
            let dy = my - anchor_y;
            let len = dx.hypot(dy);
            if len <= 1.0e-3 {
                return (1.0, 0.0);
            }
            (dx / len, dy / len)
        }
        AimMode::MouseDelta { sensitivity } => {
            let (dx, dy) = input.mouse_delta();
            let sx = dx * sensitivity;
            let sy = dy * sensitivity;
            let len = sx.hypot(sy);
            if len <= 1.0e-3 {
                return (1.0, 0.0);
            }
            (sx / len, sy / len)
        }
    }
}
