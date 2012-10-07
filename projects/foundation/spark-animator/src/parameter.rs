//! 状态机参数。数值由游戏写入，条件由过渡读取。

/// 控制器参数的当前值。
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ParameterValue {
    Bool(bool),
    Float(f32),
    /// `true` 表示本帧触发，被过渡消耗后清掉。
    Trigger(bool),
}
