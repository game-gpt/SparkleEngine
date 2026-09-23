//! 按项目持久化停靠布局（`.spark/studio-layout.json`）。

use std::path::{Path, PathBuf};

use super::DockLayoutState;

/// 项目内布局文件路径。
pub fn layout_path(project_root: &Path) -> PathBuf {
    project_root.join(".spark").join("studio-layout.json")
}

/// 读取已保存的停靠布局；缺失或损坏时返回 `None`。
pub fn load_dock_layout(project_root: &Path) -> Option<DockLayoutState> {
    let path = layout_path(project_root);
    let text = std::fs::read_to_string(&path).ok()?;
    serde_json::from_str(&text).ok()
}

/// 将停靠布局写入项目 `.spark/` 目录。
pub fn save_dock_layout(project_root: &Path, dock: &DockLayoutState) -> Result<(), String> {
    let dir = project_root.join(".spark");
    std::fs::create_dir_all(&dir).map_err(|e| format!("创建 .spark 目录失败：{e}"))?;
    let json = serde_json::to_string_pretty(dock).map_err(|e| format!("序列化布局失败：{e}"))?;
    std::fs::write(layout_path(project_root), json).map_err(|e| format!("写入布局失败：{e}"))?;
    Ok(())
}
