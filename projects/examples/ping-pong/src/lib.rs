//! 纯 Rust 乒乓示例：[`SparkRuntime`] + `run_runtime`。

#![forbid(missing_docs)]
mod ball;
mod paddle;
pub mod runtime;

pub use ball::Ball;
pub use paddle::{Paddle, PaddleSide};
pub use runtime::build_runtime;
