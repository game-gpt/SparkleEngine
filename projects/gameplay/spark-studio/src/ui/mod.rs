//! 编辑器 Widget 壳：顶栏、面板与编排入口。

mod asset_browser;
mod bottom_bar;
mod console;
mod dock_panel;
mod hierarchy;
mod inspector;
mod problems;
mod scene_canvas;
mod scene_toolbar;
mod shell;
mod splitter;
mod status_bar;
mod style;
mod top_bar;
mod viewport;

pub use shell::build_shell;
