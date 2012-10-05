//! 键鼠输入状态：引擎自有键码，**不**把窗口后端（如 winit）漏给游戏。
//!
//! 后端适配放在 `spark-renderer-wgpu` / `spark-napi`；本 crate 只认 `Key` / `MouseBtn`。

mod actions;

pub use actions::ActionMap;

use std::collections::HashSet;

/// 物理键（布局无关的常用集合；按需扩展）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u16)]
pub enum Key {
    Escape,
    Enter,
    Space,
    Tab,
    Backspace,
    Left,
    Right,
    Up,
    Down,
    A,
    B,
    C,
    D,
    E,
    F,
    G,
    H,
    I,
    J,
    K,
    L,
    M,
    N,
    O,
    P,
    Q,
    R,
    S,
    T,
    U,
    V,
    W,
    X,
    Y,
    Z,
    Digit0,
    Digit1,
    Digit2,
    Digit3,
    Digit4,
    Digit5,
    Digit6,
    Digit7,
    Digit8,
    Digit9,
    F1,
    F2,
    F3,
    F4,
    F5,
    F6,
    F7,
    F8,
    F9,
    F10,
    F11,
    F12,
    LShift,
    RShift,
}

/// 鼠标按键。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum MouseBtn {
    Left,
    Right,
    Middle,
}

/// 按键边沿。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ButtonState {
    Pressed,
    Released,
}

/// 一帧可读的输入快照。
#[derive(Debug, Default, Clone)]
pub struct Input {
    down: HashSet<Key>,
    pressed: HashSet<Key>,
    released: HashSet<Key>,
    mouse_pos: (f32, f32),
    mouse_down: HashSet<MouseBtn>,
    mouse_pressed: HashSet<MouseBtn>,
    mouse_released: HashSet<MouseBtn>,
    /// 本帧鼠标增量（设备像素，后端在 `begin_frame` 前写入，帧末清零）。
    mouse_delta: (f32, f32),
    /// 本帧滚轮。正值朝上（远离用户）。
    wheel: f32,
}

impl Input {
    pub fn begin_frame(&mut self) {
        self.pressed.clear();
        self.released.clear();
        self.mouse_pressed.clear();
        self.mouse_released.clear();
        self.mouse_delta = (0.0, 0.0);
        self.wheel = 0.0;
    }

    pub fn on_wheel(&mut self, dy: f32) {
        self.wheel += dy;
    }

    pub fn on_mouse_delta(&mut self, dx: f32, dy: f32) {
        self.mouse_delta.0 += dx;
        self.mouse_delta.1 += dy;
    }

    pub fn on_key(&mut self, key: Key, state: ButtonState) {
        match state {
            ButtonState::Pressed => {
                if self.down.insert(key) {
                    self.pressed.insert(key);
                }
            }
            ButtonState::Released => {
                if self.down.remove(&key) {
                    self.released.insert(key);
                }
            }
        }
    }

    pub fn on_mouse_button(&mut self, button: MouseBtn, state: ButtonState) {
        match state {
            ButtonState::Pressed => {
                if self.mouse_down.insert(button) {
                    self.mouse_pressed.insert(button);
                }
            }
            ButtonState::Released => {
                if self.mouse_down.remove(&button) {
                    self.mouse_released.insert(button);
                }
            }
        }
    }

    pub fn on_cursor(&mut self, x: f32, y: f32) {
        self.mouse_pos = (x, y);
    }

    pub fn key_down(&self, key: Key) -> bool {
        self.down.contains(&key)
    }

    pub fn key_pressed(&self, key: Key) -> bool {
        self.pressed.contains(&key)
    }

    pub fn key_released(&self, key: Key) -> bool {
        self.released.contains(&key)
    }

    pub fn mouse_pos(&self) -> (f32, f32) {
        self.mouse_pos
    }

    pub fn mouse_down(&self, button: MouseBtn) -> bool {
        self.mouse_down.contains(&button)
    }

    pub fn mouse_pressed(&self, button: MouseBtn) -> bool {
        self.mouse_pressed.contains(&button)
    }

    pub fn mouse_released(&self, button: MouseBtn) -> bool {
        self.mouse_released.contains(&button)
    }

    pub fn mouse_delta(&self) -> (f32, f32) {
        self.mouse_delta
    }

    pub fn wheel(&self) -> f32 {
        self.wheel
    }
}
