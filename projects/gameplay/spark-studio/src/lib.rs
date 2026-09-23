//! Spark Studio 库面（供 `tests/` 与 bin 共用）。
//!
//! 编辑器壳在 `ui/`，停靠布局在 `layout/`，会话状态在 `state/`。

#![forbid(missing_docs)]

pub mod app;
pub mod layout;
pub mod play;
pub mod project;
pub mod state;
pub mod ui;
