//! 纯 Rust 乒乓示例：[`SparkRuntime`] + `run_runtime`。
//!
//! 遗留 [`PingPongGame`]（[`GameHost`]）在 `game` 模块，仅供对照，入口已迁移。

#![forbid(missing_docs)]
mod ball;
mod game;
mod paddle;
pub mod runtime;

pub use ball::Ball;
pub use game::PingPongGame;
pub use paddle::{Paddle, PaddleSide};
