//! 手柄泵占位：轴/键类型在 [`spark_input`]，后端可在网络可用时接入 gilrs。

use spark_input::Input;

/// 轮询手柄并写入 `input`（当前为 no-op，保留接口）。
pub struct GamepadPump;

impl GamepadPump {
    /// 构造。
    pub fn new() -> Self {
        Self
    }

    /// 每帧在 `Input::begin_frame` 之前调用。
    pub fn poll(&mut self, _input: &mut Input) {}
}
