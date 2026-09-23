//! Studio Play：进程内嵌入示例 [`SparkRuntime`]（Esc / Stop 回编辑器）。

use ping_pong::build_runtime as ping_pong_runtime;
use snake::build_runtime as snake_runtime;
use spark_engine::RuntimeHost2d;
use spark_renderer::{DrawList, FrameCtx, WindowPump2d};
use tetris::build_runtime as tetris_runtime;

use crate::project::{ProjectInfo, ProjectKind};

/// 进程内 Play 会话：持有示例运行时的窗口泵适配器。
pub enum PlaySession {
    /// 乒乓示例。
    PingPong(RuntimeHost2d),
    /// 俄罗斯方块示例。
    Tetris(RuntimeHost2d),
    /// 贪吃蛇示例。
    Snake(RuntimeHost2d),
}

impl PlaySession {
    /// 按项目 `runTarget` / 名称 / kind 解析并启动会话。
    pub fn start(project: &ProjectInfo) -> Result<Self, String> {
        if let Some(session) = resolve_by_target(project) {
            return Ok(session);
        }
        if let Some(session) = resolve_by_name(&project.name) {
            return Ok(session);
        }
        match project.kind {
            ProjectKind::Valkyrie => Ok(Self::Snake(snake_runtime().into_host())),
            ProjectKind::Hybrid => Ok(Self::Tetris(tetris_runtime().into_host())),
            ProjectKind::Rust => Ok(Self::PingPong(ping_pong_runtime().into_host())),
        }
    }

    /// 推进仿真相位。
    pub fn simulate(&mut self, frame: &FrameCtx<'_>) {
        match self {
            Self::PingPong(h) => h.simulate(frame),
            Self::Tetris(h) => h.simulate(frame),
            Self::Snake(h) => h.simulate(frame),
        }
    }

    /// 绘制对局世界层。
    pub fn present_world(&mut self, draw: &mut DrawList) {
        match self {
            Self::PingPong(h) => h.present_world(draw),
            Self::Tetris(h) => h.present_world(draw),
            Self::Snake(h) => h.present_world(draw),
        }
    }

    /// 会话是否请求结束（回编辑器或退出浸入模式）。
    pub fn should_exit(&self) -> bool {
        match self {
            Self::PingPong(h) => h.should_exit(),
            Self::Tetris(h) => h.should_exit(),
            Self::Snake(h) => h.should_exit(),
        }
    }

    /// 状态栏 / 日志用短标签。
    pub fn label(&self) -> &'static str {
        match self {
            Self::PingPong(_) => "ping-pong",
            Self::Tetris(_) => "tetris",
            Self::Snake(_) => "snake",
        }
    }
}

fn resolve_by_target(project: &ProjectInfo) -> Option<PlaySession> {
    let raw = project.run_target.as_deref()?;
    resolve_token(raw)
}

fn resolve_by_name(name: &str) -> Option<PlaySession> {
    resolve_token(name)
}

fn resolve_token(raw: &str) -> Option<PlaySession> {
    let key = raw.trim().to_ascii_lowercase().replace('_', "-");
    if key.contains("ping") || key == "ping-pong" || key == "pingpong" {
        return Some(PlaySession::PingPong(ping_pong_runtime().into_host()));
    }
    if key.contains("tetris") {
        return Some(PlaySession::Tetris(tetris_runtime().into_host()));
    }
    if key.contains("snake") {
        return Some(PlaySession::Snake(snake_runtime().into_host()));
    }
    None
}
