//! 树形列表与游戏通用 HUD 控件。

use spark_core::{Color, Rect, Vec2};
use spark_input::Key;

use crate::accessibility::{AccessNode, Role};
use crate::response::Response;
use crate::ui::Ui;

/// 树节点：`children` 非空时可展开。
#[derive(Debug, Clone)]
pub struct TreeNode {
    pub id: u64,
    pub label: String,
    pub children: Vec<TreeNode>,
}

impl Ui<'_> {
    /// 树形列表。`expanded` 记录已展开节点 id；返回本帧点击的叶子或节点 id。
    pub fn tree(
        &mut self,
        salt: impl std::hash::Hash,
        nodes: &[TreeNode],
        expanded: &mut std::collections::HashSet<u64>,
    ) -> Option<u64> {
        let root = self.id_from(("tree", salt));
        let mut clicked = None;
        for node in nodes {
            if let Some(id) = self.tree_node(root.raw(), node, expanded, 0) {
                clicked = Some(id);
            }
        }
        clicked
    }

    fn tree_node(
        &mut self,
        parent: u64,
        node: &TreeNode,
        expanded: &mut std::collections::HashSet<u64>,
        depth: u32,
    ) -> Option<u64> {
        let height = self.theme.metrics.row_height;
        let rect = self.allocate(height, None);
        let id = self.id_from(("tree_node", parent, node.id));
        self.state.register_focusable(id, rect);
        let mut response = self.interact(id, rect, true);
        if self.keyboard_activate(id) {
            response.clicked = true;
        }
        let has_children = !node.children.is_empty();
        let open = expanded.contains(&node.id);
        if response.clicked && has_children {
            if open {
                expanded.remove(&node.id);
            } else {
                expanded.insert(node.id);
            }
        }
        let fill = if response.hovered || response.focused {
            self.theme.colors.primary
        } else {
            self.theme.colors.panel
        };
        self.draw.fill_rect(rect, fill);
        let indent = 12.0 * depth as f32;
        let mark = if !has_children {
            " "
        } else if open {
            "▼"
        } else {
            "▶"
        };
        let size = self.theme.typography.label;
        self.draw.text(
            rect.x + 6.0 + indent,
            rect.y + (rect.h - size) * 0.5,
            size,
            self.theme.colors.text,
            &format!("{mark} {}", node.label),
        );
        let mut node_access = AccessNode::new(id, Role::ListItem)
            .label(node.label.clone())
            .rect(rect)
            .focusable(true)
            .focused(response.focused);
        if has_children {
            node_access = node_access.expanded(open);
        }
        self.access(node_access);
        let mut clicked = if response.clicked {
            Some(node.id)
        } else {
            None
        };
        if has_children && open {
            for child in &node.children {
                if let Some(id) = self.tree_node(node.id, child, expanded, depth + 1) {
                    clicked = Some(id);
                }
            }
        }
        clicked
    }

    /// 按键提示：显示 `Key` 标签与说明文字。
    pub fn key_prompt(&mut self, key: Key, label: impl AsRef<str>) -> Response {
        let label = label.as_ref();
        let height = self.theme.metrics.row_height;
        let rect = self.allocate(height, None);
        let id = self.id_from(("key_prompt", format!("{key:?}"), label));
        let key_w = 56.0;
        let key_rect = Rect::new(
            rect.x,
            rect.y + (rect.h - height.min(28.0)) * 0.5,
            key_w,
            height.min(28.0),
        );
        self.draw.fill_rect(key_rect, self.theme.colors.panel_title);
        self.draw.fill_rect(
            Rect::new(key_rect.x - 1.0, key_rect.y - 1.0, key_rect.w + 2.0, key_rect.h + 2.0),
            self.theme.colors.border,
        );
        let size = self.theme.typography.label;
        let key_text = format!("{key:?}");
        let tw = self.measure_width(&key_text, size);
        self.draw.text(
            key_rect.x + (key_rect.w - tw) * 0.5,
            key_rect.y + (key_rect.h - size) * 0.5,
            size,
            self.theme.colors.text,
            &key_text,
        );
        self.draw.text(
            key_rect.x + key_rect.w + 10.0,
            rect.y + (rect.h - size) * 0.5,
            size,
            self.theme.colors.text,
            label,
        );
        self.access(
            AccessNode::new(id, Role::Label)
                .label(format!("{key_text} {label}"))
                .rect(rect),
        );
        Response::empty(id, rect)
    }

    /// 生命/资源条：`value` 相对 `max`，可自定义填充色。
    pub fn health_bar(
        &mut self,
        value: f32,
        max: f32,
        fill: Color,
    ) -> Response {
        let height = self.theme.metrics.slider_height.max(12.0);
        let rect = self.allocate(height, None);
        let id = self.id_from("health_bar");
        let max = max.max(1e-6);
        let t = (value / max).clamp(0.0, 1.0);
        self.draw.fill_rect(rect, self.theme.colors.track);
        if t > 0.0 {
            self.draw
                .fill_rect(Rect::new(rect.x, rect.y, rect.w * t, rect.h), fill);
        }
        let size = (self.theme.typography.label * 0.85).max(10.0);
        let text = format!("{:.0}/{:.0}", value.max(0.0), max);
        let tw = self.measure_width(&text, size);
        self.draw.text(
            rect.x + (rect.w - tw) * 0.5,
            rect.y + (rect.h - size) * 0.5,
            size,
            self.theme.colors.text,
            &text,
        );
        self.access(
            AccessNode::new(id, Role::ProgressBar)
                .label("health")
                .value(text)
                .rect(rect),
        );
        Response::empty(id, rect)
    }

    /// 小地图视口：在固定尺寸框内绘制世界范围与相机框。
    /// `world` / `camera` 使用同一世界坐标系；点击返回相对世界坐标（可选导航）。
    pub fn minimap_viewport(
        &mut self,
        salt: impl std::hash::Hash,
        size: Vec2,
        world: Rect,
        camera: Rect,
    ) -> Response {
        let size = Vec2::new(size.x.max(32.0), size.y.max(32.0));
        let rect = self.allocate(size.y, Some(size.x));
        let id = self.id_from(("minimap", salt));
        self.state.register_focusable(id, rect);
        let mut response = self.interact(id, rect, true);
        if self.keyboard_activate(id) {
            response.clicked = true;
        }

        self.draw.fill_rect(rect, Color::rgb(0.06, 0.09, 0.14));
        self.draw.fill_rect(
            Rect::new(rect.x - 1.0, rect.y - 1.0, rect.w + 2.0, rect.h + 2.0),
            self.theme.colors.border,
        );

        let ww = world.w.max(1.0);
        let wh = world.h.max(1.0);
        let sx = rect.w / ww;
        let sy = rect.h / wh;
        let map_cam = Rect::new(
            rect.x + (camera.x - world.x) * sx,
            rect.y + (camera.y - world.y) * sy,
            (camera.w * sx).max(2.0),
            (camera.h * sy).max(2.0),
        );
        self.draw.fill_rect(
            map_cam,
            Color::rgba(
                self.theme.colors.focus_ring.r,
                self.theme.colors.focus_ring.g,
                self.theme.colors.focus_ring.b,
                0.35,
            ),
        );
        self.draw.fill_rect(
            Rect::new(map_cam.x, map_cam.y, map_cam.w, 1.0),
            self.theme.colors.focus_ring,
        );
        self.draw.fill_rect(
            Rect::new(map_cam.x, map_cam.y + map_cam.h - 1.0, map_cam.w, 1.0),
            self.theme.colors.focus_ring,
        );
        self.draw.fill_rect(
            Rect::new(map_cam.x, map_cam.y, 1.0, map_cam.h),
            self.theme.colors.focus_ring,
        );
        self.draw.fill_rect(
            Rect::new(map_cam.x + map_cam.w - 1.0, map_cam.y, 1.0, map_cam.h),
            self.theme.colors.focus_ring,
        );

        self.access(
            AccessNode::new(id, Role::Image)
                .label("minimap")
                .rect(rect)
                .focusable(true)
                .focused(response.focused),
        );
        response
    }
}
