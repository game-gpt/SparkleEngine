# spark-widget

Retained Widget 树：布局、事件、主题，经 `paint_tree` 写入 `DrawList`。

```rust
use spark_types::{Color, Vec2};
use spark_renderer::DrawList;
use spark_widget::{
    Theme, WidgetTree, paint_tree, run_layout,
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

let mut draw = DrawList::new(Color::rgb(0.0, 0.0, 0.0));
paint_tree(
    &tree,
    &Theme::default(),
    &MotionManager::new(),
    &mut NullTextureResolver,
    &mut draw,
);
```

也可用 `tree.mount(parent, WidgetKind::…)` 直接挂节点。事件、焦点、滚动见各 `tests/*.rs`。入口还有 `UiRuntime`、`WidgetId`。UI
动效在 `motion`；Studio：`cargo run -p spark-studio`。

```bash
cargo test -p spark-widget
```
