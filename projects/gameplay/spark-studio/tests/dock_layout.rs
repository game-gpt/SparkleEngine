//! 停靠布局序列化与持久化路径。

use spark_studio::layout::{DockLayoutState, layout_path, save_dock_layout};

#[test]
fn dock_layout_json_roundtrip() {
    let dock = DockLayoutState {
        hierarchy_width: 260.0,
        inspector_width: 280.0,
        bottom_height: 200.0,
        hierarchy_collapsed: false,
        inspector_collapsed: true,
        bottom_collapsed: false,
    };
    let json = serde_json::to_string(&dock).unwrap();
    let parsed: DockLayoutState = serde_json::from_str(&json).unwrap();
    assert_eq!(parsed, dock);
}

#[test]
fn saves_layout_under_dot_spark() {
    let dir = std::env::temp_dir().join(format!("spark-studio-layout-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let dock = DockLayoutState::default();
    save_dock_layout(&dir, &dock).unwrap();
    assert!(layout_path(&dir).is_file());
    let _ = std::fs::remove_dir_all(&dir);
}
