# spark-studio

Spark editor: library surface + binary, shell built with `spark-widget`; can open sample projects and Play.

```bash
cargo run -p spark-studio
# or pnpm exec spark studio
```

Project discovery:

```rust
use spark_studio::project::{load_project, ProjectKind};
use std::path::PathBuf;

let root = PathBuf::from("projects/examples");
assert_eq!(load_project(&root.join("ping-pong")).unwrap().kind, ProjectKind::Rust);
assert_eq!(load_project(&root.join("snake")).unwrap().kind, ProjectKind::Valkyrie);
assert_eq!(load_project(&root.join("tetris")).unwrap().kind, ProjectKind::Hybrid);
```

Library types: `StudioApp`, `EditorState`, `PlaySession`, etc. Errors: `ProjectError`.

```bash
cargo test -p spark-studio
```
