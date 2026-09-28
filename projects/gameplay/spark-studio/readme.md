# spark-studio

Spark 编辑器：库面 + 二进制，用 `spark-widget` 搭壳，可打开示例工程并 Play。

```bash
cargo run -p spark-studio
# 或 pnpm exec spark studio
```

工程发现：

```rust
use spark_studio::project::{load_project, ProjectKind};
use std::path::PathBuf;

let root = PathBuf::from("projects/examples");
assert_eq!(load_project(&root.join("ping-pong")).unwrap().kind, ProjectKind::Rust);
assert_eq!(load_project(&root.join("snake")).unwrap().kind, ProjectKind::Valkyrie);
assert_eq!(load_project(&root.join("tetris")).unwrap().kind, ProjectKind::Hybrid);
```

库面类型：`StudioApp`、`EditorState`、`PlaySession` 等。错误：`ProjectError`。

```bash
cargo test -p spark-studio
```
