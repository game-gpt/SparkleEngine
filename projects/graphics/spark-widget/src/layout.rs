//! 布局尺寸与线性作用域。

use spark_core::{Rect, Vec2};

/// 主轴方向。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Direction {
    #[default]
    Vertical,
    Horizontal,
}

/// 交叉轴对齐。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Align {
    #[default]
    Start,
    Center,
    End,
    Stretch,
}

/// 主轴分布。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Justify {
    #[default]
    Start,
    Center,
    End,
    SpaceBetween,
}

/// 单轴尺寸。
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Size {
    Auto,
    Px(f32),
    Percent(f32),
    Fill,
}

impl Default for Size {
    fn default() -> Self {
        Self::Auto
    }
}

impl Size {
    pub fn resolve(self, available: f32, content: f32) -> f32 {
        match self {
            Self::Auto | Self::Fill => available.max(0.0),
            Self::Px(px) => px.max(0.0),
            Self::Percent(p) => (available * p.clamp(0.0, 1.0)).max(0.0),
        }
        .max(content.min(available.max(0.0)))
    }
}

/// 四边内边距。
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Insets {
    pub left: f32,
    pub right: f32,
    pub top: f32,
    pub bottom: f32,
}

impl Insets {
    pub fn all(v: f32) -> Self {
        Self {
            left: v,
            right: v,
            top: v,
            bottom: v,
        }
    }

    pub fn symmetric(horizontal: f32, vertical: f32) -> Self {
        Self {
            left: horizontal,
            right: horizontal,
            top: vertical,
            bottom: vertical,
        }
    }

    pub fn horizontal(self) -> f32 {
        self.left + self.right
    }

    pub fn vertical(self) -> f32 {
        self.top + self.bottom
    }
}

/// 布局参数。
#[derive(Debug, Clone, Copy)]
pub struct Layout {
    pub direction: Direction,
    pub width: Size,
    pub height: Size,
    pub min_size: Vec2,
    pub max_size: Vec2,
    pub padding: Insets,
    pub gap: f32,
    pub align: Align,
    pub justify: Justify,
}

impl Default for Layout {
    fn default() -> Self {
        Self {
            direction: Direction::Vertical,
            width: Size::Fill,
            height: Size::Auto,
            min_size: Vec2::ZERO,
            max_size: Vec2::new(f32::INFINITY, f32::INFINITY),
            padding: Insets::default(),
            gap: 8.0,
            align: Align::Start,
            justify: Justify::Start,
        }
    }
}

impl Layout {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn vertical() -> Self {
        Self {
            direction: Direction::Vertical,
            ..Self::default()
        }
    }

    pub fn horizontal() -> Self {
        Self {
            direction: Direction::Horizontal,
            ..Self::default()
        }
    }

    pub fn width(mut self, width: Size) -> Self {
        self.width = width;
        self
    }

    pub fn height(mut self, height: Size) -> Self {
        self.height = height;
        self
    }

    pub fn padding(mut self, padding: impl Into<Insets>) -> Self {
        self.padding = padding.into();
        self
    }

    pub fn gap(mut self, gap: f32) -> Self {
        self.gap = gap.max(0.0);
        self
    }

    pub fn align(mut self, align: Align) -> Self {
        self.align = align;
        self
    }
}

impl From<f32> for Insets {
    fn from(value: f32) -> Self {
        Self::all(value)
    }
}

/// 当前布局游标：在可用矩形内顺序分配子矩形。
#[derive(Debug, Clone)]
pub struct LayoutCursor {
    pub layout: Layout,
    pub bounds: Rect,
    pub content: Rect,
    pub cursor: Vec2,
    pub line_cross: f32,
    /// 网格模式：按列填满后换行。
    pub grid: Option<GridState>,
}

/// 等宽列网格状态。
#[derive(Debug, Clone, Copy)]
pub struct GridState {
    pub columns: usize,
    pub gap: f32,
    pub row_height: f32,
    pub index: usize,
    pub cell_w: f32,
}

impl LayoutCursor {
    pub fn new(bounds: Rect, layout: Layout) -> Self {
        let content = Rect::new(
            bounds.x + layout.padding.left,
            bounds.y + layout.padding.top,
            (bounds.w - layout.padding.horizontal()).max(0.0),
            (bounds.h - layout.padding.vertical()).max(0.0),
        );
        Self {
            layout,
            bounds,
            content,
            cursor: Vec2::new(content.x, content.y),
            line_cross: 0.0,
            grid: None,
        }
    }

    pub fn grid(bounds: Rect, columns: usize, row_height: f32, gap: f32) -> Self {
        let columns = columns.max(1);
        let gap = gap.max(0.0);
        let content = bounds;
        let cell_w = if columns == 0 {
            content.w
        } else {
            ((content.w - gap * (columns.saturating_sub(1) as f32)) / columns as f32).max(1.0)
        };
        Self {
            layout: Layout::vertical().gap(gap),
            bounds,
            content,
            cursor: Vec2::new(content.x, content.y),
            line_cross: 0.0,
            grid: Some(GridState {
                columns,
                gap,
                row_height: row_height.max(1.0),
                index: 0,
                cell_w,
            }),
        }
    }

    /// 分配下一块。网格模式下忽略 `main`/`cross`，按单元格推进。
    pub fn allocate(&mut self, main: f32, cross: Option<f32>) -> Rect {
        if let Some(grid) = self.grid.as_mut() {
            let col = grid.index % grid.columns;
            let row = grid.index / grid.columns;
            let x = self.content.x + col as f32 * (grid.cell_w + grid.gap);
            let y = self.content.y + row as f32 * (grid.row_height + grid.gap);
            grid.index += 1;
            self.cursor.y = y + grid.row_height;
            return Rect::new(x, y, grid.cell_w, grid.row_height);
        }
        let gap = if self.has_placed() {
            self.layout.gap
        } else {
            0.0
        };
        match self.layout.direction {
            Direction::Vertical => {
                self.cursor.y += gap;
                let w = cross.unwrap_or(self.content.w).min(self.content.w).max(0.0);
                let x = match self.layout.align {
                    Align::Start | Align::Stretch => self.content.x,
                    Align::Center => self.content.x + (self.content.w - w) * 0.5,
                    Align::End => self.content.x + (self.content.w - w).max(0.0),
                };
                let rect = Rect::new(x, self.cursor.y, w, main.max(0.0));
                self.cursor.y += main.max(0.0);
                self.line_cross = self.line_cross.max(w);
                rect
            }
            Direction::Horizontal => {
                self.cursor.x += gap;
                let h = cross.unwrap_or(self.content.h).min(self.content.h).max(0.0);
                let y = match self.layout.align {
                    Align::Start | Align::Stretch => self.content.y,
                    Align::Center => self.content.y + (self.content.h - h) * 0.5,
                    Align::End => self.content.y + (self.content.h - h).max(0.0),
                };
                let rect = Rect::new(self.cursor.x, y, main.max(0.0), h);
                self.cursor.x += main.max(0.0);
                self.line_cross = self.line_cross.max(h);
                rect
            }
        }
    }

    pub fn remaining_main(&self) -> f32 {
        match self.layout.direction {
            Direction::Vertical => (self.content.y + self.content.h - self.cursor.y).max(0.0),
            Direction::Horizontal => (self.content.x + self.content.w - self.cursor.x).max(0.0),
        }
    }

    fn has_placed(&self) -> bool {
        match self.layout.direction {
            Direction::Vertical => self.cursor.y > self.content.y + 0.5,
            Direction::Horizontal => self.cursor.x > self.content.x + 0.5,
        }
    }
}
