//! 捕获 / 目标 / 冒泡路径。

use crate::id::WidgetId;
use crate::response::EventResponse;
use crate::tree::WidgetTree;

/// 从 `target` 到根的祖先链（含自身，近→远）。
pub fn bubble_path(tree: &WidgetTree, target: WidgetId) -> Vec<WidgetId> {
    let mut path = Vec::new();
    let mut cur = Some(target);
    while let Some(id) = cur {
        path.push(id);
        cur = tree.node(id).and_then(|n| n.parent);
    }
    path
}

/// 根到目标的捕获路径（远→近，不含目标时可与冒泡拼接）。
pub fn capture_path(tree: &WidgetTree, target: WidgetId) -> Vec<WidgetId> {
    let mut path = bubble_path(tree, target);
    path.reverse();
    path.pop(); // 去掉目标，留给 target 阶段
    path
}

/// 在冒泡阶段沿祖先调用 `visit`，直到 `stop_propagation`。
pub fn bubble_from<F>(tree: &WidgetTree, target: WidgetId, mut visit: F) -> EventResponse
where
    F: FnMut(WidgetId) -> EventResponse,
{
    let mut out = EventResponse::default();
    for id in bubble_path(tree, target) {
        let resp = visit(id);
        out.merge(resp);
        if out.stop_propagation {
            break;
        }
    }
    out
}

/// 完整三阶段：capture → target → bubble。
pub fn propagate<F>(tree: &WidgetTree, target: WidgetId, mut visit: F) -> EventResponse
where
    F: FnMut(Phase, WidgetId) -> EventResponse,
{
    let mut out = EventResponse::default();
    for id in capture_path(tree, target) {
        let resp = visit(Phase::Capture, id);
        out.merge(resp);
        if out.stop_propagation {
            return out;
        }
    }
    let resp = visit(Phase::Target, target);
    out.merge(resp);
    if out.stop_propagation {
        return out;
    }
    if let Some(parent) = tree.node(target).and_then(|n| n.parent) {
        for id in bubble_path(tree, parent) {
            let resp = visit(Phase::Bubble, id);
            out.merge(resp);
            if out.stop_propagation {
                break;
            }
        }
    }
    out
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Phase {
    Capture,
    Target,
    Bubble,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::widgets::{button_widget, column};

    #[test]
    fn bubble_path_lists_ancestors() {
        let mut tree = WidgetTree::new();
        let root = tree.root();
        let col = column()
            .child(button_widget().text("x"))
            .mount(&mut tree, root)
            .unwrap();
        let btn = tree.node(col).unwrap().children[0];
        let path = bubble_path(&tree, btn);
        assert_eq!(path[0], btn);
        assert!(path.contains(&col));
        assert!(path.contains(&root));
    }

    #[test]
    fn stop_halts_further_ancestors() {
        let mut tree = WidgetTree::new();
        let root = tree.root();
        let col = column()
            .child(button_widget().text("x"))
            .mount(&mut tree, root)
            .unwrap();
        let btn = tree.node(col).unwrap().children[0];
        let mut seen = Vec::new();
        let resp = bubble_from(&tree, btn, |id| {
            seen.push(id);
            if id == btn {
                EventResponse::stop()
            } else {
                EventResponse::default()
            }
        });
        assert!(resp.stop_propagation);
        assert_eq!(seen, vec![btn]);
    }
}
