//! 混合俄罗斯方块入口。

use spark_engine::run_runtime;
use spark_renderer::WindowConfig;
use tetris::build_runtime;

fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")))
        .init();

    let runtime = build_runtime();
    if let Err(err) = run_runtime(
        WindowConfig { title: "tetris".into(), width: 480, height: 720, clear_color: [0.04, 0.05, 0.08, 1.0] },
        runtime,
    ) {
        eprintln!("tetris: {err}");
        std::process::exit(1);
    }
}
