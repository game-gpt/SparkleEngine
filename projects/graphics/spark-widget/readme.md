# spark-widget

Retained Widget 树：布局、事件、主题，经 `paint_tree_into` 写入 `UiRenderBatch`，由宿主 `draw_ui` / GPU 后端直读。

```rust
use spark_types::Vec2;
use spark_renderer::UiRenderBatch;
use spark_widget::{
    Theme, WidgetTree, paint_tree_into, run_layout,
    asset::NullTextureResolver,
    motion::MotionManager,
    text::EstimateMeasurer,
    widgets::{button_widget, column, label_widget},
    UiMetrics,
};

let mut tree = WidgetTree::new();
let root = tree.root();
column()
    .child(label_widget().text("Hello"))
    .child(button_widget().text("OK"))
    .mount(&mut tree, root)
    .unwrap();

run_layout(&mut tree, Vec2::new(320.0, 240.0), UiMetrics::new(1.0), &mut EstimateMeasurer);

let mut batch = UiRenderBatch::new();
paint_tree_into(
    &tree,
    &Theme::default(),
    &MotionManager::new(),
    &mut NullTextureResolver,
    &mut batch,
);
// 宿主循环：GameHost::draw_ui 写入批次，与世界 DrawList 分开提交。
```

也可用 `tree.mount(parent, WidgetKind::…)` 直接挂节点。事件、焦点、滚动见各 `tests/*.rs`。入口还有 `UiRuntime`、`WidgetId`。UI
动效在 `motion`；Studio：`cargo run -p spark-studio`。

兼容桥 `paint_tree`（刷入 `DrawList` HUD）已废弃，请勿在新代码使用。

```bash
cargo test -p spark-widget
```
