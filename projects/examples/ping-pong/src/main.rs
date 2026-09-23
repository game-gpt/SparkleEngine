//! 纯 Rust 乒乓入口。

use ping_pong::runtime::build_runtime;
use spark_engine::run_runtime;
use spark_renderer::WindowConfig;

fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")))
        .init();

    let runtime = build_runtime();
    if let Err(err) = run_runtime(
        WindowConfig { title: "ping-pong".into(), width: 960, height: 540, clear_color: [0.05, 0.07, 0.10, 1.0] },
        runtime,
    ) {
        eprintln!("ping-pong: {err}");
        std::process::exit(1);
    }
}
