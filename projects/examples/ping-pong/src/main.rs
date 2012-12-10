//! 纯 Rust 乒乓入口（Play 接通前仅保证可编译）。

mod ball;
mod game;
mod paddle;

use spark_engine::run_game;
use spark_renderer::WindowConfig;

use crate::game::PingPongApp;

fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .init();

    println!("ping-pong: 纯 Rust 示例。完整玩法由 Studio Play / cargo run 接通。");
    if let Err(err) = run_game(
        WindowConfig {
            title: "ping-pong".into(),
            width: 960,
            height: 540,
            clear_color: [0.05, 0.07, 0.10, 1.0],
        },
        PingPongApp::default(),
    ) {
        eprintln!("ping-pong: {err}");
        std::process::exit(1);
    }
}
