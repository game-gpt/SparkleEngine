//! 键鼠输入状态：引擎自有键码，**不**把窗口后端（如 winit）漏给游戏。
//!
//! 后端适配放在 `spark-renderer-wgpu` / `spark-napi`；本 crate 只认 `Key` / `MouseBtn`。
//!
//! 帧模型：每帧先 [`Input::begin_frame`] 清边沿与瞬时量，再由宿主喂入事件，
//! 游戏在帧内只读查询。

#![forbid(missing_docs)]
mod actions;

pub use actions::ActionMap;

use std::collections::HashSet;

/// 物理键（布局无关的常用集合；按需扩展）。
///
/// 字母键按 **US QWERTY 物理位置** 语义命名，不随输入法布局改码。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u16)]
pub enum Key {
    /// Escape。
    Escape,
    /// 回车 / Return。
    Enter,
    /// 空格。
    Space,
    /// Tab。
    Tab,
    /// 退格。
    Backspace,
    /// Delete（向前删）。
    Delete,
    /// Home。
    Home,
    /// End。
    End,
    /// 左方向键。
    Left,
    /// 右方向键。
    Right,
    /// 上方向键。
    Up,
    /// 下方向键。
    Down,
    /// 字母 A。
    A,
    /// 字母 B。
    B,
    /// 字母 C。
    C,
    /// 字母 D。
    D,
    /// 字母 E。
    E,
    /// 字母 F。
    F,
    /// 字母 G。
    G,
    /// 字母 H。
    H,
    /// 字母 I。
    I,
    /// 字母 J。
    J,
    /// 字母 K。
    K,
    /// 字母 L。
    L,
    /// 字母 M。
    M,
    /// 字母 N。
    N,
    /// 字母 O。
    O,
    /// 字母 P。
    P,
    /// 字母 Q。
    Q,
    /// 字母 R。
    R,
    /// 字母 S。
    S,
    /// 字母 T。
    T,
    /// 字母 U。
    U,
    /// 字母 V。
    V,
    /// 字母 W。
    W,
    /// 字母 X。
    X,
    /// 字母 Y。
    Y,
    /// 字母 Z。
    Z,
    /// 主键盘数字 0。
    Digit0,
    /// 主键盘数字 1。
    Digit1,
    /// 主键盘数字 2。
    Digit2,
    /// 主键盘数字 3。
    Digit3,
    /// 主键盘数字 4。
    Digit4,
    /// 主键盘数字 5。
    Digit5,
    /// 主键盘数字 6。
    Digit6,
    /// 主键盘数字 7。
    Digit7,
    /// 主键盘数字 8。
    Digit8,
    /// 主键盘数字 9。
    Digit9,
    /// F1。
    F1,
    /// F2。
    F2,
    /// F3。
    F3,
    /// F4。
    F4,
    /// F5。
    F5,
    /// F6。
    F6,
    /// F7。
    F7,
    /// F8。
    F8,
    /// F9。
    F9,
    /// F10。
    F10,
    /// F11。
    F11,
    /// F12。
    F12,
    /// 左 Shift。
    LShift,
    /// 右 Shift。
    RShift,
    /// 左 Ctrl。
    LCtrl,
    /// 右 Ctrl。
    RCtrl,
}

/// 鼠标按键。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum MouseBtn {
    /// 主键（通常左键）。
    Left,
    /// 次键（通常右键）。
    Right,
    /// 中键 / 滚轮按下。
    Middle,
}

/// 按键边沿（按下或抬起瞬时）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ButtonState {
    /// 按下沿：仅在从未按下→按下时记入 `pressed`。
    Pressed,
    /// 抬起沿：仅在曾按下→抬起时记入 `released`。
    Released,
}

/// 一帧可读的输入快照。
///
/// 持有态（`down`）跨帧保留；边沿集、鼠标增量、滚轮与文本在 [`begin_frame`](Self::begin_frame) 清空。
/// IME 预编辑串跨帧保留，直至空 Preedit 或 Commit。
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
    /// 本帧文本输入（IME 提交与按键产生的字符）。
    text: String,
    /// IME 预编辑串（跨帧保持，直至 Preedit 清空或 Commit）。
    composition: String,
    /// 预编辑内光标字节范围（与 winit 一致）；`None` 表示未知。
    composition_cursor: Option<(usize, usize)>,
}

