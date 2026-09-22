//! 尺寸与约束。

/// 单轴尺寸意图：自动、像素、百分比、填满或内容驱动。
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Size {
    /// 由布局引擎按内容与约束决定。
    Auto,
    /// 固定逻辑像素。
    Px(f32),
    /// 相对父内容区百分比（`0..=100` 语义由引擎解释）。
    Percent(f32),
    /// 占满主轴剩余空间。
    Fill,
    /// 按最小内容尺寸。
    MinContent,
    /// 按最大内容尺寸。
    MaxContent,
}

impl Default for Size {
    fn default() -> Self {
        Self::Auto
    }
}

/// 二维具体尺寸（布局中间结果与约束上下界）。
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct Size2 {
    /// 宽度（逻辑像素）。
    pub width: f32,
    /// 高度（逻辑像素）。
    pub height: f32,
}

impl Size2 {
    /// 构造宽高。
    pub const fn new(width: f32, height: f32) -> Self {
        Self { width, height }
    }

    /// 将宽高钳制到给定约束的 `[min, max]`。
    pub fn clamp(self, constraints: Constraints) -> Self {
        Self {
            width: self.width.clamp(constraints.min.width, constraints.max.width),
            height: self.height.clamp(constraints.min.height, constraints.max.height),
        }
    }
}

/// 父节点传给子节点的布局约束盒。
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Constraints {
    /// 最小可接受尺寸。
    pub min: Size2,
    /// 最大可接受尺寸（可含无穷）。
    pub max: Size2,
}

impl Default for Constraints {
    fn default() -> Self {
        Self { min: Size2::default(), max: Size2 { width: f32::INFINITY, height: f32::INFINITY } }
    }
}

impl Constraints {
    /// 强制刚好等于 `size`（min == max）。
    pub fn tight(size: Size2) -> Self {
        Self { min: size, max: size }
    }

    /// 最小为 0、最大为 `max` 的松约束。
    pub fn loose(max: Size2) -> Self {
        Self { min: Size2::default(), max }
    }

    /// 收紧最大宽度（不低于已有 `min.width`）。
    pub fn with_max_width(mut self, width: f32) -> Self {
        self.max.width = self.max.width.min(width).max(self.min.width);
        self
    }

    /// 收紧最大高度（不低于已有 `min.height`）。
    pub fn with_max_height(mut self, height: f32) -> Self {
        self.max.height = self.max.height.min(height).max(self.min.height);
        self
    }

    /// 扣除内边距后的子约束（padding / border 内可用区）。
    pub fn deflate(self, insets: super::Insets) -> Self {
        let width = (self.max.width - insets.horizontal()).max(0.0);
        let height = (self.max.height - insets.vertical()).max(0.0);
        let min_w = self.min.width.min(width);
        let min_h = self.min.height.min(height);
        Self { min: Size2::new(min_w, min_h), max: Size2::new(width, height) }
    }
}

/// 交叉轴对齐（Flex 子项在交叉轴上的位置）。
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum Align {
    /// 靠交叉轴起点。
    #[default]
    Start,
    /// 居中。
    Center,
    /// 靠交叉轴终点。
    End,
    /// 拉伸填满交叉轴。
    Stretch,
}

/// 主轴分布（Flex 子项在主轴上的排布）。
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum Justify {
    /// 靠主轴起点堆积。
    #[default]
    Start,
    /// 整体居中。
    Center,
    /// 靠主轴终点堆积。
    End,
    /// 两端对齐，中间等分空隙。
    SpaceBetween,
}

/// Flex 主轴方向。
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum FlexDirection {
    /// 纵向（默认，子项自上而下）。
    #[default]
    Column,
    /// 横向（子项自左而右）。
    Row,
}
