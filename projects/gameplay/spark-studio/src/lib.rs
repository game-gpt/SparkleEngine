//! Spark Studio 库面（供 `tests/` 与 bin 共用）。
//!
//! 提供 Unity-like 编辑器壳、项目识别与进程内 Play 会话；
//! 二进制入口见同 crate 的 `main`。

#![forbid(missing_docs)]

pub mod app;
pub mod play;
pub mod project;
pub mod shell;
pub mod state;
