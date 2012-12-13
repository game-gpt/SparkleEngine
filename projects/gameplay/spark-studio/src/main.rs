//! Spark Studio 入口：打开当前 npm 游戏项目的 Unity-like 编辑器。

mod app;
mod project;
mod shell;
mod state;

use spark_engine::run_game;
use spark_renderer::WindowConfig;

use crate::app::StudioApp;
use crate::project::{load_project, resolve_project_dir};

fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .init();

    let args: Vec<String> = std::env::args().skip(1).collect();
    let root = resolve_project_dir(&args);
    let project = match load_project(&root) {
        Ok(p) => p,
        Err(e) => {
            eprintln!("spark-studio: {e}");
            std::process::exit(2);
        }
    };

    if !project.has_sparkle_engine_dep {
        eprintln!(
            "spark-studio: 警告：package.json 未声明依赖 @game-gpt/sparkle-engine（继续打开）"
        );
    }

    tracing::info!(
        name = %project.name,
        kind = project.kind.as_str(),
        inferred = project.kind_inferred,
        root = %project.root.display(),
        scene = ?project.startup_scene,
        "启动 Spark Studio"
    );
    println!(
        "spark-studio: 打开项目 {} [{}] ({})",
        project.name,
        project.kind.as_str(),
        project.root.display()
    );
    if let Some(scene) = &project.startup_scene {
        println!("spark-studio: startupScene = {scene}");
    }

    let title = format!("Spark Studio — {}", project.name);
    let host = StudioApp::new(project);
    if let Err(err) = run_game(
        WindowConfig {
            title,
            width: 1440,
            height: 900,
            clear_color: [0.10, 0.11, 0.12, 1.0],
        },
        host,
    ) {
        tracing::error!(?err, "退出异常");
        eprintln!("spark-studio: {err}");
        std::process::exit(1);
    }
}