impl Input {
    /// 帧初：清边沿、鼠标增量、滚轮与文本；保留 `down` / 鼠标持有态与 IME 预编辑。
    pub fn begin_frame(&mut self) {
        self.pressed.clear();
        self.released.clear();
        self.mouse_pressed.clear();
        self.mouse_released.clear();
        self.mouse_delta = (0.0, 0.0);
        self.wheel = 0.0;
        self.text.clear();
        // composition 跨帧保留，由 on_ime_preedit / on_ime_commit 更新。
    }

    /// 累加本帧滚轮增量；`dy` 单位与后端一致，正值朝上。
    pub fn on_wheel(&mut self, dy: f32) {
        self.wheel += dy;
    }

    /// 追加本帧文本输入。
    pub fn on_text(&mut self, text: impl AsRef<str>) {
        self.text.push_str(text.as_ref());
    }

    /// IME 预编辑更新。空串表示取消预编辑。
    ///
    /// `cursor` 为预编辑串内 UTF-8 **字节**起止；空串时强制清为 `None`。
    pub fn on_ime_preedit(&mut self, text: impl AsRef<str>, cursor: Option<(usize, usize)>) {
        self.composition.clear();
        self.composition.push_str(text.as_ref());
        self.composition_cursor = if self.composition.is_empty() { None } else { cursor };
    }

    /// IME 提交：清空预编辑并写入本帧文本。
    pub fn on_ime_commit(&mut self, text: impl AsRef<str>) {
        self.composition.clear();
        self.composition_cursor = None;
        self.on_text(text);
    }

    /// 当前 IME 预编辑串（可能为空）。
    pub fn composition(&self) -> &str {
        &self.composition
    }

    /// 预编辑光标的 UTF-8 字节范围；无预编辑或未知时为 `None`。
    pub fn composition_cursor(&self) -> Option<(usize, usize)> {
        self.composition_cursor
    }

    /// 累加本帧鼠标位移（设备像素）；在下次 `begin_frame` 前可读。
    pub fn on_mouse_delta(&mut self, dx: f32, dy: f32) {
        self.mouse_delta.0 += dx;
        self.mouse_delta.1 += dy;
    }

    /// 键盘边沿：重复 Pressed 不重复记入 `pressed`；未按下时的 Released 忽略。
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

    /// 鼠标键边沿，语义同 [`on_key`](Self::on_key)。
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

    /// 更新光标位置（逻辑像素或后端约定单位，由宿主保证一致）。
    pub fn on_cursor(&mut self, x: f32, y: f32) {
        self.mouse_pos = (x, y);
    }

    /// 键当前是否按住。
    pub fn key_down(&self, key: Key) -> bool {
        self.down.contains(&key)
    }

    /// 本帧是否刚按下（边沿）。
    pub fn key_pressed(&self, key: Key) -> bool {
        self.pressed.contains(&key)
    }

    /// 本帧是否刚抬起（边沿）。
    pub fn key_released(&self, key: Key) -> bool {
        self.released.contains(&key)
    }

    /// 本帧新按下的键（无序）。
    pub fn keys_pressed(&self) -> impl Iterator<Item = Key> + '_ {
        self.pressed.iter().copied()
    }

    /// 最近一次光标坐标 `(x, y)`。
    pub fn mouse_pos(&self) -> (f32, f32) {
        self.mouse_pos
    }

    /// 鼠标键是否按住。
    pub fn mouse_down(&self, button: MouseBtn) -> bool {
        self.mouse_down.contains(&button)
    }

    /// 本帧鼠标键是否刚按下。
    pub fn mouse_pressed(&self, button: MouseBtn) -> bool {
        self.mouse_pressed.contains(&button)
    }

    /// 本帧鼠标键是否刚抬起。
    pub fn mouse_released(&self, button: MouseBtn) -> bool {
        self.mouse_released.contains(&button)
    }

    /// 本帧累计鼠标增量（设备像素）。
    pub fn mouse_delta(&self) -> (f32, f32) {
        self.mouse_delta
    }

    /// 本帧累计滚轮；正值朝上。
    pub fn wheel(&self) -> f32 {
        self.wheel
    }

    /// 本帧累计的文本输入（不含控制键）。
    pub fn text(&self) -> &str {
        &self.text
    }
}
