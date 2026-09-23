//! 场景操作工具（与工具栏互斥）。

/// 场景操作工具（与工具栏互斥）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Tool {
    /// 平移视口。
    Hand,
    /// 平移选中实体。
    Move,
    /// 旋转选中实体。
    Rotate,
    /// 缩放选中实体。
    Scale,
}
