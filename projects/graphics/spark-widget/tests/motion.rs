//! 自 `src/motion/mod.rs` 迁出的原 `#[cfg(test)] mod tests`。
use spark_widget::*;

use spark_widget::{
    node::{WidgetKind, WidgetStateFlags},
    tree::WidgetTree,
};

#[test]
fn hover_raises_scale_over_time() {
    let mut tree = WidgetTree::new();
    let id = tree.mount(tree.root(), WidgetKind::Button).unwrap();
    if let Some(node) = tree.node_mut(id) {
        node.state = WidgetStateFlags { hovered: true, visible: true, ..WidgetStateFlags::default() };
    }
    let mut motion = MotionManager::new();
    motion.sync_and_tick(&tree, 0.0);
    let before = motion.sample(id).scale;
    motion.sync_and_tick(&tree, 0.05);
    let mid = motion.sample(id).scale;
    assert!(mid > before);
    for _ in 0..20 {
        motion.sync_and_tick(&tree, 0.05);
    }
    assert!((motion.sample(id).scale - 1.03).abs() < 0.01);
}
