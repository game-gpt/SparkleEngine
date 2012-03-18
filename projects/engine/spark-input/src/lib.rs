//! 键鼠输入状态：引擎自有键码，**不**把窗口后端（如 winit）漏给游戏。
//!
//! 后端适配放在 `spark-render` / 将来的 `spark-app`；本 crate 只认 `Key` / `MouseBtn`。

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
}

impl Input {
    pub fn begin_frame(&mut self) {
        self.pressed.clear();
        self.released.clear();
        self.mouse_pressed.clear();
        self.mouse_released.clear();
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

    pub fn mouse_pos(&self) -> (f32, f32) {
        self.mouse_pos
    }

    pub fn mouse_down(&self, button: MouseBtn) -> bool {
        self.mouse_down.contains(&button)
    }

    pub fn mouse_pressed(&self, button: MouseBtn) -> bool {
        self.mouse_pressed.contains(&button)
    }
}
