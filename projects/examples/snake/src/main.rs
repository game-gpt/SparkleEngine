//! 贪吃蛇试玩入口（Valkyrie 项目在 VM Play 接通前的 native 宿主）。

use snake::SnakeApp;
use spark_engine::run_game;
use spark_renderer::WindowConfig;

fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .init();

    if let Err(err) = run_game(
        WindowConfig {
            title: "snake".into(),
            width: 720,
            height: 600,
            clear_color: [0.06, 0.08, 0.07, 1.0],
        },
        SnakeApp::new(),
    ) {
        eprintln!("snake: {err}");
        std::process::exit(1);
    }
}
