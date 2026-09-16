//! 键鼠输入状态。由窗口事件驱动，供游戏每帧读取。

use std::collections::HashSet;

use winit::event::{ElementState, MouseButton};
use winit::keyboard::{KeyCode, PhysicalKey};

/// 一帧可读的输入快照。
#[derive(Debug, Default, Clone)]
pub struct Input {
    down: HashSet<KeyCode>,
    pressed: HashSet<KeyCode>,
    released: HashSet<KeyCode>,
    mouse_pos: (f32, f32),
    mouse_down: HashSet<MouseButton>,
    mouse_pressed: HashSet<MouseButton>,
    mouse_released: HashSet<MouseButton>,
}

impl Input {
    pub fn begin_frame(&mut self) {
        self.pressed.clear();
        self.released.clear();
        self.mouse_pressed.clear();
        self.mouse_released.clear();
    }

    pub fn on_key(&mut self, key: PhysicalKey, state: ElementState) {
        let PhysicalKey::Code(code) = key else {
            return;
        };
        match state {
            ElementState::Pressed => {
                if self.down.insert(code) {
                    self.pressed.insert(code);
                }
            }
            ElementState::Released => {
                if self.down.remove(&code) {
                    self.released.insert(code);
                }
            }
        }
    }

    pub fn on_mouse_button(&mut self, button: MouseButton, state: ElementState) {
        match state {
            ElementState::Pressed => {
                if self.mouse_down.insert(button) {
                    self.mouse_pressed.insert(button);
                }
            }
            ElementState::Released => {
                if self.mouse_down.remove(&button) {
                    self.mouse_released.insert(button);
                }
            }
        }
    }

    pub fn on_cursor(&mut self, x: f32, y: f32) {
        self.mouse_pos = (x, y);
    }

    pub fn key_down(&self, code: KeyCode) -> bool {
        self.down.contains(&code)
    }

    pub fn key_pressed(&self, code: KeyCode) -> bool {
        self.pressed.contains(&code)
    }

    pub fn mouse_pos(&self) -> (f32, f32) {
        self.mouse_pos
    }

    pub fn mouse_down(&self, button: MouseButton) -> bool {
        self.mouse_down.contains(&button)
    }

    pub fn mouse_pressed(&self, button: MouseButton) -> bool {
        self.mouse_pressed.contains(&button)
    }
}
