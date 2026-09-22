//! 状态之间的过渡。条件全部成立才开始。

/// 一条过渡上的条件。多个条件是与关系。
#[derive(Debug, Clone, PartialEq)]
pub enum AnimatorCondition {
    /// 布尔参数等于给定值时成立。
    BoolEquals {
        /// 参数名。
        parameter: String,
        /// 期望的布尔值。
        value: bool,
    },
    /// 浮点参数**严格大于**给定阈值时成立。
    FloatGreater {
        /// 参数名。
        parameter: String,
        /// 比较阈值。
        value: f32,
    },
    /// 触发器当前为 `true` 时成立（匹配后会被消耗）。
    Trigger {
        /// 触发器参数名。
        parameter: String,
    },
}

/// 从 `from` 到 `to`。`duration` 为 0 时立即切换。
#[derive(Debug, Clone, PartialEq)]
pub struct AnimatorTransition {
    /// 源状态名。
    pub from: String,
    /// 目标状态名。
    pub to: String,
    /// 过渡时长，单位秒；`≤ 0` 视为立即切入。
    pub duration: f32,
    /// 须全部满足的条件（空表示无条件即可切）。
    pub conditions: Vec<AnimatorCondition>,
}

impl AnimatorTransition {
    /// 无条件过渡；`duration` 会被钳到非负。
    pub fn new(from: impl Into<String>, to: impl Into<String>, duration: f32) -> Self {
        Self { from: from.into(), to: to.into(), duration: duration.max(0.0), conditions: Vec::new() }
    }

    /// 追加一条与条件。
    pub fn when(mut self, condition: AnimatorCondition) -> Self {
        self.conditions.push(condition);
        self
    }
}
