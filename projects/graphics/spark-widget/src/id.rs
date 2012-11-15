//! 稳定 Widget 标识。

/// 跨帧稳定的控件 ID。不得由每帧调用顺序临时生成。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct WidgetId(pub u64);

impl WidgetId {
    pub const ROOT: Self = Self(0);

    pub const fn raw(self) -> u64 {
        self.0
    }
}
