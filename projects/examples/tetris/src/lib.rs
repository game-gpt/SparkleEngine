//! 混合示例：Rust 棋盘权威 + [`SparkRuntime`] 主循环（Valkyrie HUD 元数据在 assets/scripts）。

#![forbid(missing_docs)]
mod board;
mod collision;
mod pieces;
pub mod runtime;

pub use board::Board;
pub use pieces::PieceKind;
pub use runtime::build_runtime;
