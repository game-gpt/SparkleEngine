//! 手柄抽象：轴与键独立于 winit / gilrs，由宿主每帧写入 [`crate::Input`]。

use std::collections::HashSet;

use crate::ButtonState;

/// 标准布局轴（左/右摇杆 XY）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum GamepadAxis {
    /// 左摇杆 X，左负右正。
    LeftX,
    /// 左摇杆 Y，上负下正（与屏幕 Y 一致）。
    LeftY,
    /// 右摇杆 X。
    RightX,
    /// 右摇杆 Y。
    RightY,
}

impl GamepadAxis {
    /// 轴在 [`GamepadFrame::axes`] 中的下标。
    pub const fn index(self) -> usize {
        match self {
            Self::LeftX => 0,
            Self::LeftY => 1,
            Self::RightX => 2,
            Self::RightY => 3,
        }
    }
}

/// 常用面键（南西北东 + 肩键）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum GamepadButton {
    /// 南（Xbox A / PS Cross）。
    South,
    /// 东（Xbox B / PS Circle）。
    East,
    /// 北（Xbox Y / PS Triangle）。
    North,
    /// 西（Xbox X / PS Square）。
    West,
    /// 左肩键。
    LeftShoulder,
    /// 右肩键。
    RightShoulder,
    /// 左扳机（视为数字键，阈值由宿主决定）。
    LeftTrigger,
    /// 右扳机。
    RightTrigger,
    /// Start / Options。
    Start,
    /// Select / Share。
    Select,
}

/// 单帧手柄快照（可随 [`crate::Input::begin_frame`] 清边沿）。
#[derive(Debug, Clone, Default)]
pub struct GamepadFrame {
    /// 是否已连接（由宿主在连接/断开时设置）。
    pub connected: bool,
    /// 四轴：LX, LY, RX, RY，范围约 [-1, 1]。
    pub axes: [f32; 4],
    down: HashSet<GamepadButton>,
    pressed: HashSet<GamepadButton>,
    released: HashSet<GamepadButton>,
}

impl GamepadFrame {
    /// 帧初：清边沿，保留轴与 down（轴由宿主重写）。
    pub fn begin_frame(&mut self) {
        self.pressed.clear();
        self.released.clear();
    }

    /// 写入轴值；NaN 视为 0。
    pub fn set_axis(&mut self, axis: GamepadAxis, value: f32) {
        self.axes[axis.index()] = if value.is_finite() { value.clamp(-1.0, 1.0) } else { 0.0 };
    }

    /// 读取轴。
    pub fn axis(&self, axis: GamepadAxis) -> f32 {
        self.axes[axis.index()]
    }

    /// 键边沿，语义同 [`crate::Input::on_key`]。
    pub fn on_button(&mut self, button: GamepadButton, state: ButtonState) {
        match state {
            ButtonState::Pressed => {
                if self.down.insert(button) {
                    self.pressed.insert(button);
                }
            }
            ButtonState::Released => {
                if self.down.remove(&button) {
                    self.released.insert(button);
                }
            }
        }
    }

    /// 键是否按住。
    pub fn button_down(&self, button: GamepadButton) -> bool {
        self.down.contains(&button)
    }

    /// 本帧刚按下。
    pub fn button_pressed(&self, button: GamepadButton) -> bool {
        self.pressed.contains(&button)
    }

    /// 本帧刚抬起。
    pub fn button_released(&self, button: GamepadButton) -> bool {
        self.released.contains(&button)
    }

    /// 左摇杆 `(x, y)`。
    pub fn left_stick(&self) -> (f32, f32) {
        (self.axis(GamepadAxis::LeftX), self.axis(GamepadAxis::LeftY))
    }

    /// 右摇杆 `(x, y)`。
    pub fn right_stick(&self) -> (f32, f32) {
        (self.axis(GamepadAxis::RightX), self.axis(GamepadAxis::RightY))
    }
}
