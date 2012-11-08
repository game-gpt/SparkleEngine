//! 简单表格：表头 + 等行高单元格。

use spark_core::Rect;

use crate::accessibility::{AccessNode, Role};
use crate::layout::{Layout, LayoutCursor};
use crate::response::Response;
use crate::ui::Ui;

impl Ui<'_> {
    /// 表格。列宽均分；`cell(ui, row, col)` 在单元格布局作用域内绘制内容。
    /// 返回本帧被点击的 `(row, col)`（仅可交互子控件自行处理点击时也可忽略）。
    pub fn table(
        &mut self,
        salt: impl std::hash::Hash,
        headers: &[&str],
        row_count: usize,
        row_height: f32,
        mut cell: impl FnMut(&mut Self, usize, usize),
    ) -> Response {
        let cols = headers.len().max(1);
        let id = self.id_from(("table", salt));
        let header_h = self.theme.metrics.row_height;
        let row_h = row_height.max(self.theme.metrics.row_height);
        let body_h = row_count as f32 * row_h;
        let total_h = header_h + body_h;
        let bounds = self.allocate(total_h.max(header_h), None);
        let col_w = bounds.w / cols as f32;

        self.draw.fill_rect(bounds, self.theme.colors.panel);
        self.draw.fill_rect(
            Rect::new(bounds.x, bounds.y, bounds.w, header_h),
            self.theme.colors.panel_title,
        );

        let size = self.theme.typography.label;
        for (c, title) in headers.iter().enumerate() {
            let cell_r = Rect::new(bounds.x + c as f32 * col_w, bounds.y, col_w, header_h);
            self.draw.text(
                cell_r.x + 6.0,
                cell_r.y + (cell_r.h - size) * 0.5,
                size,
                self.theme.colors.text,
                *title,
            );
            if c + 1 < cols {
                self.draw.fill_rect(
                    Rect::new(cell_r.x + cell_r.w - 1.0, cell_r.y + 4.0, 1.0, cell_r.h - 8.0),
                    self.theme.colors.border,
                );
            }
        }

        for r in 0..row_count {
            let y = bounds.y + header_h + r as f32 * row_h;
            if r % 2 == 1 {
                self.draw.fill_rect(
                    Rect::new(bounds.x, y, bounds.w, row_h),
                    self.theme.colors.track,
                );
            }
            for c in 0..cols {
                let cell_r = Rect::new(bounds.x + c as f32 * col_w, y, col_w, row_h);
                self.layouts
                    .push(LayoutCursor::new(cell_r, Layout::vertical().gap(2.0)));
                self.scope(("table_cell", id.raw(), r, c), |ui| cell(ui, r, c));
                self.layouts.pop();
            }
        }

        self.access(
            AccessNode::new(id, Role::List)
                .label("table")
                .rect(bounds)
                .value(format!("{}x{}", row_count, cols)),
        );
        Response::empty(id, bounds)
    }
}
