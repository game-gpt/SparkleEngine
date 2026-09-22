//! 状态机参数。数值由游戏写入，条件由过渡读取。

/// 控制器参数的当前值。
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ParameterValue {
    /// 布尔开关，与 [`crate::AnimatorCondition::BoolEquals`] 配对。
    Bool(bool),
    /// 浮点标量，与 [`crate::AnimatorCondition::FloatGreater`] 配对。
    Float(f32),
    /// `true` 表示本帧触发，被过渡消耗后清掉。
    Trigger(bool),
}
