//! 动作名到键鼠的绑定。具体哪套绑定属于游戏。

use std::collections::HashMap;

use crate::{Input, Key, MouseBtn};

/// 一个动作可以绑多颗键或鼠标键，任一成立即成立。
#[derive(Debug, Clone, Default)]
pub struct ActionMap {
    keys: HashMap<String, Vec<Key>>,
    mouse: HashMap<String, Vec<MouseBtn>>,
}

impl ActionMap {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn bind_key(&mut self, action: impl Into<String>, key: Key) -> &mut Self {
        self.keys.entry(action.into()).or_default().push(key);
        self
    }

    pub fn bind_mouse(&mut self, action: impl Into<String>, button: MouseBtn) -> &mut Self {
        self.mouse.entry(action.into()).or_default().push(button);
        self
    }

    pub fn down(&self, input: &Input, action: &str) -> bool {
        self.keys.get(action).is_some_and(|keys| keys.iter().any(|k| input.key_down(*k)))
            || self.mouse.get(action).is_some_and(|btns| btns.iter().any(|b| input.mouse_down(*b)))
    }

    pub fn pressed(&self, input: &Input, action: &str) -> bool {
        self.keys.get(action).is_some_and(|keys| keys.iter().any(|k| input.key_pressed(*k)))
            || self.mouse.get(action).is_some_and(|btns| btns.iter().any(|b| input.mouse_pressed(*b)))
    }

    pub fn released(&self, input: &Input, action: &str) -> bool {
        self.keys.get(action).is_some_and(|keys| keys.iter().any(|k| input.key_released(*k)))
            || self.mouse.get(action).is_some_and(|btns| btns.iter().any(|b| input.mouse_released(*b)))
    }

    /// 负方向与正方向合成的一维轴，结果在 -1、0、1 之间。
    pub fn axis(&self, input: &Input, negative: &str, positive: &str) -> f32 {
        let mut v = 0.0;
        if self.down(input, negative) {
            v -= 1.0;
        }
        if self.down(input, positive) {
            v += 1.0;
        }
        v
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ButtonState;

    #[test]
    fn axis_and_edge_follow_bindings() {
        let mut map = ActionMap::new();
        map.bind_key("left", Key::A).bind_key("left", Key::Left);
        map.bind_key("right", Key::D);
        map.bind_mouse("use", MouseBtn::Left);

        let mut input = Input::default();
        input.on_key(Key::A, ButtonState::Pressed);
        input.on_mouse_button(MouseBtn::Left, ButtonState::Pressed);
        assert!(map.down(&input, "left"));
        assert!(map.pressed(&input, "use"));
        assert!((map.axis(&input, "left", "right") + 1.0).abs() < 1e-5);

        input.begin_frame();
        input.on_key(Key::A, ButtonState::Released);
        assert!(map.released(&input, "left"));
        assert!(!map.down(&input, "left"));
    }
}
