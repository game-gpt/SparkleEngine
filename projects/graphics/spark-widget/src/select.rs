//! 选择与图像控件。

use spark_core::{Color, Rect, Vec2};
use spark_renderer::TextureId;

use crate::accessibility::{AccessNode, Role};
use crate::response::Response;
use crate::ui::Ui;

impl Ui<'_> {
    /// 下拉选择。展开状态存在 [`crate::WidgetMemory::opened`]。
    pub fn combo_box(
        &mut self,
        salt: impl std::hash::Hash,
        options: &[&str],
        selected: &mut usize,
    ) -> Response {
        if options.is_empty() {
            return Response::empty(self.id_from(("combo_empty", salt)), self.available_rect());
        }
        *selected = (*selected).min(options.len() - 1);
        let height = self.theme.metrics.button_height;
        let rect = self.allocate(height, None);
        let id = self.id_from(("combo", salt));
        self.state.register_focusable(id, rect);
        let mut response = self.interact(id, rect, true);
        if self.keyboard_activate(id) {
            response.clicked = true;
        }
        if response.clicked {
            let open = self.state.memory_mut(id).opened;
            self.state.memory_mut(id).opened = !open;
        }
        let open = self.state.memory(id).map(|m| m.opened).unwrap_or(false);
        let fill = if response.hovered || open {
            self.theme.colors.primary_hover
        } else {
            self.theme.colors.primary
        };
        self.draw.fill_rect(rect, fill);
        self.draw.fill_rect(
            Rect::new(rect.x - 1.0, rect.y - 1.0, rect.w + 2.0, rect.h + 2.0),
            self.theme.colors.border,
        );
        let label = options[*selected];
        let size = self.theme.typography.label;
        self.draw.text(
            rect.x + 8.0,
            rect.y + (rect.h - size) * 0.5,
            size,
            self.theme.colors.text,
            label,
        );
        self.draw.text(
            rect.x + rect.w - 20.0,
            rect.y + (rect.h - size) * 0.5,
            size,
            self.theme.colors.text,
            if open { "▲" } else { "▼" },
        );

        if open {
            let row_h = self.theme.metrics.row_height;
            for (i, option) in options.iter().enumerate() {
                let opt = Rect::new(rect.x, rect.y + rect.h + i as f32 * row_h, rect.w, row_h);
                let opt_id = self.id_from(("combo_opt", id.raw(), i));
                let mut opt_response = self.interact(opt_id, opt, true);
                if self.keyboard_activate(opt_id) {
                    opt_response.clicked = true;
                }
                let on = i == *selected || opt_response.hovered;
                self.draw.fill_rect(
                    opt,
                    if on {
                        self.theme.colors.primary_hover
                    } else {
                        self.theme.colors.panel
                    },
                );
                self.draw.text(
                    opt.x + 8.0,
                    opt.y + (opt.h - size) * 0.5,
                    size,
                    self.theme.colors.text,
                    *option,
                );
                if opt_response.clicked {
                    if *selected != i {
                        *selected = i;
                        response.changed = true;
                    }
                    self.state.memory_mut(id).opened = false;
                    response.clicked = true;
                }
            }
            // 点击外部关闭：无捕获时若按下且不在 header/options 内
            if self.input().mouse_pressed(spark_input::MouseBtn::Left) && !response.hovered {
                let (mx, my) = self.input().mouse_pos();
                let p = Vec2::new(mx, my);
                let menu = Rect::new(
                    rect.x,
                    rect.y + rect.h,
                    rect.w,
                    row_h * options.len() as f32,
                );
                if !menu.contains(p) && !rect.contains(p) {
                    self.state.memory_mut(id).opened = false;
                }
            }
        }

        self.access(
            AccessNode::new(id, Role::Button)
                .label(label.to_string())
                .value(format!("{selected}"))
                .rect(rect)
                .expanded(open)
                .focusable(true)
                .focused(response.focused),
        );
        response
    }

    /// 分段控件：与 tabs 类似，但单行紧凑样式。
    pub fn segmented(&mut self, options: &[&str], selected: &mut usize) -> Response {
        self.tabs(options, selected)
    }

    /// 固定尺寸图像。UV 默认全图。
    pub fn image(&mut self, texture: TextureId, size: Vec2) -> Response {
        self.image_uv(
            texture,
            size,
            Rect::new(0.0, 0.0, 1.0, 1.0),
            Color::rgb(1.0, 1.0, 1.0),
        )
    }

    pub fn image_uv(
        &mut self,
        texture: TextureId,
        size: Vec2,
        uv: Rect,
        tint: Color,
    ) -> Response {
        let rect = self.allocate(size.y.max(1.0), Some(size.x.max(1.0)));
        let id = self.id_from(("image", texture.0));
        // 若交叉轴未拉伸，把宽限制到 size.x
        let dest = Rect::new(rect.x, rect.y, size.x.min(rect.w), size.y.min(rect.h));
        self.draw.tex_rect(texture, dest, uv, tint);
        self.access(AccessNode::new(id, Role::Image).rect(dest));
        Response::empty(id, dest)
    }
}
