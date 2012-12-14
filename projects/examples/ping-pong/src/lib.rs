//! 纯 Rust 乒乓示例：可独立 `cargo run`，也可由 Studio Play 嵌入。

mod ball;
mod game;
mod paddle;

pub use ball::Ball;
pub use game::PingPongGame;
pub use paddle::{Paddle, PaddleSide};
