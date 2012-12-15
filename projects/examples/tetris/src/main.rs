//! 混合俄罗斯方块入口。

use spark_engine::run_game;
use spark_renderer::WindowConfig;
use tetris::TetrisApp;

fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .init();

    if let Err(err) = run_game(
        WindowConfig {
            title: "tetris".into(),
            width: 480,
            height: 720,
            clear_color: [0.04, 0.05, 0.08, 1.0],
        },
        TetrisApp::new(),
    ) {
        eprintln!("tetris: {err}");
        std::process::exit(1);
    }
}
