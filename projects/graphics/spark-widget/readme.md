# spark-widget

Retained widget tree: layout, events, and theming; `paint_tree_into` writes into `UiRenderBatch`, consumed by the host `draw_ui` / GPU backend.

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
// Host loop: GameHost::draw_ui fills the batch, submitted separately from the world DrawList.
```

You can also `tree.mount(parent, WidgetKind::…)` directly. Events, focus, and scrolling are covered in `tests/*.rs`. Entry points include `UiRuntime` and `WidgetId`. UI motion lives in `motion`; Studio: `cargo run -p spark-studio`.

The compatibility bridge `paint_tree` (flushing into `DrawList` HUD) is deprecated; do not use it in new code.

```bash
cargo test -p spark-widget
```
