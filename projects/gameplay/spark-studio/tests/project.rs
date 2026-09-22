//! 自 `src/project.rs` 迁出的原 `#[cfg(test)] mod tests`。
use std::path::PathBuf;

use spark_studio::project::{ProjectKind, load_project};

#[test]
fn detects_three_example_kinds() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../examples");
    let ping = load_project(&root.join("ping-pong")).unwrap();
    assert_eq!(ping.kind, ProjectKind::Rust);
    assert!(!ping.kind_inferred);
    let snake = load_project(&root.join("snake")).unwrap();
    assert_eq!(snake.kind, ProjectKind::Valkyrie);
    let tet = load_project(&root.join("tetris")).unwrap();
    assert_eq!(tet.kind, ProjectKind::Hybrid);
}
