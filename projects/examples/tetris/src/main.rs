//! 混合示例入口：Rust native 注册（Play 接通前可编译）。

mod board;
mod collision;
mod pieces;

use spark_engine::run_game;
use spark_renderer::WindowConfig;

use crate::board::Board;

#[derive(Debug, Default)]
struct TetrisApp {
    board: Board,
    exit: bool,
}

impl spark_renderer::GameHost for TetrisApp {
    fn update(&mut self, _frame: &spark_renderer::FrameCtx<'_>) {
        let _ = &mut self.board;
    }

    fn draw(&mut self, _draw: &mut spark_renderer::DrawList) {}

    fn should_exit(&self) -> bool {
        self.exit
    }
}

fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .init();

    println!("tetris: 混合示例。Rust 棋盘 + Valkyrie HUD/流程，由 Studio Play 接通。");
    if let Err(err) = run_game(
        WindowConfig {
            title: "tetris".into(),
            width: 480,
            height: 720,
            clear_color: [0.04, 0.05, 0.08, 1.0],
        },
        TetrisApp::default(),
    ) {
        eprintln!("tetris: {err}");
        std::process::exit(1);
    }
}
