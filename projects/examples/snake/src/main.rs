//! 贪吃蛇试玩入口（Valkyrie 项目在 VM Play 接通前的 native 宿主）。

use snake::runtime::build_runtime;
use spark_engine::run_runtime;
use spark_renderer::WindowConfig;

fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")))
        .init();

    let runtime = build_runtime();
    if let Err(err) = run_runtime(
        WindowConfig { title: "snake".into(), width: 720, height: 600, clear_color: [0.06, 0.08, 0.07, 1.0] },
        runtime,
    ) {
        eprintln!("snake: {err}");
        std::process::exit(1);
    }
}
