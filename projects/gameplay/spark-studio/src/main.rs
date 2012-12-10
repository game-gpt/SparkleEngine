//! Spark Studio 入口二进制。

mod app;
mod shell;

use spark_engine::run_game;
use spark_renderer::WindowConfig;

use crate::app::StudioApp;

fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .init();

    let project = std::env::args()
        .skip(1)
        .find(|a| !a.starts_with('-'))
        .unwrap_or_else(|| ".".into());

    tracing::info!(%project, "启动 Spark Studio");
    println!("spark-studio: 正在打开 Spark Studio …");
    println!("spark-studio: 项目路径 = {project}");

    let host = StudioApp::new(project);
    if let Err(err) = run_game(
        WindowConfig {
            title: "Spark Studio".into(),
            width: 1440,
            height: 900,
            clear_color: [0.08, 0.09, 0.11, 1.0],
        },
        host,
    ) {
        tracing::error!(?err, "退出异常");
        eprintln!("spark-studio: {err}");
        std::process::exit(1);
    }
}
